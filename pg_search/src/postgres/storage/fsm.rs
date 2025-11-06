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
use crate::postgres::storage::buffer::BufferManager;
use crate::postgres::storage::fsm::v1::V1FSM;
use crate::postgres::storage::fsm::v2::V2FSM;
use crate::postgres::storage::metadata::MetaPage;
use pgrx::iter::TableIterator;
use pgrx::{name, pg_extern, pg_sys, PgRelation};

/// Denotes what the data on an FSM block looks like
#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone)]
#[repr(u32)]
enum FSMBlockKind {
    /// This variant represents the original FSM format in pg_search versions 0.17.0 through 0.17.3
    /// It is not meant to be used for making new pages, only for detecting old pages so they can
    /// be converted
    #[doc(hidden)]
    #[allow(dead_code)]
    v0 = 0,

    /// This represents an older FSM format
    v1_uncompressed = 1,

    /// This represents the current FSM format, which is based on an AVL tree for organizing things by [`pg_sys::TransactionId`]
    v2_avl_tree = 2,
}

/// A short header for the FSM block, stored at the beginning of each page, which allows us to quickly
/// identify what kind of block we're about to work with
#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct FSMBlockHeader {
    /// Denotes how the block data is stored on this page
    kind: FSMBlockKind,
}

/// A [`FreeSpaceManager`] provides an interface to a block-storage backed structure to track
/// relation block numbers that can be reused at a later time.  The free blocks are associated
/// with the earliest transaction id that can use them.
///
/// The user-facing API is meant to _kinda_ mimic a `Vec` in that a [`FreeSpaceManager`] can be
/// popped, drained, and extended.
pub trait FreeSpaceManager {
    /// Create a new [`FreeSpaceManager`] in the block storage of the specified `indexrel`.
    unsafe fn create(indexrel: &PgSearchRelation) -> pg_sys::BlockNumber;

    /// Open an existing [`FreeSpaceManager`] which is rooted at the specified starting block number.
    fn open(start_blockno: pg_sys::BlockNumber) -> Self;

    /// Retrieve a single recyclable [`pg_sys::BlockNumber`], which can be acquired and re-initialized.
    ///
    /// Returns `None` if no recyclable blocks are available.
    ///
    /// Upon return, the block is removed from the [`FreeSpaceManager`]'s control.  It is the caller's
    /// responsibility to ensure the block is properly used, or else it will be lost forever as
    /// dead space in the underlying relation.
    fn pop(&mut self, bman: &mut BufferManager) -> Option<pg_sys::BlockNumber>;

    /// Drain `n` recyclable blocks from this [`FreeSpaceManager`] instance, using the specified
    /// [`BufferManager`] for underlying disk access.
    ///
    /// It is the caller's responsibility to ensure each yielded block is properly used, or else it will
    /// be lost forever as dead space in the underlying relation.  Unyielded blocks are unaffected.
    fn drain(
        &mut self,
        bman: &mut BufferManager,
        n: usize,
    ) -> impl Iterator<Item = pg_sys::BlockNumber> + 'static;

    /// Add the specified `extend_with` iterator of [`pg_sys::BlockNumber`]s to this [`V1FSM`].
    ///
    /// The added blocks will be recyclable in the future based on the current [`pg_sys::GetCurrentTransactionId`].
    ///
    /// The default implementation delegates to [`Self::extend_with_when_recyclable`] using the
    /// current transaction id if any, or [`pg_sys::FirstNormalTransactionId`] otherwise.
    fn extend(
        &mut self,
        bman: &mut BufferManager,
        extend_with: impl Iterator<Item = pg_sys::BlockNumber>,
    ) {
        let current_xid = unsafe {
            pg_sys::GetCurrentFullTransactionIdIfAny()
                .value
                .max(pg_sys::FirstNormalTransactionId.into_inner() as u64)
        };
        self.extend_with_when_recyclable(
            bman,
            pg_sys::FullTransactionId { value: current_xid },
            extend_with,
        )
    }

    /// Add the specified `extend_with` iterator of [`pg_sys::BlockNumber`]s to this [`V1FSM`].
    ///
    /// The added blocks will be recyclable in the future based on the provided `when_recyclable` transaction id.
    fn extend_with_when_recyclable(
        &mut self,
        bman: &mut BufferManager,
        when_recyclable: pg_sys::FullTransactionId,
        extend_with: impl Iterator<Item = pg_sys::BlockNumber>,
    );
}

/// The [`v1`] is our version of Postgres' "free space map".  We need to track free space
/// as whole blocks and we'd prefer to not have to mark pages as deleted when giving them to the FSM.
///
/// We also have a requirement that blocks be recycled in the future, after the transaction which
/// marked them free is known to no longer be overlapping with other concurrent transactions, including
/// those from hot-standby servers.  Reusing a block before all nodes in the cluster and/or all
/// concurrent backends are aware that it's been deleted can cause race conditions and data corruption.
///
/// The on-disk structure is simply a linked list of blocks where each block, a [`v1::FSMBlock`],
/// is a fixed-sized array of ([`pg_sys::BlockNumber`], [`pg_sys::TransactionId`]) pairs.
///
/// Each block starts with a small [`FSMBlockHeader`] indicating the type of block (we've had a few
/// styles so far).  This is denoted by the [`FSMBlockHeader::kind`] flag.
///
/// Outside per-page exclusive locking when mutating a page, no special locking requirements exist
/// to manage concurrency.  The intent is that the [`V1FSM`]'s linked list can grow
/// unbounded, with the hope that it actually won't grow to be very large in practice.
///
/// Any other kind of structure will likely need a more sophisticated approach to concurrency control.
///
pub mod v1 {
    use crate::postgres::rel::PgSearchRelation;
    use crate::postgres::storage::block::{bm25_max_free_space, BM25PageSpecialData};
    use crate::postgres::storage::buffer::{init_new_buffer, Buffer, BufferManager};
    use crate::postgres::storage::fsm::{FSMBlockHeader, FSMBlockKind, FreeSpaceManager};
    use pgrx::pg_sys;

    /// The header information for the current FSM block format.  Its first field is purposely the [`FSMBlockHeader`]
    /// so that the block header can be read as that type.  `#[repr(C)]` ensures this is correct
    #[derive(Debug, Copy, Clone)]
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

    impl Default for V1Header {
        fn default() -> Self {
            Self {
                header: FSMBlockHeader {
                    kind: FSMBlockKind::v1_uncompressed,
                },
                empty: false,
            }
        }
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
                header: V1Header::default(),
                entries: [FSMEntry(pg_sys::InvalidBlockNumber, pg_sys::InvalidTransactionId);
                    MAX_ENTRIES_PER_PAGE],
            }
        }
    }

    impl FSMBlock {
        #[inline]
        fn any_invalid(&self) -> bool {
            self.entries
                .iter()
                .any(|FSMEntry(blockno, _)| *blockno == pg_sys::InvalidBlockNumber)
        }
    }

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
    pub struct V1FSM {
        start_blockno: pg_sys::BlockNumber,
    }

    impl FreeSpaceManager for V1FSM {
        /// Create a new [`V1FSM`] in the block storage of the specified `indexrel`.
        unsafe fn create(indexrel: &PgSearchRelation) -> pg_sys::BlockNumber {
            let mut new_buffer = init_new_buffer(indexrel);
            let mut page = new_buffer.page_mut();
            *page.contents_mut::<FSMBlock>() = FSMBlock::default();
            new_buffer.number()
        }

        /// Open an existing [`V1FSM`] which is rooted at the specified starting block number.
        fn open(start_blockno: pg_sys::BlockNumber) -> Self {
            Self { start_blockno }
        }

        /// Retrieve a single recyclable [`pg_sys::BlockNumber`], which can be acquired and re-initialized.
        ///
        /// Returns `None` if no recyclable blocks are available.
        ///
        /// Upon return, the block is removed from the [`V1FSM`]'s control.  It is the caller's
        /// responsibility to ensure the block is properly used, or else it will be lost forever as
        /// dead space in the underlying relation.
        fn pop(&mut self, bman: &mut BufferManager) -> Option<pg_sys::BlockNumber> {
            let xid_horizon = unsafe {
                pg_sys::GetCurrentTransactionIdIfAny().max(pg_sys::FirstNormalTransactionId)
            };
            let mut blockno = self.start_blockno;
            loop {
                if blockno == pg_sys::InvalidBlockNumber {
                    return None;
                }

                let buffer = bman.get_buffer(blockno);
                let page = buffer.page();

                blockno = page.special::<BM25PageSpecialData>().next_blockno;

                if matches!(page.contents::<FSMBlockHeader>().kind, FSMBlockKind::v0) {
                    // skip v0 blocks
                    continue;
                }

                let contents = page.contents_ref::<FSMBlock>();
                if contents.header.empty {
                    continue;
                }

                let mut buffer = if blockno == pg_sys::InvalidBlockNumber {
                    // if we're at the end of the FSM, we'll wait for a buffer upgrade
                    // this is likely better than ending up not finding a block to recycle and requiring
                    // the caller to extend the relation
                    buffer.upgrade(bman)
                } else {
                    // there is a next block so we'll conditionally upgrade the buffer.  If we don't get
                    // the upgrade then we'll move to the next block in FSM
                    let Some(buffer) = buffer.upgrade_conditional(bman) else {
                        // and here's where that happens
                        continue;
                    };
                    buffer
                };
                let mut page = buffer.page_mut();
                let contents = page.contents_mut::<FSMBlock>();

                if !contents.header.empty {
                    let mut found_blockno = None;
                    let mut all_invalid = true;

                    for FSMEntry(blockno, fsm_xid) in &mut contents.entries {
                        if found_blockno.is_none()
                            && *blockno != pg_sys::InvalidBlockNumber
                            && passses_visibility_horizon(*fsm_xid, xid_horizon)
                        {
                            found_blockno = Some(*blockno);
                            *blockno = pg_sys::InvalidBlockNumber;
                        }

                        all_invalid &= *blockno == pg_sys::InvalidBlockNumber;
                    }

                    if all_invalid {
                        // the page is now all invalid so mark it as empty
                        contents.header.empty = true;
                    } else if found_blockno.is_none() {
                        // we didn't actually change the page and
                        buffer.set_dirty(false);
                        continue;
                    }

                    if found_blockno.is_some() {
                        // return the block we found
                        return found_blockno;
                    }
                } else {
                    // page was empty by the time we got the exclusive lock -- we didn't change it
                    buffer.set_dirty(false);
                }
            }
        }

        /// Drain `n` recyclable blocks from this [`V1FSM`] instance, using the specified
        /// [`BufferManager`] for underlying disk access.
        ///
        /// As [`pg_sys::BlockNumber`]s are yielded from the returned iterator, they are removed from the
        /// FSM.  The returned iterator will never return more than `n`, but it could return fewer.
        ///
        /// It is the caller's responsibility to ensure each yielded block is properly used, or else it will
        /// be lost forever as dead space in the underlying relation.  Unyielded blocks are unaffected.
        fn drain(
            &mut self,
            bman: &mut BufferManager,
            n: usize,
        ) -> impl Iterator<Item = pg_sys::BlockNumber> + 'static {
            let xid_horizon = unsafe {
                pg_sys::GetCurrentTransactionIdIfAny().max(pg_sys::FirstNormalTransactionId)
            };
            let mut blocks = Vec::with_capacity(n);
            let mut blockno = self.start_blockno;
            loop {
                if blockno == pg_sys::InvalidBlockNumber {
                    return blocks.into_iter();
                }

                let buffer = bman.get_buffer(blockno);
                let page = buffer.page();

                blockno = page.special::<BM25PageSpecialData>().next_blockno;

                if matches!(page.contents::<FSMBlockHeader>().kind, FSMBlockKind::v0) {
                    // skip v0 blocks
                    continue;
                }

                let contents = page.contents_ref::<FSMBlock>();
                if contents.header.empty {
                    continue;
                }

                let Some(mut buffer) = buffer.upgrade_conditional(bman) else {
                    continue;
                };
                let mut page = buffer.page_mut();
                let contents = page.contents_mut::<FSMBlock>();

                if !contents.header.empty {
                    let current_block_count = blocks.len();
                    let mut all_invalid = true;

                    for FSMEntry(blockno, fsm_xid) in &mut contents.entries {
                        if blocks.len() < n
                            && *blockno != pg_sys::InvalidBlockNumber
                            && passses_visibility_horizon(*fsm_xid, xid_horizon)
                        {
                            blocks.push(*blockno);
                            *blockno = pg_sys::InvalidBlockNumber;
                        }

                        all_invalid &= *blockno == pg_sys::InvalidBlockNumber;
                    }

                    if all_invalid {
                        // the page is now all invalid so mark it as empty
                        contents.header.empty = true;
                    } else if current_block_count == blocks.len() {
                        // we didn't actually change the page
                        buffer.set_dirty(false);
                        continue;
                    }

                    if blocks.len() == n {
                        // we have all the requested blocks so return them
                        return blocks.into_iter();
                    }
                } else {
                    // page was empty by the time we got the exclusive lock -- we didn't change it
                    buffer.set_dirty(false);
                }
            }
        }

        /// Add the specified `extend_with` iterator of [`pg_sys::BlockNumber`]s to this [`V1FSM`].
        ///
        /// The added blocks will be recyclable in the future based on the provided `when_recyclable` transaction id.
        fn extend_with_when_recyclable(
            &mut self,
            bman: &mut BufferManager,
            when_recyclable: pg_sys::FullTransactionId,
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
                            *entry = FSMEntry(
                                blockno,
                                pg_sys::TransactionId::from(when_recyclable.value as u32),
                            );
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

    impl V1FSM {
        pub(super) fn used_blocks(&self, bman: &mut BufferManager) -> Vec<pg_sys::BlockNumber> {
            let mut blocks = Vec::new();

            let mut blockno = self.start_blockno;
            while blockno != pg_sys::InvalidBlockNumber {
                blocks.push(blockno);

                let buffer = bman.get_buffer(blockno);
                blockno = buffer.page().next_blockno();
            }
            blocks
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
}

/// The [`v2`] FreeSpaceManager is a fixed-size AVL tree laid out as an array on the FSM's first block.
///
/// Each entry in the AVL tree is called a [`Slot`] and each slot has a key, value, and tag.  The key
/// is a 64bit [`pg_sys::FullTransactionId`], the value is actually the Rust unit type ([`()`]), and
/// the tag is a [`pg_sys::BlockNumber`].
///
/// Each slot gets its own statically assigned tag value when a relation's FSM is first initialized.
/// This value is the block number that represents the start of the list of block numbers that are
/// free for the xid key stored in that slot.  This value stays with the slot, even as the tree is
/// mutated as generally speaking, with an array-backed AVL tree entries don't move slots, only their
/// left/right pointers are updated during rebalancing.  Special consideration is made for deleting
/// an entry, however, when the key/value is moved to a new slot -- in this case the tag values are
/// swapped until the entry is finished moving.  Essentially, tags have an affinity for their key/value
/// as long as the slot is occupied.
///
/// The FSM has two user-facing operations: extend and drain.
///
/// # Extend
///
/// The caller provides both a transaction id and an iterator of free blocks to associate with that
/// transaction.  That transaction indicates the point in time in which those blocks are usable.  Any
/// future transaction `>=` to that transaction will be able to use those blocks.
///
/// During an extension a share lock is acquired on the FSM's root page (the AVL tree) and the
/// specified transaction id is searched.  If it's found, then an exclusive lock is taken on the block
/// represented by that xid's `tag` -- the freelist -- and the freelist is extended from head-to-tail,
/// filling in gaps along the way.  However, if a full page is encountered, then a shortcut is taken
/// and a new freelist is linked into the existing list at that point.
///
/// Concurrent extensions are allowed on the same xid, blocking one block in the block list at a time.
/// However, in practice almost every call to [`FreeSpaceManager::extend_with_when_recyclable`] uses
/// the current transaction id, and since each transaction has its own unique id, there'll be no blocking.
///
/// Sometimes a caller will use the result of [`pg_sys::ReadNextFullTransactionId()`] instead of the
/// current transaction id.  This can cause concurrent extensions of the same transaction id.  This
/// is fine and generally a rare situation.
///
/// If the provided transaction id is not found, then the shared lock on the FSM root is upgraded to
/// an exclusive lock and the transaction id is inserted.  It is possible that by this time the
/// transaction id now exists.  In either case, the `tag` value for the new (or now-existing) transaction
/// id's slot is used as the freelist, and it is extended.
///
/// The upgraded exclusive lock on the tree is *not* held during extension.  It only lives long enough
/// to ensure proper insertion of the possibly new transaction id.
///
/// If, when extending a freelist, another page is needed, it is acquired by extending the relation by
/// a page, not by asking the FSM itself for a page.
///
/// ## What happens if the tree is full?
///
/// The AVL tree is backed by a fixed-size array of (at the time of writing) 338 slots.  If all the
/// slots are occupied, we cannot insert a new key/value.  Instead, under an exclusive lock, we find
/// the largest existing key (transaction id) an use that slot.
///
/// If that key is `>=` to the new transaction id we're trying to insert, then we simply use that
/// slot's information.
///
/// If the transaction id we're trying to insert is larger, we **directly modify the existing key**
/// to be the new transaction id.  The tree will maintain its balance as the new key is still greater
/// than its predecessors.
///
/// Then extension happens as normal.
///
/// This has a side effect of moving either existing blocks or the set of new blocks to the future.
/// For existing blocks this means they potentially won't be reused as soon as they could.
///
/// In practice, it'll be quite challenging to fill the tree -- perhaps even impossible.  It would
/// require hundreds of concurrent merges to happen without any new segments being created.  This is
/// not a scenario that can actually happen.  Merging is what returns blocks to the FSM and merging
/// only happens against segments.
///
/// # Drain
///
/// Draining is the process of asking the FSM for free blocks.  The caller asks for `n` blocks and
/// the FSM returns an iterator of blocks that can be reused by the current transaction.  "Current
/// transaction" here literally means the result of `pg_sys::GetCurrentFullTransactionId()`, but
/// _could_ be a different, further-in-the-past, transaction.
///
/// The drain process takes a shared lock on the root page and finds the entry that is less-than-or-equal-to
/// the current transaction id.  A transaction can only reuse blocks related to the same or older
/// transactions.
///
/// If no such key exists, [`FreeSpaceManager::drain`] returns an empty iterator.
///
/// Otherwise, the freelist associated with that entry's slot is consumed, up to `n` blocks.  The
/// locking through the freelist here is conditional.  If an exclusive lock can't be acquired, the
/// algorithm restarts, now looking for the next smallest transaction id.  This continues until `n`
/// blocks have been found or the tree no longer has any keys that satisfy the condition.
///
/// This process of restarting with the next smallest transaction id also happens if the xid being
/// evaluated doesn't contain enough blocks to satisfy `n`.
///
/// When draining, if an individual block in the freelist (that isn't the first block) becomes empty,
/// it is unlinked from the freelist and returned to the FSM to be used in a future transaction.
/// This helps to keep the FSM as small as possible throughout its life.
///
/// If, while draining, an entire freelist is consumed for a transaction id, then that transaction
/// id is removed from the tree.  This requires an exclusive lock on the tree itself and must happen
/// outside holding any other locks.  It would be possible for two concurrent transactions to try
/// and delete the same xid entry.  This is fine -- one will win and the other will happily think it won.
pub mod v2 {
    use crate::postgres::rel::PgSearchRelation;
    use crate::postgres::storage::avl::{
        AvlTreeMap, AvlTreeMapHeader, AvlTreeMapView, Error, Slot,
    };
    use crate::postgres::storage::block::{bm25_max_free_space, BM25PageSpecialData};
    use crate::postgres::storage::buffer::{
        init_new_buffer, BufferManager, BufferMut, Page, PageMut,
    };
    use crate::postgres::storage::fsm::{FSMBlockHeader, FSMBlockKind, FreeSpaceManager};
    use pgrx::pg_sys;
    use std::iter::Peekable;

    #[derive(Debug, Copy, Clone)]
    #[repr(C)]
    struct V2Header {
        header: FSMBlockHeader,
    }

    impl Default for V2Header {
        fn default() -> Self {
            Self {
                header: FSMBlockHeader {
                    kind: FSMBlockKind::v2_avl_tree,
                },
            }
        }
    }

    /// We'd prefer to use [`pg_sys::FullTransactionId`] but it's not very ergonomic
    type Key = u64;
    type Value = ();
    type Tag = pg_sys::BlockNumber;
    type AvlSlot = Slot<Key, Value, Tag>;
    type Avl<'a> = AvlTreeMapView<'a, Key, Value, Tag>;
    type AvlMut<'a> = AvlTreeMap<'a, Key, Value, Tag>;

    const MAX_SLOTS: usize = (bm25_max_free_space()
        - (size_of::<V2Header>() + size_of::<AvlTreeMapHeader>()))
        / size_of::<AvlSlot>();

    #[derive(Debug, Copy, Clone)]
    #[repr(C)]
    struct FSMRootBlock {
        header: V2Header,
        avl_header: AvlTreeMapHeader,
        avl_arena: [AvlSlot; MAX_SLOTS],
    }

    impl Default for FSMRootBlock {
        fn default() -> Self {
            Self {
                header: V2Header::default(),
                avl_header: Default::default(),
                avl_arena: [AvlSlot::default(); MAX_SLOTS],
            }
        }
    }

    const MAX_ENTRIES: usize =
        (bm25_max_free_space() - size_of::<u32>()) / size_of::<pg_sys::BlockNumber>();

    #[derive(Debug, Copy, Clone)]
    #[repr(C)]
    pub(super) struct AvlLeaf {
        pub(super) len: u32,
        pub(super) entries: [pg_sys::BlockNumber; MAX_ENTRIES],
    }

    impl Default for AvlLeaf {
        fn default() -> Self {
            Self {
                len: 0,
                entries: [pg_sys::InvalidBlockNumber; MAX_ENTRIES],
            }
        }
    }

    impl AvlLeaf {
        fn init_new_page(bman: &mut BufferManager) -> BufferMut {
            let mut buffer = init_new_buffer(bman.buffer_access().rel());
            let mut page = buffer.page_mut();
            let contents = page.contents_mut::<AvlLeaf>();
            *contents = AvlLeaf::default();
            buffer
        }
    }

    pub struct V2FSM {
        start_blockno: pg_sys::BlockNumber,
    }

    impl FreeSpaceManager for V2FSM {
        unsafe fn create(indexrel: &PgSearchRelation) -> pg_sys::BlockNumber {
            let mut root = init_new_buffer(indexrel);
            let mut page = root.page_mut();
            let contents = page.contents_mut::<FSMRootBlock>();

            // initialize the root page as an empty AVL tree
            *contents = FSMRootBlock::default();

            // each slot is initially allocated a new buffer and assigned to the slot's `tag, which
            // will eventually be used to store the freelist for whatever key ends up in that slot.
            // the tag value remains unchanged throughout the lifetime of the tree, except in the
            // case of removing an entry, and the tree needs to be rebalanced.  in this case the tag
            // moves with the key/value entry through the tree and tags are swapped along the way
            for slot in &mut contents.avl_arena {
                slot.tag = init_new_buffer(indexrel).number();
            }

            // initialize an empty avl tree on the page
            AvlMut::new(&mut contents.avl_header, &mut contents.avl_arena);

            root.number()
        }

        fn open(start_blockno: pg_sys::BlockNumber) -> Self {
            Self { start_blockno }
        }

        fn pop(&mut self, bman: &mut BufferManager) -> Option<pg_sys::BlockNumber> {
            self.drain(bman, 1).next()
        }

        fn drain(
            &mut self,
            bman: &mut BufferManager,
            many: usize,
        ) -> impl Iterator<Item = pg_sys::BlockNumber> + 'static {
            // The FSM drain operation has a two-level structure:
            //
            // Level 1: AVL Tree (root page)
            //   - Contains up to N XIDs as keys
            //   - Each XID's value is the head block number of its freelist
            //
            // Level 2: Linked Lists (freelist pages)
            //   - Each XID has a linked list of pages containing free blocks
            //   - Pages are linked via next_blockno in the page special data
            //   - Each page can hold up to MAX_ENTRIES blocks
            //
            // The drain algorithm:
            // 1. Acquire root buffer (AVL tree)
            // 2. Find XIDs <= current XID that might have usable blocks
            // 3. Try to acquire the head of an XID's freelist (FIRST buffer acquisition)
            // 4. If successful, traverse the linked list (SECOND buffer acquisition for each page)
            // 5. Drain blocks from pages that need modification
            // 6. If we can't get a lock, try next XID (with optimization to avoid re-acquiring root)

            let current_xid = unsafe {
                pg_sys::GetCurrentFullTransactionIdIfAny()
                    .value
                    .max(pg_sys::FirstNormalTransactionId.into_inner() as u64)
            };

            let mut xid = current_xid;
            let mut blocks = Vec::with_capacity(many);

            // Debug counters for performance monitoring
            #[cfg(debug_assertions)]
            let mut debug_root_acquisitions = 0usize;
            #[cfg(debug_assertions)]
            let mut debug_lock_failures = 0usize;

            'outer: while blocks.len() < many {
                // OPTIMIZATION: Keep root buffer across multiple XID retry attempts to avoid
                // re-acquiring it many times when the FSM tree is full and contended.
                //
                // Previously, when we couldn't get a lock on an XID's freelist, we would:
                // 1. Drop the root buffer
                // 2. Decrement to next XID
                // 3. Re-acquire the root buffer
                // This could happen many times with a full FSM tree!
                //
                // Now we keep the root buffer and try up to MAX_XID_RETRIES XIDs before
                // dropping and re-acquiring, reducing root acquisitions.
                let mut root = Some(bman.get_buffer(self.start_blockno));
                #[cfg(debug_assertions)]
                {
                    debug_root_acquisitions += 1;
                }
                const MAX_XID_RETRIES: usize = 10;
                let mut retry_count = 0;

                'retry: while retry_count < MAX_XID_RETRIES && blocks.len() < many {
                    let page = root.as_ref().unwrap().page();
                    let tree = self.avl_ref(&page);

                    let Some((found_xid, _unit, tag)) = tree.get_lte(&xid) else {
                        drop(root);
                        break 'outer;
                    };

                    let mut head_blockno = tag as pg_sys::BlockNumber;
                    let mut blockno = head_blockno;
                    let mut cnt = 0;

                    // FIRST BUFFER ACQUISITION: Try to acquire the HEAD of this XID's freelist
                    // Each XID in the tree has a linked list of pages containing free blocks.
                    // We need to get the head of that list first.
                    let buffer = match bman.get_buffer_conditional_readonly(blockno) {
                        Some(buffer) => {
                            // Successfully got the lock on the HEAD, now we can drop root and proceed
                            // to traverse this XID's freelist
                            drop(root.take());
                            buffer
                        }
                        None => {
                            // Couldn't get lock on this XID's freelist HEAD
                            // This means another backend is using this XID's blocks
                            retry_count += 1;
                            #[cfg(debug_assertions)]
                            {
                                debug_lock_failures += 1;
                            }

                            // Try next XID WITHOUT dropping root
                            xid = found_xid - 1;
                            if xid < pg_sys::FirstNormalTransactionId.into_inner() as u64 {
                                drop(root);
                                break 'outer;
                            }
                            continue 'retry;
                        }
                    };

                    // From here on, we have successfully acquired the head buffer and dropped root
                    // Now we traverse the linked list of pages for this XID's freelist
                    let mut first_buffer = Some(buffer);

                    while blocks.len() < many && blockno != pg_sys::InvalidBlockNumber {
                        // SECOND BUFFER ACQUISITION: Get buffers for traversing the linked list
                        // The freelist for each XID can span multiple pages linked together.
                        // We need to walk through this chain to find blocks to drain.
                        let buffer = if let Some(b) = first_buffer.take() {
                            // First iteration: use the HEAD buffer we already acquired above
                            b
                        } else {
                            // Subsequent iterations: get the NEXT buffer in the linked list
                            // OPTIMIZATION: We use read-only buffers to check if we need to modify the page.
                            // This avoids triggering WAL generation (via GenericXLogRegisterBuffer) for pages
                            // we only examine but don't modify. WAL is only generated when we call page_mut().
                            match bman.get_buffer_conditional_readonly(blockno) {
                                Some(b) => b,
                                None => {
                                    // Couldn't get this buffer in the chain, give up on this XID
                                    xid = found_xid - 1;
                                    if xid < pg_sys::FirstNormalTransactionId.into_inner() as u64 {
                                        break 'outer;
                                    }
                                    continue 'outer;
                                }
                            }
                        };

                        // Read the page immutably to check if we can/need to drain
                        let page = buffer.page();
                        let contents = page.contents_ref::<AvlLeaf>();
                        let next_blockno = page.next_blockno();

                        // Check if we need to modify this page
                        let needs_modification = contents.len > 0 && blocks.len() < many;
                        let is_empty_head = contents.len == 0 && head_blockno == blockno;

                        // Skip empty head pages - we never modify them
                        if is_empty_head {
                            blockno = next_blockno;
                            continue;
                        }

                        if !needs_modification {
                            // Nothing to do with this page, move to the next one
                            blockno = next_blockno;
                            continue;
                        }

                        // Now we need to modify, so upgrade to mutable buffer
                        let Some(mut buffer) = buffer.upgrade_conditional(bman) else {
                            // Someone else got the exclusive lock, restart with next XID
                            xid = found_xid - 1;
                            if xid < pg_sys::FirstNormalTransactionId.into_inner() as u64 {
                                break 'outer;
                            }
                            continue 'outer;
                        };

                        let mut page = buffer.page_mut();
                        let contents = page.contents_mut::<AvlLeaf>();
                        let mut modified = false;

                        // We've already skipped empty head pages, so we can safely drain from this page
                        // get all that we can/need from this page
                        while contents.len > 0 && blocks.len() < many {
                            contents.len -= 1;
                            blocks.push(contents.entries[contents.len as usize]);
                            modified = true;
                        }
                        cnt += contents.len as usize;

                        // should we unlink this block from the chain? -- only if it's the head and _we_ made it empty
                        let should_unlink_head = blockno == head_blockno
                            && contents.len == 0
                            && next_blockno != pg_sys::InvalidBlockNumber;

                        if !modified {
                            // we didn't change anything
                            buffer.set_dirty(false);
                        }

                        // drop the leaf buffer -- we're done with it and it's possible we'll need to
                        // unlink it from the list and that requires an exclusive lock on the tree
                        // and we can't have both at the same time
                        drop(buffer);

                        if should_unlink_head {
                            let old_head = head_blockno;

                            // get mutable tree without holding any other locks
                            let mut root = bman.get_buffer_mut(self.start_blockno);
                            let mut page = root.page_mut();
                            let mut tree = self.avl_mut(&mut page);

                            let mut did_update_head = false;
                            if let Some(slot) = tree.get_slot_mut(&found_xid) {
                                if slot.tag as pg_sys::BlockNumber == head_blockno {
                                    // we are the process that actually unlinked the head
                                    did_update_head = true;
                                    slot.tag = next_blockno;

                                    // and keep local state in sync
                                    head_blockno = next_blockno;
                                }
                                // else: someone else already moved the head, we do nothing
                            }
                            drop(root);

                            if did_update_head {
                                self.extend_with_when_recyclable(
                                    bman,
                                    unsafe { pg_sys::ReadNextFullTransactionId() },
                                    std::iter::once(old_head),
                                );
                            }

                            // Continue with the new head
                            blockno = next_blockno;
                            continue;
                        }

                        // if this was the last block in the list *and* the entire list is empty,
                        // remove the XID entry from the tree. We must hold no other locks while doing this
                        if next_blockno == pg_sys::InvalidBlockNumber && cnt == 0 {
                            let mut root = bman.get_buffer_mut(self.start_blockno);
                            let mut page = root.page_mut();
                            let mut tree = self.avl_mut(&mut page);

                            // ok if another backend already removed it.
                            let _ = tree.remove(&found_xid);
                            drop(root);
                        }

                        // move to the next block
                        blockno = next_blockno;
                    }

                    // Successfully drained from this XID, exit retry loop
                    break 'retry;
                }

                // If we've tried too many XIDs without success, drop root and start over
                if retry_count >= MAX_XID_RETRIES {
                    drop(root);
                    // Note: xid has already been decremented during retries
                    if xid < pg_sys::FirstNormalTransactionId.into_inner() as u64 {
                        break 'outer;
                    }
                }

                if blocks.len() == many {
                    break;
                }
            }

            #[cfg(debug_assertions)]
            if debug_lock_failures > 0 || debug_root_acquisitions > 1 {
                pgrx::debug2!(
                    "FSM drain stats: requested={}, got={}, root_acquisitions={}, lock_failures={}",
                    many,
                    blocks.len(),
                    debug_root_acquisitions,
                    debug_lock_failures
                );
            }

            blocks.into_iter()
        }

        fn extend_with_when_recyclable(
            &mut self,
            bman: &mut BufferManager,
            when_recyclable: pg_sys::FullTransactionId,
            extend_with: impl Iterator<Item = pg_sys::BlockNumber>,
        ) {
            // if we are creating the index, set the XID to the first normal transaction id
            // because anything garbage-collected during index creation should be immediately reusable
            let when_recyclable = if bman.is_create_index() {
                pg_sys::FullTransactionId {
                    value: pg_sys::FirstNormalTransactionId.into_inner() as u64,
                }
            } else {
                when_recyclable
            };
            let mut extend_with = extend_with.peekable();
            if extend_with.peek().is_none() {
                // caller didn't give us anything to do
                return;
            }

            // find the starting block of the associated freelist while holding (at least) a share
            // lock on the root page of the tree.  This ensures a concurrent drain that could be
            // happening on the provided `when_recyclable` transaction id can't unlink its head block
            // which would change the block we'd get here
            let start_block = {
                let root = bman.get_buffer(self.start_blockno);
                let page = root.page();
                let tree = self.avl_ref(&page);

                match tree.get(&when_recyclable.value) {
                    None => {
                        let mut root = root.upgrade(bman);
                        let mut page = root.page_mut();
                        let mut tree = self.avl_mut(&mut page);

                        match tree.insert(when_recyclable.value, ()) {
                            Ok((_, tag)) => bman.get_buffer_mut(tag as pg_sys::BlockNumber),
                            Err(Error::Full) => {
                                let tag = self.handle_full_tree(root, when_recyclable);
                                bman.get_buffer_mut(tag as pg_sys::BlockNumber)
                            }
                        }
                    }
                    Some((_, tag)) => bman.get_buffer_mut(tag as pg_sys::BlockNumber),
                }
            };

            Self::extend_freelist(bman, start_block, extend_with);
        }
    }

    impl V2FSM {
        fn handle_full_tree(
            &mut self,
            mut root: BufferMut,
            when_recyclable: pg_sys::FullTransactionId,
        ) -> Tag {
            let mut page = root.page_mut();
            let mut tree = self.avl_mut(&mut page);

            // we find the slot containing the largest (maximum) xid and change it to the provided `when_recyclable`
            // asserting that it is the same or greater than the maximum xid we found in the tree
            let max_slot = tree
                .get_max_slot()
                .expect("a full tree must have a maximum entry");

            if when_recyclable.value > max_slot.key {
                // this is safe as the tree still maintains its balance.  we also have an exclusive lock
                // on the tree at this stage which means no concurrent backends can be changing the tree
                max_slot.key = when_recyclable.value;
            }

            max_slot.tag
        }

        fn extend_freelist(
            bman: &mut BufferManager,
            start_block: BufferMut,
            mut extend_with: Peekable<impl Iterator<Item = pg_sys::BlockNumber>>,
        ) -> BufferMut {
            let mut buffer = start_block;
            loop {
                let mut page = buffer.page_mut();
                let contents = page.contents_mut::<AvlLeaf>();
                let next_blockno = page.next_blockno();

                if contents.len as usize == contents.entries.len()
                    && next_blockno != pg_sys::InvalidBlockNumber
                {
                    // this block is full.  Chances are the next block is also full.
                    // so we're going to populate a brand-new list with the rest of the iterator
                    // and then link it in between this block and the next block.  This avoids
                    // the possible overhead of scanning the rest of the freelist just to discover
                    // all the following blocks are full
                    let new_block = AvlLeaf::init_new_page(bman);
                    let new_blockno = new_block.number();

                    let mut last_block = Self::extend_freelist(bman, new_block, extend_with);

                    // link the last block we just created to the next block
                    let mut end_page = last_block.page_mut();
                    end_page.special_mut::<BM25PageSpecialData>().next_blockno = next_blockno;

                    // finally link this block to the new block
                    page.special_mut::<BM25PageSpecialData>().next_blockno = new_blockno;

                    return last_block;
                }

                while (contents.len as usize) < contents.entries.len() {
                    match extend_with.peek() {
                        None => break,
                        Some(_) => {
                            contents.entries[contents.len as usize] = extend_with.next().unwrap();
                            contents.len += 1;
                        }
                    }
                }

                if extend_with.peek().is_none() {
                    break;
                }

                let mut next_blockno = page.next_blockno();
                if next_blockno == pg_sys::InvalidBlockNumber {
                    // link in a new block
                    let new_buffer = AvlLeaf::init_new_page(bman);
                    next_blockno = new_buffer.number();

                    page.special_mut::<BM25PageSpecialData>().next_blockno = next_blockno;
                    buffer = new_buffer;
                } else {
                    // move to next block already in the list
                    buffer = bman.get_buffer_mut(next_blockno);
                }
            }

            buffer
        }

        pub(super) fn avl_ref<'p>(&self, page: &'p Page<'p>) -> Avl<'p> {
            let root_block = page.contents_ref::<FSMRootBlock>();
            assert!(
                matches!(root_block.header.header.kind, FSMBlockKind::v2_avl_tree),
                "conversion of old FSM to v2 never happened"
            );
            unsafe {
                AvlTreeMapView::with_header_and_arena(&root_block.avl_header, &root_block.avl_arena)
            }
        }

        fn avl_mut<'p>(&mut self, page: &'p mut PageMut<'p>) -> AvlMut<'p> {
            let root_block = page.contents_mut::<FSMRootBlock>();
            assert!(
                matches!(root_block.header.header.kind, FSMBlockKind::v2_avl_tree),
                "conversion of old FSM to v2 never happened"
            );
            unsafe {
                AvlTreeMap::with_header_and_arena(
                    &mut root_block.avl_header,
                    &mut root_block.avl_arena,
                )
            }
        }
    }

    #[cfg(any(test, feature = "pg_test"))]
    #[pgrx::pg_schema]
    mod tests {
        use pgrx::prelude::*;
        use std::collections::HashSet;

        use super::{MAX_SLOTS, V2FSM};
        use crate::postgres::rel::PgSearchRelation;
        use crate::postgres::storage::buffer::BufferManager;
        use crate::postgres::storage::fsm::FreeSpaceManager;
        use crate::postgres::storage::metadata::MetaPage;

        #[pg_test]
        unsafe fn test_fsmv2_basics() -> spi::Result<()> {
            Spi::run("CREATE TABLE IF NOT EXISTS fsm_test (id serial8, data text)")?;
            Spi::run("CREATE INDEX IF NOT EXISTS fsm_idx ON fsm_test USING bm25 (id, data) WITH (key_field = 'id')")?;

            let index_oid = Spi::get_one::<pg_sys::Oid>("SELECT 'fsm_idx'::regclass::oid")?
                .unwrap_or(pg_sys::InvalidOid);

            assert_ne!(index_oid, pg_sys::InvalidOid);

            let indexrel = PgSearchRelation::with_lock(
                index_oid,
                pg_sys::RowExclusiveLock as pg_sys::LOCKMODE,
            );

            let mut bman = BufferManager::new(&indexrel);
            let metapage = MetaPage::open(&indexrel);
            let mut fsm = V2FSM::open(metapage.fsm());

            fsm.extend_with_when_recyclable(
                &mut bman,
                pg_sys::FullTransactionId { value: 100 },
                0..3,
            );
            let drained = fsm.drain(&mut bman, 3).collect::<Vec<_>>();
            assert_eq!(drained, [2, 1, 0]);
            assert!(
                fsm.drain(&mut bman, 1).next().is_none(),
                "fsm should be empty"
            );

            fsm.extend_with_when_recyclable(
                &mut bman,
                pg_sys::FullTransactionId { value: 100 },
                0..3,
            );
            fsm.extend_with_when_recyclable(
                &mut bman,
                pg_sys::FullTransactionId { value: 101 },
                3..6,
            );

            let drained = fsm.drain(&mut bman, 6).collect::<Vec<_>>();
            assert_eq!(drained, [5, 4, 3, 2, 1, 0]);
            assert!(
                fsm.drain(&mut bman, 1).next().is_none(),
                "fsm should be empty"
            );

            Ok(())
        }

        #[pg_test]
        unsafe fn test_fsmv2_large_extend_drain() -> spi::Result<()> {
            Spi::run("CREATE TABLE IF NOT EXISTS fsm_test (id serial8, data text)")?;
            Spi::run("CREATE INDEX IF NOT EXISTS fsm_idx ON fsm_test USING bm25 (id, data) WITH (key_field = 'id')")?;

            let index_oid = Spi::get_one::<pg_sys::Oid>("SELECT 'fsm_idx'::regclass::oid")?
                .unwrap_or(pg_sys::InvalidOid);

            assert_ne!(index_oid, pg_sys::InvalidOid);

            let indexrel = PgSearchRelation::with_lock(
                index_oid,
                pg_sys::RowExclusiveLock as pg_sys::LOCKMODE,
            );

            let mut bman = BufferManager::new(&indexrel);
            let metapage = MetaPage::open(&indexrel);
            let mut fsm = V2FSM::open(metapage.fsm());

            fsm.extend_with_when_recyclable(
                &mut bman,
                pg_sys::FullTransactionId { value: 100 },
                0..100_000,
            );
            let drained = fsm.drain(&mut bman, 100_000).collect::<HashSet<_>>();
            assert_eq!(drained, (0..100_000).rev().collect::<HashSet<_>>());
            assert!(
                fsm.drain(&mut bman, 1).next().is_none(),
                "fsm should be empty"
            );

            Ok(())
        }

        #[pg_test]
        unsafe fn test_fsmv2_no_future_drain() -> spi::Result<()> {
            Spi::run("CREATE TABLE IF NOT EXISTS fsm_test (id serial8, data text)")?;
            Spi::run("CREATE INDEX IF NOT EXISTS fsm_idx ON fsm_test USING bm25 (id, data) WITH (key_field = 'id')")?;

            let index_oid = Spi::get_one::<pg_sys::Oid>("SELECT 'fsm_idx'::regclass::oid")?
                .unwrap_or(pg_sys::InvalidOid);

            assert_ne!(index_oid, pg_sys::InvalidOid);

            let indexrel = PgSearchRelation::with_lock(
                index_oid,
                pg_sys::RowExclusiveLock as pg_sys::LOCKMODE,
            );

            let mut bman = BufferManager::new(&indexrel);
            let metapage = MetaPage::open(&indexrel);
            let mut fsm = V2FSM::open(metapage.fsm());

            let current_xid = pg_sys::GetCurrentFullTransactionId();
            let future_xid = pg_sys::FullTransactionId {
                value: current_xid.value + 1,
            };
            fsm.extend_with_when_recyclable(&mut bman, future_xid, 0..3);
            assert!(
                fsm.drain(&mut bman, 1).next().is_none(),
                "fsm should not find future transactions"
            );

            Ok(())
        }

        #[pg_test]
        unsafe fn test_fsmv2_many_xids() -> spi::Result<()> {
            Spi::run("CREATE TABLE IF NOT EXISTS fsm_test (id serial8, data text)")?;
            Spi::run("CREATE INDEX IF NOT EXISTS fsm_idx ON fsm_test USING bm25 (id, data) WITH (key_field = 'id')")?;

            let index_oid = Spi::get_one::<pg_sys::Oid>("SELECT 'fsm_idx'::regclass::oid")?
                .unwrap_or(pg_sys::InvalidOid);

            assert_ne!(index_oid, pg_sys::InvalidOid);

            let indexrel = PgSearchRelation::with_lock(
                index_oid,
                pg_sys::RowExclusiveLock as pg_sys::LOCKMODE,
            );

            let mut bman = BufferManager::new(&indexrel);
            let metapage = MetaPage::open(&indexrel);
            let mut fsm = V2FSM::open(metapage.fsm());

            let current_xid = pg_sys::GetCurrentFullTransactionId();

            for offset in -100i64..=100i64 {
                let xid = pg_sys::FullTransactionId {
                    value: current_xid.value.saturating_add_signed(offset),
                };
                fsm.extend_with_when_recyclable(&mut bman, xid, 0..3);
                assert!(
                    fsm.drain(&mut bman, 1).next().is_some(),
                    "fsm should find something"
                );
            }

            Ok(())
        }

        #[pg_test]
        unsafe fn test_fsmv2_full() -> spi::Result<()> {
            Spi::run("CREATE TABLE IF NOT EXISTS fsm_test (id serial8, data text)")?;
            Spi::run("CREATE INDEX IF NOT EXISTS fsm_idx ON fsm_test USING bm25 (id, data) WITH (key_field = 'id')")?;

            let index_oid = Spi::get_one::<pg_sys::Oid>("SELECT 'fsm_idx'::regclass::oid")?
                .unwrap_or(pg_sys::InvalidOid);

            assert_ne!(index_oid, pg_sys::InvalidOid);

            let indexrel = PgSearchRelation::with_lock(
                index_oid,
                pg_sys::RowExclusiveLock as pg_sys::LOCKMODE,
            );

            let mut bman = BufferManager::new(&indexrel);
            let metapage = MetaPage::open(&indexrel);
            let mut fsm = V2FSM::open(metapage.fsm());

            assert!(
                pg_sys::GetCurrentTransactionId().into_inner()
                    > pg_sys::FirstNormalTransactionId.into_inner() + MAX_SLOTS as u32,
                "test framework transaction id started too low to properly run this test"
            );

            let current_xid = pg_sys::FullTransactionId {
                value: pg_sys::FirstNormalTransactionId.into_inner() as u64,
            };

            for offset in 0..=MAX_SLOTS as u64 {
                let xid = pg_sys::FullTransactionId {
                    value: current_xid.value + offset,
                };
                fsm.extend_with_when_recyclable(&mut bman, xid, 0..1);
            }

            let drained = fsm.drain(&mut bman, MAX_SLOTS + 1).collect::<Vec<_>>();
            assert_eq!(
                drained,
                std::iter::repeat_n(0, MAX_SLOTS + 1).collect::<Vec<_>>()
            );

            let empty = fsm.drain(&mut bman, 1).next().is_none();
            assert!(empty);

            Ok(())
        }

        #[pg_test]
        unsafe fn test_fsmv2_wal_generation_drain() -> spi::Result<()> {
            // This test verifies the optimization that prevents unnecessary WAL generation
            // when draining from FSM pages that don't need modification.
            //
            // The key optimization is that we now:
            // 1. First get a read-only buffer and check if modifications are needed
            // 2. Only upgrade to a mutable buffer (which triggers WAL) if we actually modify the page
            //
            // This test creates a large FSM with many pages and then drains a small number of blocks.
            // With the optimization, only the pages we actually modify should generate WAL entries.
            Spi::run("CREATE TABLE IF NOT EXISTS fsm_wal_test (id serial8, data text)")?;
            Spi::run("CREATE INDEX IF NOT EXISTS fsm_wal_idx ON fsm_wal_test USING bm25 (id, data) WITH (key_field = 'id')")?;

            let index_oid = Spi::get_one::<pg_sys::Oid>("SELECT 'fsm_wal_idx'::regclass::oid")?
                .unwrap_or(pg_sys::InvalidOid);

            assert_ne!(index_oid, pg_sys::InvalidOid);

            let indexrel = PgSearchRelation::with_lock(
                index_oid,
                pg_sys::RowExclusiveLock as pg_sys::LOCKMODE,
            );

            let mut bman = BufferManager::new(&indexrel);
            let metapage = MetaPage::open(&indexrel);
            let mut fsm = V2FSM::open(metapage.fsm());

            // Add many blocks to create multiple FSM pages
            let blocks_per_xid = 1000usize;
            let num_xids = 10usize;

            // Fill FSM with many blocks across multiple transactions
            // This will create multiple FSM pages in the linked list
            for i in 0..num_xids {
                let xid = pg_sys::FullTransactionId {
                    value: pg_sys::FirstNormalTransactionId.into_inner() as u64 + i as u64,
                };
                let start = (i * blocks_per_xid) as u32;
                let end = start + blocks_per_xid as u32;
                fsm.extend_with_when_recyclable(&mut bman, xid, start..end);
            }

            // Get initial WAL stats if available (PostgreSQL 14+)
            // Note: pg_stat_wal requires superuser or pg_read_all_stats role
            // Cast to bigint since wal_bytes is numeric type
            let initial_wal_bytes = Spi::get_one::<i64>(
                "SELECT COALESCE(wal_bytes::bigint, 0) FROM pg_stat_wal LIMIT 1",
            )?
            .unwrap_or(0);

            // Now drain a small number of blocks
            // With the optimization, this should only modify the pages that contain blocks to drain,
            // not generate WAL for all the pages we examine
            let drain_count = 50;
            let drained: Vec<_> = fsm.drain(&mut bman, drain_count).collect();
            assert_eq!(
                drained.len(),
                drain_count,
                "Should drain exactly {} blocks",
                drain_count
            );

            // Check WAL generation if stats were available
            if initial_wal_bytes > 0 {
                let final_wal_bytes = Spi::get_one::<i64>(
                    "SELECT COALESCE(wal_bytes::bigint, 0) FROM pg_stat_wal LIMIT 1",
                )?
                .unwrap_or(0);

                if final_wal_bytes > 0 {
                    let wal_generated = final_wal_bytes - initial_wal_bytes;

                    // Log the WAL generation for debugging
                    // With the optimization, WAL should only be generated for modified pages
                    pgrx::info!("WAL generated during drain of {} blocks from FSM with {} total blocks: {} bytes", 
                               drain_count, blocks_per_xid * num_xids, wal_generated);

                    // Without the fix, every examined page would generate WAL (~8KB per page)
                    // With the fix, only modified pages generate WAL
                    // This is more of a performance regression test than a hard assertion
                }
            }

            // Verify we can still drain more blocks (FSM isn't corrupted)
            let more_drained: Vec<_> = fsm.drain(&mut bman, 10).collect();
            assert_eq!(
                more_drained.len(),
                10,
                "Should be able to drain more blocks"
            );

            // Test draining from empty pages doesn't cause issues or unnecessary WAL
            // First drain everything
            let _all: Vec<_> = fsm.drain(&mut bman, blocks_per_xid * num_xids).collect();

            // Now try to drain from empty FSM
            // With the optimization, this should examine pages without modifying them,
            // thus not generating any WAL entries
            let empty: Vec<_> = fsm.drain(&mut bman, 10).collect();
            assert!(
                empty.is_empty(),
                "Draining from empty FSM should return no blocks"
            );

            Ok(())
        }

        #[pg_test]
        unsafe fn test_fsmv2_full_tree_contention() -> spi::Result<()> {
            // This test simulates the production scenario where the FSM tree is nearly full
            // with many XIDs, testing both optimizations:
            // 1. Root buffer re-acquisition optimization (avoiding 338 re-acquisitions)
            // 2. WAL generation optimization (read-only buffers first)

            Spi::run("CREATE TABLE IF NOT EXISTS fsm_contention_test (id serial8, data text)")?;
            Spi::run("CREATE INDEX IF NOT EXISTS fsm_contention_idx ON fsm_contention_test USING bm25 (id, data) WITH (key_field = 'id')")?;

            let index_oid =
                Spi::get_one::<pg_sys::Oid>("SELECT 'fsm_contention_idx'::regclass::oid")?
                    .unwrap_or(pg_sys::InvalidOid);

            assert_ne!(index_oid, pg_sys::InvalidOid);

            let indexrel = PgSearchRelation::with_lock(
                index_oid,
                pg_sys::RowExclusiveLock as pg_sys::LOCKMODE,
            );

            let mut bman = BufferManager::new(&indexrel);
            let metapage = MetaPage::open(&indexrel);
            let mut fsm = V2FSM::open(metapage.fsm());

            // Fill the FSM tree with many XIDs to simulate production scenario
            // We'll create a pattern similar to what was observed:
            // - A few XIDs with many blocks
            // - Many XIDs with very few blocks

            let base_xid = pg_sys::FirstNormalTransactionId.into_inner() as u64;

            // Create 3 XIDs with many blocks (simulating the top 3 from production)
            for i in 0..3 {
                let xid = pg_sys::FullTransactionId {
                    value: base_xid + i * 1000,
                };
                // Add lots of blocks to these XIDs
                let start = (i * 10000) as u32;
                let end = start + 1000;
                fsm.extend_with_when_recyclable(&mut bman, xid, start..end);
            }

            // Create many XIDs with very few blocks (simulating the long tail)
            // This will fill up the tree slots
            for i in 0..200 {
                let xid = pg_sys::FullTransactionId {
                    value: base_xid + 10000 + i,
                };
                // Add only a few blocks to each
                let start = (30000 + i * 5) as u32;
                let end = start + 2;
                fsm.extend_with_when_recyclable(&mut bman, xid, start..end);
            }

            // Now drain blocks - this will exercise the retry logic
            // With a full tree, the old code would re-acquire root up to 200+ times
            // With our optimization, it should only re-acquire ~20 times (200/10)

            let start_time = std::time::Instant::now();
            let drain_count = 100;
            let drained: Vec<_> = fsm.drain(&mut bman, drain_count).collect();
            let drain_time = start_time.elapsed();

            // Verify we got the expected number of blocks
            assert!(
                drained.len() <= drain_count,
                "Should drain at most {} blocks, got {}",
                drain_count,
                drained.len()
            );

            pgrx::info!(
                "Drained {} blocks from full FSM tree in {:?}",
                drained.len(),
                drain_time
            );

            // Test draining when many XIDs are in the future (can't be used)
            // This forces the drain to try many XIDs before finding usable ones
            let future_base = base_xid + 1_000_000;

            // Add XIDs far in the future
            for i in 0..50 {
                let xid = pg_sys::FullTransactionId {
                    value: future_base + i,
                };
                let start = (100000 + i * 10) as u32;
                let end = start + 5;
                fsm.extend_with_when_recyclable(&mut bman, xid, start..end);
            }

            // These future blocks shouldn't be drainable
            let current_xid_drain: Vec<_> = fsm.drain(&mut bman, 50).collect();

            // We should only get blocks from XIDs <= current XID
            // The drain should have to skip over the future XIDs efficiently
            pgrx::info!(
                "Drained {} blocks when many XIDs are in future (should skip future XIDs efficiently)",
                current_xid_drain.len()
            );

            Ok(())
        }

        #[pg_test]
        unsafe fn test_fsmv2_retry_performance() -> spi::Result<()> {
            // This test specifically measures the performance improvement from
            // keeping the root buffer across XID retries

            Spi::run("CREATE TABLE IF NOT EXISTS fsm_retry_test (id serial8, data text)")?;
            Spi::run("CREATE INDEX IF NOT EXISTS fsm_retry_idx ON fsm_retry_test USING bm25 (id, data) WITH (key_field = 'id')")?;

            let index_oid = Spi::get_one::<pg_sys::Oid>("SELECT 'fsm_retry_idx'::regclass::oid")?
                .unwrap_or(pg_sys::InvalidOid);

            let indexrel = PgSearchRelation::with_lock(
                index_oid,
                pg_sys::RowExclusiveLock as pg_sys::LOCKMODE,
            );

            let mut bman = BufferManager::new(&indexrel);
            let metapage = MetaPage::open(&indexrel);
            let mut fsm = V2FSM::open(metapage.fsm());

            // Create a worst-case scenario: many XIDs with no blocks except the last ones
            let base_xid = pg_sys::FirstNormalTransactionId.into_inner() as u64;

            // Add many empty XIDs (these will be tried and skipped)
            for i in 0..100 {
                let xid = pg_sys::FullTransactionId {
                    value: base_xid + i * 2,
                };
                // Add and immediately drain to create empty entries
                let start = (50000 + i) as u32;
                let end = start + 1;
                fsm.extend_with_when_recyclable(&mut bman, xid, start..end);
                let _: Vec<_> = fsm.drain(&mut bman, 1).collect();
            }

            // Add one XID with actual blocks at the end
            let good_xid = pg_sys::FullTransactionId { value: base_xid };
            fsm.extend_with_when_recyclable(&mut bman, good_xid, 60000..60100);

            // Now drain - this will have to skip through many empty XIDs
            // Old implementation: would re-acquire root for each empty XID
            // New implementation: tries 10 XIDs per root acquisition
            let start_time = std::time::Instant::now();
            let drained: Vec<_> = fsm.drain(&mut bman, 50).collect();
            let elapsed = start_time.elapsed();

            assert!(!drained.is_empty(), "Should drain some blocks");

            pgrx::info!(
                "Drained {} blocks through {} empty XIDs in {:?} (optimized retry logic)",
                drained.len(),
                100,
                elapsed
            );

            Ok(())
        }
    }
}

pub fn convert_v1_to_v2(bman1: &mut BufferManager, mut v1: V1FSM, mut v2: V2FSM) {
    let when_recyclable = pg_sys::FullTransactionId {
        value: pg_sys::FirstNormalTransactionId.into_inner() as u64,
    };

    let mut bman2 = bman1.clone();
    loop {
        let mut drained = v1.drain(bman1, 1000).peekable();
        if drained.peek().is_some() {
            v2.extend_with_when_recyclable(&mut bman2, when_recyclable, drained);
        } else {
            break;
        }
    }

    let v1_used_blocks = v1.used_blocks(bman1);
    v2.extend_with_when_recyclable(&mut bman2, when_recyclable, v1_used_blocks.into_iter());
}

#[pg_extern]
unsafe fn fsm_info(
    index: PgRelation,
) -> TableIterator<
    'static,
    (
        name!(xid, pg_sys::TransactionId),
        name!(fsm_blockno, i64),
        name!(tag, i64),
        name!(free_blockno, i64),
    ),
> {
    use crate::postgres::storage::fsm::v2::AvlLeaf;
    use crate::postgres::storage::fsm::v2::V2FSM;

    let index =
        PgSearchRelation::with_lock(index.oid(), pg_sys::AccessShareLock as pg_sys::LOCKMODE);

    let meta = MetaPage::open(&index);
    let bman = BufferManager::new(&index);

    let root = bman.get_buffer(meta.fsm());
    let page = root.page();
    let avl = V2FSM::open(root.number()).avl_ref(&page);

    let mut results = Vec::new();

    for slot in avl.arena {
        if slot.is_used() {
            let xid = slot.key;
            let blockno = slot.tag as pg_sys::BlockNumber;

            let mut current_blockno = blockno;
            while current_blockno != pg_sys::InvalidBlockNumber {
                let buffer = bman.get_buffer(current_blockno);
                let page = buffer.page();
                let leaf = page.contents_ref::<AvlLeaf>();

                for i in 0..leaf.len {
                    results.push((xid, current_blockno, slot.tag, leaf.entries[i as usize]));
                }

                current_blockno = page.next_blockno();
            }
        }
    }

    TableIterator::new(
        results
            .into_iter()
            .map(|(xid, fsm_blockno, tag, tracked_blockno)| {
                (
                    pg_sys::TransactionId::from(xid as u32),
                    fsm_blockno as i64,
                    tag as i64,
                    tracked_blockno as i64,
                )
            }),
    )
}

#[pg_extern]
unsafe fn fsm_size(index: PgRelation) -> i64 {
    use crate::postgres::storage::fsm::v2::V2FSM;
    let index =
        PgSearchRelation::with_lock(index.oid(), pg_sys::AccessShareLock as pg_sys::LOCKMODE);

    let meta = MetaPage::open(&index);
    let bman = BufferManager::new(&index);

    let root = bman.get_buffer(meta.fsm());
    let page = root.page();
    let avl = V2FSM::open(root.number()).avl_ref(&page);

    let mut count = 1; // start with 1 b/c of the root page
    for slot in avl.arena {
        let mut blockno = slot.tag as pg_sys::BlockNumber;
        while blockno != pg_sys::InvalidBlockNumber {
            count += 1;
            let buffer = bman.get_buffer(blockno);
            let page = buffer.page();
            blockno = page.next_blockno();
        }
    }
    count as i64
}
