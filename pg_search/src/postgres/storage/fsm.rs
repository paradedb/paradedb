// Copyright (c) 2023-2025 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::{bm25_max_free_space, BM25PageSpecialData};
use crate::postgres::storage::buffer::{init_new_buffer, Buffer, BufferManager};
use crate::postgres::storage::metadata::MetaPage;
use pgrx::iter::TableIterator;
use pgrx::{name, pg_extern, pg_sys, AnyNumeric, PgRelation};

/// Denotes what the data on an FSM block looks like
#[allow(non_camel_case_types)]
#[derive(Default, Debug, Copy, Clone)]
#[repr(u32)]
enum FSMBlockKind {
    /// This variant represents the original FSM format in pg_search versions 0.17.0 through 0.17.3
    /// It is not meant to be used for making new pages, only for detecting old pages so they can
    /// be converted
    #[doc(hidden)]
    #[allow(dead_code)]
    v0 = 0,

    /// This represents the current FSM format and is the default for new FSM pages
    #[default]
    v1_uncompressed = 1,
}

/// A short header for the FSM block, stored at the beginning of each page, which allows us to quickly
/// identify what kind of block we're about to work with
#[derive(Default, Debug, Copy, Clone)]
#[repr(C)]
struct FSMBlockHeader {
    /// Denotes how the block data is stored on this page
    kind: FSMBlockKind,
}

/// The header information for the current FSM block format.  Its first field is purposely the [`FSMBlockHeader`]
/// so that the block header can be read as that type.  `#[repr(C)]` ensures this is correct
#[derive(Default, Debug, Copy, Clone)]
#[repr(C)]
struct V1Header {
    header: FSMBlockHeader,

    /// Denotes if this block is completely empty, meaning all its entries reference the
    /// [`pg_sys::InvalidBlockNumber`].
    ///
    /// Empty blocks can be skipped without needing to read the entire entry data.  This is just a
    /// hint towards `true`.  If it's `false` but all the entries are invalid, that's okay --
    /// we only wasted some CPU cycles scanning the empty block.
    empty: bool,
}

/// An individual entry on a [`FSMBlock`].  Represented as a pair of [`pg_sys::BlockNumber`] and
/// [`pg_sys::TransactionId`].  The transaction id is the [`pg_sys::GetCurrentTransactionId()`] (or perhaps caller-provided)
/// of the transaction that added the entry to the FSM.
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct FSMEntry(pg_sys::BlockNumber, pg_sys::TransactionId);

const MAX_ENTRIES_PER_PAGE: usize =
    (bm25_max_free_space() - size_of::<V1Header>()) / size_of::<FSMEntry>();

#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct FSMBlock {
    header: V1Header,
    entries: [FSMEntry; MAX_ENTRIES_PER_PAGE],
}

impl Default for FSMBlock {
    fn default() -> Self {
        Self {
            header: Default::default(),
            entries: [FSMEntry(pg_sys::InvalidBlockNumber, pg_sys::InvalidTransactionId);
                MAX_ENTRIES_PER_PAGE],
        }
    }
}

impl FSMBlock {
    #[inline]
    fn all_invalid(&self) -> bool {
        self.entries
            .iter()
            .all(|FSMEntry(blockno, _)| *blockno == pg_sys::InvalidBlockNumber)
    }

    #[inline]
    fn any_invalid(&self) -> bool {
        self.entries
            .iter()
            .any(|FSMEntry(blockno, _)| *blockno == pg_sys::InvalidBlockNumber)
    }

    #[inline]
    fn any_valid(&self) -> bool {
        self.entries
            .iter()
            .any(|FSMEntry(blockno, _)| *blockno != pg_sys::InvalidBlockNumber)
    }
}

/// The [`FreeSpaceManager`] is our version of Postgres' "free space map".  We need to track free space
/// as whole blocks and we'd prefer to not have to mark pages as deleted when giving them to the FSM.
///
/// We also have a requirement that blocks be recycled in the future, after the transaction which
/// marked them free is known to no longer be overlapping with other concurrent transactions, including
/// those from hot-standby servers.  Reusing a block before all nodes in the cluster and/or all
/// concurrent backends are aware that it's been deleted can cause race conditions and data corruption.
///
/// The on-disk structure is simply a linked list of blocks where each block, a [`FSMBlock`],
/// is a fixed-sized array of ([`pg_sys::BlockNumber`], [`pg_sys::TransactionId`]) pairs.
///
/// Each block starts with a small [`FSMBlockHeader`] indicating the type of block (we've had a few
/// styles so far).  This is denoted by the [`FSMBlockHeader::kind`] flag.
///
/// Outside per-page exclusive locking when mutating a page, no special locking requirements exist
/// to manage concurrency.  The intent is that the [`FreeSpaceManager`]'s linked list can grow
/// unbounded, with the hope that it actually won't grow to be very large in practice.
///
/// Any other kind of structure will likely need a more sophisticated approach to concurrency control.
///
/// The user-facing API is meant to _kinda_ mimic a `Vec` in that the [`FreeSpaceManager`] can be
/// popped, drained, and extended.
///
/// There is a [`FSMBlockKind::v0`] variant which is used to represent the original FSM format, used
/// prior to pg_search 0.17.4.  This variant is not meant to be used for making new pages, and if
/// found on disk, we will immediately convert it to the new format, with the caveat that any data
/// the `v0` block contains will be lost.  This will effectively orphan blocks that it referenced.
///
/// # V1
///
/// A `v1` block's content layout disk layout is:
///
/// ```text
/// [kind (4 bytes)] [empty (1 byte)] [ [blockno (4 bytes)] [xid (4 bytes)] ][ ... ] (up to MAX_ENTRIES_PER_PAGE)
/// ```
///
/// And blocks are linked together with the `next_blockno` field in the [`BM25PageSpecialData`].
///
#[derive(Debug)]
pub struct FreeSpaceManager {
    start_blockno: pg_sys::BlockNumber,
}

impl FreeSpaceManager {
    /// Create a new [`FreeSpaceManager`] in the block storage of the specified `indexrel`.
    pub unsafe fn create(indexrel: &PgSearchRelation) -> pg_sys::BlockNumber {
        let mut new_buffer = init_new_buffer(indexrel);
        let mut page = new_buffer.page_mut();
        *page.contents_mut::<FSMBlock>() = FSMBlock::default();
        new_buffer.number()
    }

    /// Open an existing [`FreeSpaceManager`] which is rooted at the specified starting block number.
    pub fn open(start_blockno: pg_sys::BlockNumber) -> Self {
        Self { start_blockno }
    }

    /// Retrieve a single recyclable [`pg_sys::BlockNumber`], which can be acquired and re-initialized.
    ///
    /// Returns `None` if no recyclable blocks are available.
    ///
    /// Upon return, the block is removed from the [`FreeSpaceManager`]'s control.  It is the caller's
    /// responsibility to ensure the block is properly used, or else it will be lost forever as
    /// dead space in the underlying relation.
    pub fn pop(&mut self, bman: &mut BufferManager) -> Option<pg_sys::BlockNumber> {
        let xid_horizon =
            unsafe { pg_sys::GetCurrentTransactionIdIfAny().max(pg_sys::FirstNormalTransactionId) };
        let mut blockno = self.start_blockno;
        loop {
            if blockno == pg_sys::InvalidBlockNumber {
                return None;
            }

            let buffer = bman.get_buffer(blockno);
            let page = buffer.page();
            blockno = page.special::<BM25PageSpecialData>().next_blockno;

            let contents = page.contents_ref::<FSMBlock>();
            if contents.any_valid() {
                let mut buffer = buffer.upgrade(bman);
                let mut page = buffer.page_mut();
                let contents = page.contents_mut::<FSMBlock>();

                let mut found_blockno = None;
                for FSMEntry(blockno, fsm_xid) in &mut contents.entries {
                    if *blockno != pg_sys::InvalidBlockNumber
                        && passses_visibility_horizon(*fsm_xid, xid_horizon)
                    {
                        found_blockno = Some(*blockno);
                        *blockno = pg_sys::InvalidBlockNumber;
                        break;
                    }
                }

                if found_blockno.is_none() {
                    buffer.set_dirty(false);
                } else {
                    contents.header.empty = contents.all_invalid();
                    return found_blockno;
                }
            }
        }
    }

    /// Drain `n` recyclable blocks from this [`FreeSpaceManager`] instance, using the specified
    /// [`BufferManager`] for underlying disk access.
    ///
    /// As [`pg_sys::BlockNumber`]s are yielded from the returned iterator, they are removed from the
    /// FSM.  The returned iterator will never return more than `n`, but it could return fewer.
    ///
    /// It is the caller's responsibility to ensure each yielded block is properly used, or else it will
    /// be lost forever as dead space in the underlying relation.  Unyielded blocks are unaffected.
    pub fn drain(
        &mut self,
        bman: &mut BufferManager,
        n: usize,
    ) -> impl Iterator<Item = pg_sys::BlockNumber> {
        let xid_horizon =
            unsafe { pg_sys::GetCurrentTransactionIdIfAny().max(pg_sys::FirstNormalTransactionId) };
        let mut blocks = Vec::with_capacity(n);
        let mut blockno = self.start_blockno;
        loop {
            if blockno == pg_sys::InvalidBlockNumber || blocks.len() == n {
                return blocks.into_iter();
            }

            let buffer = bman.get_buffer(blockno);
            let page = buffer.page();
            blockno = page.special::<BM25PageSpecialData>().next_blockno;

            let contents = page.contents_ref::<FSMBlock>();
            if contents.any_valid() {
                let mut buffer = buffer.upgrade(bman);
                let mut page = buffer.page_mut();
                let contents = page.contents_mut::<FSMBlock>();
                let current_block_count = blocks.len();
                for FSMEntry(blockno, fsm_xid) in &mut contents.entries {
                    if *blockno != pg_sys::InvalidBlockNumber
                        && passses_visibility_horizon(*fsm_xid, xid_horizon)
                    {
                        blocks.push(*blockno);
                        *blockno = pg_sys::InvalidBlockNumber;
                        if blocks.len() == n {
                            break;
                        }
                    }
                }

                if blocks.len() == current_block_count {
                    buffer.set_dirty(false);
                } else {
                    contents.header.empty = contents.all_invalid();
                }
            }
        }
    }

    /// Add the specified `extend_with` iterator of [`pg_sys::BlockNumber`]s to this [`FreeSpaceManager`].
    ///
    /// The added blocks will be recyclable in the future based on the current [`pg_sys::GetCurrentTransactionId`].
    pub fn extend(
        &self,
        bman: &mut BufferManager,
        extend_with: impl Iterator<Item = pg_sys::BlockNumber>,
    ) {
        let current_xid =
            unsafe { pg_sys::GetCurrentTransactionIdIfAny().max(pg_sys::FirstNormalTransactionId) };
        self.extend_with_when_recyclable(bman, current_xid, extend_with);
    }

    /// Add the specified `extend_with` iterator of [`pg_sys::BlockNumber`]s to this [`FreeSpaceManager`].
    ///
    /// The added blocks will be recyclable in the future based on the provided `when_recyclable` transaction id.
    pub fn extend_with_when_recyclable(
        &self,
        bman: &mut BufferManager,
        when_recyclable: pg_sys::TransactionId,
        extend_with: impl Iterator<Item = pg_sys::BlockNumber>,
    ) {
        let mut extend_with = extend_with.peekable();
        let mut blockno = self.start_blockno;
        loop {
            let buffer = bman.get_buffer(blockno);

            let need_v0_upgrade = |buffer: &Buffer| {
                matches!(
                    buffer.page().contents::<FSMBlockHeader>().kind,
                    FSMBlockKind::v0
                )
            };
            let space_available = |buffer: &Buffer| {
                need_v0_upgrade(buffer) || {
                    let page = buffer.page();
                    let block = page.contents_ref::<FSMBlock>();
                    block.header.empty || block.any_invalid()
                }
            };

            let mut buffer = if space_available(&buffer) {
                let mut buffer = buffer.upgrade(bman);

                if need_v0_upgrade(&buffer) {
                    *buffer.page_mut().contents_mut::<FSMBlock>() = FSMBlock::default();
                }

                let mut page = buffer.page_mut();
                let contents = page.contents_mut::<FSMBlock>();
                let mut cnt = 0;
                contents
                    .entries
                    .iter_mut()
                    .filter(|FSMEntry(blockno, _)| *blockno == pg_sys::InvalidBlockNumber)
                    .zip(&mut extend_with)
                    .for_each(|(entry, blockno)| {
                        *entry = FSMEntry(blockno, when_recyclable);
                        cnt += 1;
                    });

                if cnt == 0 {
                    // we didn't make any modifications so the page is not dirty -- no need to WAL log it
                    buffer.set_dirty(false);
                } else {
                    // we added at least one block to this page so it's no longer empty
                    contents.header.empty = false;
                }

                if extend_with.peek().is_none() {
                    // no more blocks to add to the FSM
                    return;
                }

                blockno = buffer.page().special::<BM25PageSpecialData>().next_blockno;
                if blockno != pg_sys::InvalidBlockNumber {
                    // move to next block
                    continue;
                }
                buffer
            } else {
                blockno = buffer.page().special::<BM25PageSpecialData>().next_blockno;
                if blockno != pg_sys::InvalidBlockNumber {
                    // move to next block
                    continue;
                }
                buffer.upgrade(bman)
            };

            // we still have blocks to apply but have no more space on this page
            // so allocate a new page
            let mut new_buffer = init_new_buffer(bman.buffer_access().rel());
            let mut new_page = new_buffer.page_mut();

            // initialize the new page with a default FSMBlock
            *new_page.contents_mut::<FSMBlock>() = FSMBlock::default();

            // move to this new page
            let new_blockno = new_buffer.number();
            buffer
                .page_mut()
                .special_mut::<BM25PageSpecialData>()
                .next_blockno = new_blockno;

            // loop back around to try extending this new page
            blockno = new_blockno;
        }
    }
}

/// The `xid_horizon` argument represents the oldest transaction id, across the Postgres cluster,
/// that can see blocks in the FSM.
///
/// When being drained, the FSM compares each block's stored `fsm_xid`` with this value, ensuring the stored
/// value precedes or equals this `xid_horizon`, before it is considered recyclable.
#[inline(always)]
fn passses_visibility_horizon(
    fsm_xid: pg_sys::TransactionId,
    xid_horizon: pg_sys::TransactionId,
) -> bool {
    crate::postgres::utils::TransactionIdPrecedesOrEquals(fsm_xid, xid_horizon)
}

#[pg_extern]
unsafe fn fsm_info(
    index: PgRelation,
) -> TableIterator<
    'static,
    (
        name!(fsm_blockno, AnyNumeric),
        name!(free_blockno, AnyNumeric),
    ),
> {
    let index = PgSearchRelation::from_pg(index.as_ptr());

    let meta = MetaPage::open(&index);
    let fsm_start = meta.fsm();
    let bman = BufferManager::new(&index);
    let mut mapping = Vec::<(pg_sys::BlockNumber, Vec<FSMEntry>)>::default();

    let mut blockno = fsm_start;

    while blockno != pg_sys::InvalidBlockNumber {
        let buffer = bman.get_buffer(blockno);
        let page = buffer.page();
        let block = page.contents::<FSMBlock>();
        if !block.header.empty {
            let free_blocks = block.entries.to_vec();
            mapping.push((blockno, free_blocks));
        }
        blockno = page.special::<BM25PageSpecialData>().next_blockno;
    }

    TableIterator::new(mapping.into_iter().flat_map(|(fsm_blockno, blocks)| {
        blocks
            .into_iter()
            .filter(|FSMEntry(blockno, _)| *blockno != pg_sys::InvalidBlockNumber)
            .map(move |blockno| (fsm_blockno.into(), blockno.0.into()))
    }))
}
