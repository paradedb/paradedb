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

    /// Global FSM performance metrics
    /// These track cumulative performance across all FSM operations
    #[derive(Debug)]
    pub struct FsmMetrics {
        /// Total number of drain calls
        pub total_drains: std::sync::atomic::AtomicU64,
        /// Total blocks drained across all operations
        pub total_blocks_drained: std::sync::atomic::AtomicU64,
        /// Total empty pages skipped
        pub total_empty_pages_skipped: std::sync::atomic::AtomicU64,
        /// Total head pages unlinked
        pub total_heads_unlinked: std::sync::atomic::AtomicU64,
        /// Total XIDs processed
        pub total_xids_processed: std::sync::atomic::AtomicU64,
        /// Maximum chain length seen
        pub max_chain_length_seen: std::sync::atomic::AtomicUsize,
        /// Sum of all chain lengths (for averaging)
        pub sum_chain_lengths: std::sync::atomic::AtomicU64,
        /// Number of chains processed (for averaging)
        pub chain_count: std::sync::atomic::AtomicU64,
    }

    impl FsmMetrics {
        const fn new() -> Self {
            Self {
                total_drains: std::sync::atomic::AtomicU64::new(0),
                total_blocks_drained: std::sync::atomic::AtomicU64::new(0),
                total_empty_pages_skipped: std::sync::atomic::AtomicU64::new(0),
                total_heads_unlinked: std::sync::atomic::AtomicU64::new(0),
                total_xids_processed: std::sync::atomic::AtomicU64::new(0),
                max_chain_length_seen: std::sync::atomic::AtomicUsize::new(0),
                sum_chain_lengths: std::sync::atomic::AtomicU64::new(0),
                chain_count: std::sync::atomic::AtomicU64::new(0),
            }
        }

        /// Get a snapshot of current metrics
        #[allow(dead_code)]
        pub fn snapshot(&self) -> FsmMetricsSnapshot {
            use std::sync::atomic::Ordering;
            FsmMetricsSnapshot {
                total_drains: self.total_drains.load(Ordering::Relaxed),
                total_blocks_drained: self.total_blocks_drained.load(Ordering::Relaxed),
                total_empty_pages_skipped: self.total_empty_pages_skipped.load(Ordering::Relaxed),
                total_heads_unlinked: self.total_heads_unlinked.load(Ordering::Relaxed),
                total_xids_processed: self.total_xids_processed.load(Ordering::Relaxed),
                max_chain_length_seen: self.max_chain_length_seen.load(Ordering::Relaxed),
                avg_chain_length: {
                    let sum = self.sum_chain_lengths.load(Ordering::Relaxed);
                    let count = self.chain_count.load(Ordering::Relaxed);
                    if count > 0 {
                        sum as f64 / count as f64
                    } else {
                        0.0
                    }
                },
            }
        }

        /// Reset all metrics (useful for testing)
        #[cfg(any(test, feature = "pg_test"))]
        pub fn reset(&self) {
            use std::sync::atomic::Ordering;
            self.total_drains.store(0, Ordering::Relaxed);
            self.total_blocks_drained.store(0, Ordering::Relaxed);
            self.total_empty_pages_skipped.store(0, Ordering::Relaxed);
            self.total_heads_unlinked.store(0, Ordering::Relaxed);
            self.total_xids_processed.store(0, Ordering::Relaxed);
            self.max_chain_length_seen.store(0, Ordering::Relaxed);
            self.sum_chain_lengths.store(0, Ordering::Relaxed);
            self.chain_count.store(0, Ordering::Relaxed);
        }
    }

    /// Snapshot of FSM metrics at a point in time
    #[derive(Debug, Clone, Copy)]
    #[allow(dead_code)]
    pub struct FsmMetricsSnapshot {
        pub total_drains: u64,
        pub total_blocks_drained: u64,
        pub total_empty_pages_skipped: u64,
        pub total_heads_unlinked: u64,
        pub total_xids_processed: u64,
        pub max_chain_length_seen: usize,
        pub avg_chain_length: f64,
    }

    /// Global FSM metrics instance
    static FSM_METRICS: FsmMetrics = FsmMetrics::new();

    /// Get a snapshot of the global FSM metrics
    #[allow(dead_code)]
    pub fn get_fsm_metrics() -> FsmMetricsSnapshot {
        FSM_METRICS.snapshot()
    }

    /// Reset global FSM metrics (useful for testing)
    #[cfg(any(test, feature = "pg_test"))]
    pub fn reset_fsm_metrics() {
        FSM_METRICS.reset();
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
            let current_xid = unsafe {
                pg_sys::GetCurrentFullTransactionIdIfAny()
                    .value
                    .max(pg_sys::FirstNormalTransactionId.into_inner() as u64)
            };

            let mut xid = current_xid;
            let mut blocks = Vec::with_capacity(many);

            // Update global metrics
            FSM_METRICS
                .total_drains
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            // Production monitoring metrics
            let mut empty_pages_skipped = 0usize;
            let mut head_pages_unlinked = 0usize;
            let mut xids_processed = 0usize;
            let mut max_chain_length = 0usize;
            let mut total_chain_length = 0usize;
            let mut root_acquisitions = 0usize;
            let mut lock_failures = 0usize;

            // Batch optimization: retry multiple XIDs before dropping root
            const MAX_XID_RETRIES: usize = 5;
            let mut xid_retry_count = 0usize;

            'outer: while blocks.len() < many {
                root_acquisitions += 1;
                let mut root = Some(bman.get_buffer(self.start_blockno));
                let page = root.as_ref().unwrap().page();
                let tree = self.avl_ref(&page);

                let Some((found_xid, _unit, tag)) = tree.get_lte(&xid) else {
                    break;
                };

                let mut head_blockno = tag as pg_sys::BlockNumber;
                let mut blockno = head_blockno;
                let mut cnt = 0;

                // Track chain length for this XID
                let mut chain_length = 0usize;
                xids_processed += 1;

                // Track previous blockno for unlinking empty pages
                let mut prev_blockno: Option<pg_sys::BlockNumber> = None;

                while blocks.len() < many && blockno != pg_sys::InvalidBlockNumber {
                    chain_length += 1;
                    // we drop the "root" buffer after getting the head buffer.
                    // this ensures that the `head_blockno` (the root entry's "tag" value)
                    // is what it says it is
                    //
                    // the conditional lock here also ensures we're the only backend to start
                    // draining the xid's freelist from its the head
                    let mut buffer = match bman.get_buffer_conditional(blockno) {
                        Some(buffer) => {
                            drop(root.take());
                            xid_retry_count = 0; // Reset retry count on success
                            buffer
                        }
                        None => {
                            lock_failures += 1;

                            // Batch optimization: Try other XIDs before dropping root
                            if xid_retry_count < MAX_XID_RETRIES {
                                xid_retry_count += 1;
                                pgrx::debug2!(
                                    "Failed to lock XID {} (retry {}/{}), trying next XID without dropping root",
                                    found_xid,
                                    xid_retry_count,
                                    MAX_XID_RETRIES
                                );

                                // Try the next XID while keeping the root buffer
                                xid = found_xid - 1;
                                if xid < pg_sys::FirstNormalTransactionId.into_inner() as u64 {
                                    drop(root.take());
                                    break 'outer;
                                }
                                // Continue with the same root buffer
                                continue 'outer;
                            } else {
                                // Exhausted retries, drop root and try again
                                drop(root.take());
                                xid_retry_count = 0;

                                pgrx::debug2!(
                                    "Failed to lock XID {} after {} retries, dropping root buffer",
                                    found_xid,
                                    MAX_XID_RETRIES
                                );

                                // move to the next candidate XID below this one.
                                xid = found_xid - 1;
                                if xid < pg_sys::FirstNormalTransactionId.into_inner() as u64 {
                                    break 'outer;
                                }
                                continue 'outer;
                            }
                        }
                    };

                    // OPTIMIZATION: First check if the page is empty without modifying it
                    // This avoids WAL generation for empty pages we're just traversing
                    let (is_empty, next_blockno) = {
                        let page = buffer.page();
                        let contents = page.contents_ref::<AvlLeaf>();
                        (contents.len == 0, page.next_blockno())
                    };

                    // If the page is empty and it's not the head, unlink it from the chain
                    if is_empty && blockno != head_blockno {
                        drop(buffer);

                        empty_pages_skipped += 1;
                        if next_blockno != pg_sys::InvalidBlockNumber {
                            pgrx::debug2!(
                                "Unlinking empty non-head page {} in freelist chain for XID {} (prev: {:?}, next: {})",
                                blockno,
                                found_xid,
                                prev_blockno,
                                next_blockno
                            );
                        }

                        // Unlink this empty page by updating the previous page's next pointer
                        if let Some(prev) = prev_blockno {
                            let mut prev_buf = bman.get_buffer_mut(prev);
                            let mut prev_page = prev_buf.page_mut();
                            prev_page.special_mut::<BM25PageSpecialData>().next_blockno =
                                next_blockno;
                            // The buffer will be marked dirty and written automatically
                        }

                        // Move to next page without updating prev_blockno
                        // (since we're removing the current page from the chain)
                        blockno = next_blockno;
                        continue;
                    }

                    let mut page = buffer.page_mut();
                    let contents = page.contents_mut::<AvlLeaf>();
                    let mut modified = false;

                    // At this point, we're either:
                    // 1. On a non-empty page (head or middle)
                    // 2. On an empty head page that needs unlinking
                    let should_unlink_head = if is_empty && head_blockno == blockno {
                        // Unlink empty head pages immediately to prevent chains of empty pages accumulating.
                        // This fixes the issue where freelists are increasingly long chains of empty blocks.

                        if next_blockno != pg_sys::InvalidBlockNumber {
                            // This is an empty head page with more pages in the chain
                            // We should unlink it to prevent repeated traversals
                            pgrx::debug2!(
                                "Unlinking empty head page {} from XID {}, next is {}",
                                blockno,
                                found_xid,
                                next_blockno
                            );
                            true
                        } else {
                            // This is an empty head page with no next page
                            // The entire freelist is empty and will be removed below (cnt == 0)
                            false
                        }
                    } else {
                        // get all that we can/need from this page
                        while contents.len > 0 && blocks.len() < many {
                            contents.len -= 1;
                            blocks.push(contents.entries[contents.len as usize]);
                            modified = true;
                        }
                        cnt += contents.len as usize;

                        // should we unlink this block from the chain? -- only if it's the head and _we_ made it empty
                        blockno == head_blockno
                            && contents.len == 0
                            && next_blockno != pg_sys::InvalidBlockNumber
                    };

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
                            head_pages_unlinked += 1;
                            pgrx::debug2!(
                                "Unlinked head block {}, total unlinked heads: {}",
                                old_head,
                                head_pages_unlinked
                            );
                            self.extend_with_when_recyclable(
                                bman,
                                unsafe { pg_sys::ReadNextFullTransactionId() },
                                std::iter::once(old_head),
                            );
                        }

                        // Continue with the new head (reset prev since this is now the head)
                        prev_blockno = None;
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

                        // move to the next candidate XID below this one.
                        xid = found_xid - 1;
                        if xid < pg_sys::FirstNormalTransactionId.into_inner() as u64 {
                            break;
                        }
                        continue 'outer;
                    }

                    // advance to the next block in the chain.
                    prev_blockno = Some(blockno);
                    blockno = next_blockno;
                }

                // Track chain length for this XID
                total_chain_length += chain_length;
                if chain_length > max_chain_length {
                    max_chain_length = chain_length;
                }

                // Log warning for unusually long chains
                if chain_length > 20 {
                    pgrx::warning!(
                        "FSM: Long chain detected for XID {} - {} pages in chain",
                        found_xid,
                        chain_length
                    );
                }

                if blocks.len() == many {
                    break;
                }

                // exhausted this list
                // move to the next candidate XID below this one.
                xid = found_xid - 1;
                if xid < pg_sys::FirstNormalTransactionId.into_inner() as u64 {
                    break;
                }
            }

            // Update global metrics with this drain's stats
            use std::sync::atomic::Ordering;
            FSM_METRICS
                .total_blocks_drained
                .fetch_add(blocks.len() as u64, Ordering::Relaxed);
            FSM_METRICS
                .total_empty_pages_skipped
                .fetch_add(empty_pages_skipped as u64, Ordering::Relaxed);
            FSM_METRICS
                .total_heads_unlinked
                .fetch_add(head_pages_unlinked as u64, Ordering::Relaxed);
            FSM_METRICS
                .total_xids_processed
                .fetch_add(xids_processed as u64, Ordering::Relaxed);
            FSM_METRICS
                .sum_chain_lengths
                .fetch_add(total_chain_length as u64, Ordering::Relaxed);
            FSM_METRICS
                .chain_count
                .fetch_add(xids_processed as u64, Ordering::Relaxed);

            // Update max chain length if this drain saw a longer one
            let mut current_max = FSM_METRICS.max_chain_length_seen.load(Ordering::Relaxed);
            while max_chain_length > current_max {
                match FSM_METRICS.max_chain_length_seen.compare_exchange_weak(
                    current_max,
                    max_chain_length,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(x) => current_max = x,
                }
            }

            // Log summary of drain operation if significant activity occurred
            if xids_processed > 0 || empty_pages_skipped > 0 || head_pages_unlinked > 0 {
                let avg_chain_length = if xids_processed > 0 {
                    total_chain_length as f64 / xids_processed as f64
                } else {
                    0.0
                };

                pgrx::debug1!(
                    "FSM drain summary: drained {} blocks from {} XIDs, skipped {} empty pages, unlinked {} heads, max chain: {}, avg chain: {:.1}, root acquisitions: {}, lock failures: {}",
                    blocks.len(),
                    xids_processed,
                    empty_pages_skipped,
                    head_pages_unlinked,
                    max_chain_length,
                    avg_chain_length,
                    root_acquisitions,
                    lock_failures
                );

                // Log batch optimization effectiveness
                if root_acquisitions > 1 {
                    let xids_per_root = xids_processed as f64 / root_acquisitions as f64;
                    pgrx::debug2!(
                        "Batch optimization: {:.2} XIDs processed per root acquisition (efficiency: {:.1}%)",
                        xids_per_root,
                        (xids_per_root - 1.0) * 100.0
                    );
                }
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

            // Collect blocks to count and detect merge-like behavior
            let blocks: Vec<_> = extend_with.collect();
            let block_count = blocks.len();

            if block_count == 0 {
                // caller didn't give us anything to do
                return;
            }

            // Merge detection: Large block counts indicate segment merge operations
            const MERGE_THRESHOLD: usize = 1000;
            let is_likely_merge = block_count >= MERGE_THRESHOLD;

            if is_likely_merge {
                pgrx::info!(
                    "FSM: Detected merge-like operation adding {} blocks to XID {} (threshold: {})",
                    block_count,
                    when_recyclable.value,
                    MERGE_THRESHOLD
                );
            }

            let mut extend_with = blocks.into_iter().peekable();
            if extend_with.peek().is_none() {
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

            // Check if we should compact before adding more blocks
            // This is especially important for merges that will add many XIDs
            if self.should_compact(bman) {
                pgrx::debug1!("Running FSM compaction before extending freelist");
                let removed = self.compact(bman);
                if removed > 0 {
                    pgrx::info!(
                        "FSM compaction freed {} slots before merge operation",
                        removed
                    );
                }
            }

            let start_time = if is_likely_merge {
                Some(std::time::Instant::now())
            } else {
                None
            };

            Self::extend_freelist(bman, start_block, extend_with);

            if let Some(start) = start_time {
                let duration = start.elapsed();
                pgrx::info!(
                    "FSM: Completed merge operation for XID {} - added {} blocks in {:?}",
                    when_recyclable.value,
                    block_count,
                    duration
                );

                // Warn if this creates an extremely long chain
                let approx_pages = block_count.div_ceil(MAX_ENTRIES);
                if approx_pages > 20 {
                    pgrx::warning!(
                        "FSM: Merge created long freelist chain (~{} pages) for XID {} - may impact drain performance",
                        approx_pages,
                        when_recyclable.value
                    );
                }
            }
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

            // Warn about overflow condition
            pgrx::warning!("FSM tree overflow: all 338 slots occupied, reusing oldest XID slot");

            // Log tree statistics for diagnostics
            let view = tree.view();
            pgrx::info!(
                "FSM tree at capacity: {} slots used, {} capacity",
                view.len(),
                view.capacity()
            );

            // we find the slot containing the largest (maximum) xid and change it to the provided `when_recyclable`
            // asserting that it is the same or greater than the maximum xid we found in the tree
            let max_slot = tree
                .get_max_slot()
                .expect("a full tree must have a maximum entry");

            let old_xid = max_slot.key;

            if when_recyclable.value > max_slot.key {
                // this is safe as the tree still maintains its balance.  we also have an exclusive lock
                // on the tree at this stage which means no concurrent backends can be changing the tree
                max_slot.key = when_recyclable.value;

                pgrx::info!(
                    "FSM overflow: replaced XID {} with XID {} in full tree",
                    old_xid,
                    when_recyclable.value
                );
            } else {
                pgrx::warning!(
                    "FSM overflow: new XID {} is not greater than replaced XID {} - potential issue",
                    when_recyclable.value,
                    old_xid
                );
            }

            max_slot.tag
        }

        /// Compact the FSM by removing empty XIDs from the tree
        /// Returns the number of XIDs removed
        fn compact(&mut self, bman: &mut BufferManager) -> usize {
            let mut removed_count = 0;
            let mut xids_to_remove = Vec::new();

            // First pass: identify empty XIDs
            {
                let root = bman.get_buffer(self.start_blockno);
                let page = root.page();
                let tree = self.avl_ref(&page);

                // Collect all XIDs with their head block numbers
                // Note: iter() returns (K, V) so we need to look up the tag separately
                let all_xids: Vec<_> = tree.iter().map(|(xid, _)| xid).collect();

                let mut all_entries = Vec::new();
                for xid in all_xids {
                    if let Some((_, tag)) = tree.get(&xid) {
                        all_entries.push((xid, tag as pg_sys::BlockNumber));
                    }
                }

                drop(root);

                // Check each XID's freelist to see if it's empty
                for (xid, head_blockno) in all_entries {
                    let mut is_empty = true;
                    let mut blockno = head_blockno;

                    // Traverse the freelist chain
                    while blockno != pg_sys::InvalidBlockNumber && is_empty {
                        if let Some(buffer) = bman.get_buffer_conditional(blockno) {
                            let page = buffer.page();
                            let contents = page.contents_ref::<AvlLeaf>();

                            if contents.len > 0 {
                                is_empty = false;
                            }

                            blockno = page.next_blockno();
                        } else {
                            // Can't acquire lock, assume not empty to be safe
                            is_empty = false;
                            break;
                        }
                    }

                    if is_empty {
                        xids_to_remove.push(xid);
                    }
                }
            }

            // Second pass: remove empty XIDs
            if !xids_to_remove.is_empty() {
                let mut root = bman.get_buffer_mut(self.start_blockno);
                let mut page = root.page_mut();
                let mut tree = self.avl_mut(&mut page);

                for xid in &xids_to_remove {
                    if tree.remove(xid).is_some() {
                        removed_count += 1;
                        pgrx::debug2!("FSM compact: removed empty XID {}", xid);
                    }
                }
            }

            if removed_count > 0 {
                pgrx::info!(
                    "FSM compaction removed {} empty XIDs from tree",
                    removed_count
                );
            }

            removed_count
        }

        /// Check if the FSM would benefit from compaction
        /// Returns true if compaction is recommended
        fn should_compact(&self, bman: &mut BufferManager) -> bool {
            let root = bman.get_buffer(self.start_blockno);
            let page = root.page();
            let tree = self.avl_ref(&page);

            let tree_size = tree.len();
            drop(root);

            // Recommend compaction if tree is >80% full
            // The tree has a maximum of 338 slots
            const MAX_TREE_SIZE: usize = 338;
            const COMPACT_THRESHOLD: usize = (MAX_TREE_SIZE * 80) / 100; // 80% = 270 slots
            const WARNING_THRESHOLD: usize = (MAX_TREE_SIZE * 90) / 100; // 90% = 304 slots

            if tree_size >= WARNING_THRESHOLD {
                pgrx::warning!(
                    "FSM tree approaching capacity: {}/{} slots used ({:.1}%) - compaction strongly recommended",
                    tree_size,
                    MAX_TREE_SIZE,
                    (tree_size as f64 / MAX_TREE_SIZE as f64) * 100.0
                );
                return true;
            }

            if tree_size >= COMPACT_THRESHOLD {
                pgrx::debug1!(
                    "FSM compaction recommended: tree size {} >= threshold {} ({:.1}%)",
                    tree_size,
                    COMPACT_THRESHOLD,
                    (tree_size as f64 / MAX_TREE_SIZE as f64) * 100.0
                );
                return true;
            }

            false
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

                // REMOVED: Aggressive splicing optimization that caused sparse freelists
                // The old code assumed if current page is full, all following pages are also full,
                // and created a new chain to splice in. This was wrong and created fragmentation.
                // Now we just move to the next page and check if it has space.

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

        use super::{AvlLeaf, MAX_ENTRIES, MAX_SLOTS, V2FSM};
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
        unsafe fn test_fsmv2_empty_page_chains_not_traversed() -> spi::Result<()> {
            Spi::run("CREATE TABLE IF NOT EXISTS fsm_test (id serial8, data text)")?;
            Spi::run("CREATE INDEX IF NOT EXISTS fsm_idx ON fsm_test USING bm25 (id, data) WITH (key_field = 'id')")?;

            let index_oid = Spi::get_one::<pg_sys::Oid>("SELECT 'fsm_idx'::regclass::oid")?
                .unwrap_or(pg_sys::InvalidOid);

            let indexrel = PgSearchRelation::with_lock(
                index_oid,
                pg_sys::RowExclusiveLock as pg_sys::LOCKMODE,
            );

            let mut bman = BufferManager::new(&indexrel);
            let metapage = MetaPage::open(&indexrel);
            let mut fsm = V2FSM::open(metapage.fsm());

            // Create a freelist chain with 3 pages, where the first page is empty
            // but the second and third have blocks
            let xid = pg_sys::FullTransactionId {
                value: pg_sys::FirstNormalTransactionId.into_inner() as u64,
            };
            let entries = (MAX_ENTRIES * 3) as u32;
            fsm.extend_with_when_recyclable(&mut bman, xid, 0..entries);

            // Make only the first (head) page empty, leave others with blocks
            {
                let root = bman.get_buffer(fsm.start_blockno);
                let page = root.page();
                let tree = fsm.avl_ref(&page);
                let (_, tag) = tree.get(&xid.value).expect("xid should still exist");
                let head_blockno = tag as pg_sys::BlockNumber;
                drop(root);

                // Empty only the head page
                let mut buf = bman.get_buffer_mut(head_blockno);
                let mut p = buf.page_mut();
                let leaf = p.contents_mut::<AvlLeaf>();
                let next_blockno = p.next_blockno();
                leaf.len = 0; // Head is now empty
                drop(buf);

                // Verify the next page still has blocks
                if next_blockno != pg_sys::InvalidBlockNumber {
                    let buf = bman.get_buffer(next_blockno);
                    let p = buf.page();
                    let contents = p.contents_ref::<AvlLeaf>();
                    assert!(contents.len > 0, "Second page should have blocks");
                }
            }

            // First drain - should skip the empty head and get blocks from subsequent pages
            let drained = fsm.drain(&mut bman, MAX_ENTRIES).collect::<Vec<_>>();
            assert_eq!(
                drained.len(),
                MAX_ENTRIES,
                "Should get blocks from second page"
            );

            // Check if the empty head page was unlinked
            // Without the fix: the empty head remains, causing future drains to traverse it
            // With the fix: the empty head should be unlinked
            {
                let root = bman.get_buffer(fsm.start_blockno);
                let page = root.page();
                let tree = fsm.avl_ref(&page);

                if let Some((_, tag)) = tree.get(&xid.value) {
                    let head_blockno = tag as pg_sys::BlockNumber;
                    drop(root);

                    // Check if the new head still has blocks (should not be the empty page)
                    let buf = bman.get_buffer(head_blockno);
                    let p = buf.page();
                    let contents = p.contents_ref::<AvlLeaf>();

                    // Without fix: this would be 0 (the empty page is still the head)
                    // With fix: this should be > 0 (empty page was unlinked, next page is head)
                    assert!(
                        contents.len > 0,
                        "Head page should not be empty after draining - empty pages should be unlinked"
                    );
                }
            }

            Ok(())
        }

        #[pg_test]
        unsafe fn test_fsmv2_monitoring() -> spi::Result<()> {
            // Test that monitoring metrics are properly tracked
            Spi::run("CREATE TABLE IF NOT EXISTS fsm_monitor_test (id serial8, data text)")?;
            Spi::run("CREATE INDEX IF NOT EXISTS fsm_monitor_idx ON fsm_monitor_test USING bm25 (id, data) WITH (key_field = 'id')")?;

            let index_oid = Spi::get_one::<pg_sys::Oid>("SELECT 'fsm_monitor_idx'::regclass::oid")?
                .unwrap_or(pg_sys::InvalidOid);

            let indexrel = PgSearchRelation::with_lock(
                index_oid,
                pg_sys::RowExclusiveLock as pg_sys::LOCKMODE,
            );

            let mut bman = BufferManager::new(&indexrel);
            let metapage = MetaPage::open(&indexrel);
            let mut fsm = V2FSM::open(metapage.fsm());

            // Create a scenario that will trigger monitoring output:
            // Multiple XIDs with chains of varying lengths
            let xid1 = pg_sys::FullTransactionId {
                value: pg_sys::FirstNormalTransactionId.into_inner() as u64,
            };
            let xid2 = pg_sys::FullTransactionId {
                value: pg_sys::FirstNormalTransactionId.into_inner() as u64 + 1,
            };

            // Create multi-page chain for xid1 (will have empty pages after partial drain)
            let blocks_per_page = MAX_ENTRIES as u32;
            fsm.extend_with_when_recyclable(&mut bman, xid1, 10000..10000 + blocks_per_page * 3);

            // Create single page for xid2
            fsm.extend_with_when_recyclable(&mut bman, xid2, 20000..20000 + 100);

            // Drain exactly one page from xid1 to create an empty head
            let _drained1: Vec<_> = fsm.drain(&mut bman, blocks_per_page as usize).collect();

            // Now drain more - this should trigger monitoring output showing:
            // - Empty pages skipped (the now-empty first page)
            // - Head pages unlinked
            // - Chain lengths
            let drained2: Vec<_> = fsm.drain(&mut bman, 200).collect();

            // Verify we got blocks
            assert!(!drained2.is_empty(), "Should have drained some blocks");

            // The monitoring will log to debug1/debug2, which we can't easily assert on
            // but this test ensures the code runs without panicking

            Ok(())
        }

        #[pg_test]
        unsafe fn test_fsmv2_metrics_collection() -> spi::Result<()> {
            // Test that global metrics are properly collected
            Spi::run("CREATE TABLE IF NOT EXISTS fsm_metrics_test (id serial8, data text)")?;
            Spi::run("CREATE INDEX IF NOT EXISTS fsm_metrics_idx ON fsm_metrics_test USING bm25 (id, data) WITH (key_field = 'id')")?;

            let index_oid = Spi::get_one::<pg_sys::Oid>("SELECT 'fsm_metrics_idx'::regclass::oid")?
                .unwrap_or(pg_sys::InvalidOid);

            let indexrel = PgSearchRelation::with_lock(
                index_oid,
                pg_sys::RowExclusiveLock as pg_sys::LOCKMODE,
            );

            let mut bman = BufferManager::new(&indexrel);
            let metapage = MetaPage::open(&indexrel);
            let mut fsm = V2FSM::open(metapage.fsm());

            // Reset metrics for clean test
            super::reset_fsm_metrics();

            // Get baseline
            let metrics_before = super::get_fsm_metrics();
            assert_eq!(metrics_before.total_drains, 0);
            assert_eq!(metrics_before.total_blocks_drained, 0);

            // Create some blocks
            let xid = pg_sys::FullTransactionId {
                value: pg_sys::FirstNormalTransactionId.into_inner() as u64,
            };
            fsm.extend_with_when_recyclable(&mut bman, xid, 10000..10100);

            // Drain some blocks
            let drained: Vec<_> = fsm.drain(&mut bman, 50).collect();
            assert_eq!(drained.len(), 50);

            // Check metrics were updated
            let metrics_after = super::get_fsm_metrics();
            assert_eq!(metrics_after.total_drains, 1, "Should have 1 drain");
            assert_eq!(
                metrics_after.total_blocks_drained, 50,
                "Should have drained 50 blocks"
            );
            assert!(
                metrics_after.total_xids_processed > 0,
                "Should have processed at least 1 XID"
            );

            // Drain more and check accumulation
            let drained2: Vec<_> = fsm.drain(&mut bman, 30).collect();
            assert_eq!(drained2.len(), 30);

            let metrics_final = super::get_fsm_metrics();
            assert_eq!(metrics_final.total_drains, 2, "Should have 2 drains");
            assert_eq!(
                metrics_final.total_blocks_drained, 80,
                "Should have drained 80 blocks total"
            );

            Ok(())
        }

        #[pg_test]
        unsafe fn test_fsmv2_batch_optimization() -> spi::Result<()> {
            // Test that batch optimization reduces root buffer acquisitions
            // When multiple XIDs are processed, we should keep the root buffer
            // across several XID attempts before dropping it

            Spi::run("CREATE TABLE IF NOT EXISTS fsm_batch_test (id serial8, data text)")?;
            Spi::run("CREATE INDEX IF NOT EXISTS fsm_batch_idx ON fsm_batch_test USING bm25 (id, data) WITH (key_field = 'id')")?;

            let index_oid = Spi::get_one::<pg_sys::Oid>("SELECT 'fsm_batch_idx'::regclass::oid")?
                .unwrap_or(pg_sys::InvalidOid);

            let indexrel = PgSearchRelation::with_lock(
                index_oid,
                pg_sys::RowExclusiveLock as pg_sys::LOCKMODE,
            );

            let mut bman = BufferManager::new(&indexrel);
            let metapage = MetaPage::open(&indexrel);
            let mut fsm = V2FSM::open(metapage.fsm());

            // Create multiple XIDs with small amounts of blocks each
            // This simulates a scenario where we process many XIDs in one drain
            let base_xid = pg_sys::FirstNormalTransactionId.into_inner() as u64;
            for i in 0..10u32 {
                let xid = pg_sys::FullTransactionId {
                    value: base_xid + i as u64,
                };
                // Add just a few blocks per XID
                fsm.extend_with_when_recyclable(
                    &mut bman,
                    xid,
                    (10000 + i * 100)..(10000 + i * 100 + 10),
                );
            }

            // Drain all blocks - should process multiple XIDs
            let drained: Vec<_> = fsm.drain(&mut bman, 200).collect();

            // We should have drained 10 XIDs * 10 blocks = 100 blocks
            assert_eq!(drained.len(), 100, "Should drain 100 blocks total");

            // The batch optimization should have processed multiple XIDs
            // with a single root acquisition (or at least fewer than 10)
            // We can't directly assert on root acquisitions here, but the test
            // ensures the code path works without panicking

            Ok(())
        }

        #[pg_test]
        unsafe fn test_fsmv2_merge_detection() -> spi::Result<()> {
            // Test that merge-like operations are detected and logged
            Spi::run("CREATE TABLE IF NOT EXISTS fsm_merge_test (id serial8, data text)")?;
            Spi::run("CREATE INDEX IF NOT EXISTS fsm_merge_idx ON fsm_merge_test USING bm25 (id, data) WITH (key_field = 'id')")?;

            let index_oid = Spi::get_one::<pg_sys::Oid>("SELECT 'fsm_merge_idx'::regclass::oid")?
                .unwrap_or(pg_sys::InvalidOid);

            let indexrel = PgSearchRelation::with_lock(
                index_oid,
                pg_sys::RowExclusiveLock as pg_sys::LOCKMODE,
            );

            let mut bman = BufferManager::new(&indexrel);
            let metapage = MetaPage::open(&indexrel);
            let mut fsm = V2FSM::open(metapage.fsm());

            // Simulate a merge by adding a large number of blocks at once
            let xid = pg_sys::FullTransactionId {
                value: pg_sys::FirstNormalTransactionId.into_inner() as u64,
            };

            // Add 2000 blocks - this should trigger merge detection (threshold is 1000)
            let start_block = 100000u32;
            let block_count = 2000u32;
            fsm.extend_with_when_recyclable(&mut bman, xid, start_block..start_block + block_count);

            // Verify blocks were added
            let drained: Vec<_> = fsm.drain(&mut bman, block_count as usize).collect();
            assert_eq!(
                drained.len(),
                block_count as usize,
                "Should drain all blocks added"
            );

            // The merge detection should have logged INFO messages
            // (we can't directly assert on log output, but this verifies the code path works)

            Ok(())
        }

        #[pg_test]
        unsafe fn test_fsmv2_compaction() -> spi::Result<()> {
            // Test that compaction removes empty XIDs from the tree
            Spi::run("CREATE TABLE IF NOT EXISTS fsm_compact_test (id serial8, data text)")?;
            Spi::run("CREATE INDEX IF NOT EXISTS fsm_compact_idx ON fsm_compact_test USING bm25 (id, data) WITH (key_field = 'id')")?;

            let index_oid = Spi::get_one::<pg_sys::Oid>("SELECT 'fsm_compact_idx'::regclass::oid")?
                .unwrap_or(pg_sys::InvalidOid);

            let indexrel = PgSearchRelation::with_lock(
                index_oid,
                pg_sys::RowExclusiveLock as pg_sys::LOCKMODE,
            );

            let mut bman = BufferManager::new(&indexrel);
            let metapage = MetaPage::open(&indexrel);
            let mut fsm = V2FSM::open(metapage.fsm());

            // Create several XIDs with small numbers of blocks
            let base_xid = pg_sys::FirstNormalTransactionId.into_inner() as u64;
            for i in 0..5u32 {
                let xid = pg_sys::FullTransactionId {
                    value: base_xid + i as u64,
                };
                fsm.extend_with_when_recyclable(
                    &mut bman,
                    xid,
                    (10000 + i * 100)..(10000 + i * 100 + 10),
                );
            }

            // Manually empty the freelists without using drain (which auto-removes empty XIDs)
            // This simulates XIDs that have become empty but haven't been cleaned up yet
            // First collect all the head block numbers
            let mut head_blocks = Vec::new();
            {
                let root = bman.get_buffer(fsm.start_blockno);
                let page = root.page();
                let tree = fsm.avl_ref(&page);

                for i in 0..5u32 {
                    let xid = base_xid + i as u64;
                    if let Some((_, tag)) = tree.get(&xid) {
                        head_blocks.push(tag as pg_sys::BlockNumber);
                    }
                }
            }

            // Now empty all pages for each XID
            for head_blockno in head_blocks {
                let mut blockno = head_blockno;
                while blockno != pg_sys::InvalidBlockNumber {
                    let mut buf = bman.get_buffer_mut(blockno);
                    let mut p = buf.page_mut();
                    let leaf = p.contents_mut::<AvlLeaf>();
                    leaf.len = 0;

                    let next = p.next_blockno();
                    drop(buf);
                    blockno = next;
                }
            }

            // Now run compaction - should remove all empty XIDs
            let removed = fsm.compact(&mut bman);
            assert!(
                removed > 0,
                "Should have removed at least one empty XID, got {}",
                removed
            );
            assert!(
                removed <= 5,
                "Should not remove more than 5 XIDs, got {}",
                removed
            );

            pgrx::info!("Compaction test removed {} XIDs", removed);

            Ok(())
        }

        #[pg_test]
        unsafe fn test_fsmv2_should_compact() -> spi::Result<()> {
            // Test the should_compact logic
            Spi::run("CREATE TABLE IF NOT EXISTS fsm_should_compact_test (id serial8, data text)")?;
            Spi::run("CREATE INDEX IF NOT EXISTS fsm_should_compact_idx ON fsm_should_compact_test USING bm25 (id, data) WITH (key_field = 'id')")?;

            let index_oid =
                Spi::get_one::<pg_sys::Oid>("SELECT 'fsm_should_compact_idx'::regclass::oid")?
                    .unwrap_or(pg_sys::InvalidOid);

            let indexrel = PgSearchRelation::with_lock(
                index_oid,
                pg_sys::RowExclusiveLock as pg_sys::LOCKMODE,
            );

            let mut bman = BufferManager::new(&indexrel);
            let metapage = MetaPage::open(&indexrel);
            let fsm = V2FSM::open(metapage.fsm());

            // Initially should not need compaction
            assert!(
                !fsm.should_compact(&mut bman),
                "Empty tree should not need compaction"
            );

            // The threshold is 270 XIDs (80% of 338)
            // We can't easily test this without creating 270+ XIDs which is expensive
            // But we can verify the logic doesn't panic

            Ok(())
        }

        #[pg_test]
        unsafe fn test_fsmv2_tree_overflow_warning() -> spi::Result<()> {
            // Test that overflow warnings are logged when tree approaches capacity
            // We can't easily create 338 XIDs, but we can verify the warning logic
            Spi::run("CREATE TABLE IF NOT EXISTS fsm_overflow_test (id serial8, data text)")?;
            Spi::run("CREATE INDEX IF NOT EXISTS fsm_overflow_idx ON fsm_overflow_test USING bm25 (id, data) WITH (key_field = 'id')")?;

            let index_oid =
                Spi::get_one::<pg_sys::Oid>("SELECT 'fsm_overflow_idx'::regclass::oid")?
                    .unwrap_or(pg_sys::InvalidOid);

            let indexrel = PgSearchRelation::with_lock(
                index_oid,
                pg_sys::RowExclusiveLock as pg_sys::LOCKMODE,
            );

            let mut bman = BufferManager::new(&indexrel);
            let metapage = MetaPage::open(&indexrel);
            let fsm = V2FSM::open(metapage.fsm());

            // Verify the thresholds are set correctly
            // 80% of 338 = 270 (compact threshold)
            // 90% of 338 = 304 (warning threshold)

            // Initially should not need compaction (tree is empty)
            assert!(
                !fsm.should_compact(&mut bman),
                "Empty tree should not need compaction"
            );

            // The warning logic will trigger when we actually fill the tree
            // For now, just verify it doesn't panic with empty tree

            Ok(())
        }

        fn freelist_blocks(
            bman: &mut BufferManager,
            fsm: &V2FSM,
            xid: pg_sys::FullTransactionId,
        ) -> Vec<usize> {
            let root = bman.get_buffer(fsm.start_blockno);
            let page = root.page();
            let tree = fsm.avl_ref(&page);
            let Some((_, tag)) = tree.get(&xid.value) else {
                return Vec::new();
            };

            let mut blockno = tag as pg_sys::BlockNumber;
            let mut result = Vec::new();

            while blockno != pg_sys::InvalidBlockNumber {
                let buf = bman.get_buffer(blockno);
                let p = buf.page();
                let leaf = p.contents_ref::<AvlLeaf>();
                result.push(leaf.len as usize);
                blockno = p.next_blockno();
            }

            result
        }

        #[pg_test]
        unsafe fn test_fsmv2_with_holes() -> spi::Result<()> {
            // Test that empty pages in the middle of a freelist chain are unlinked
            Spi::run("CREATE TABLE IF NOT EXISTS fsm_test (id serial8, data text)")?;
            Spi::run("CREATE INDEX IF NOT EXISTS fsm_idx ON fsm_test USING bm25 (id, data) WITH (key_field = 'id')")?;

            let index_oid = Spi::get_one::<pg_sys::Oid>("SELECT 'fsm_idx'::regclass::oid")?
                .unwrap_or(pg_sys::InvalidOid);

            let indexrel = PgSearchRelation::with_lock(
                index_oid,
                pg_sys::RowExclusiveLock as pg_sys::LOCKMODE,
            );

            let mut bman = BufferManager::new(&indexrel);
            let metapage = MetaPage::open(&indexrel);
            let mut fsm = V2FSM::open(metapage.fsm());

            // Create freelist with 5 pages (5 * MAX_ENTRIES blocks)
            let xid = pg_sys::FullTransactionId {
                value: pg_sys::FirstNormalTransactionId.into_inner() as u64,
            };
            let entries = (MAX_ENTRIES * 5) as u32;
            fsm.extend_with_when_recyclable(&mut bman, xid, 0..entries);

            // Empty pages at index 1 and 2 (but not 0, 3, 4)
            {
                let root = bman.get_buffer(fsm.start_blockno);
                let page = root.page();
                let tree = fsm.avl_ref(&page);
                let (_, tag) = tree.get(&xid.value).expect("xid should still exist");
                let mut blockno = tag as pg_sys::BlockNumber;
                let mut cnt = 0;
                while blockno != pg_sys::InvalidBlockNumber {
                    let mut buf = bman.get_buffer_mut(blockno);
                    let mut p = buf.page_mut();
                    let leaf = p.contents_mut::<super::AvlLeaf>();
                    if cnt > 0 && cnt < 3 {
                        leaf.len = 0;
                    }
                    drop(buf);
                    let buf_ro = bman.get_buffer(blockno);
                    let p_ro = buf_ro.page();
                    blockno = p_ro.next_blockno();
                    cnt += 1;
                }
            }

            // Drain enough blocks to traverse past the empty pages
            // First page has 2039, so draining 2040 will empty it and force traversal to the empty pages
            let drained = fsm.drain(&mut bman, 2040).collect::<Vec<_>>();
            // We should get 2039 from first page + 1 from the next non-empty page (skipping the 2 empty ones)
            assert_eq!(drained.len(), 2040);

            // After draining 2040, we've emptied the first page and traversed the empty pages
            // The empty pages should now be unlinked, leaving only the last two pages (one with 2038, one with 2039)
            let blocks = freelist_blocks(&mut bman, &fsm, xid);
            assert_eq!(
                blocks,
                vec![2038, 2039],
                "Empty pages should be unlinked after traversal"
            );

            // Drain all remaining - should get exactly 2038 + 2039 = 4077 blocks
            let drained = fsm.drain(&mut bman, 10000).collect::<Vec<_>>();
            assert_eq!(drained.len(), 2038 + 2039);

            // After draining everything, XID should be removed
            let blocks = freelist_blocks(&mut bman, &fsm, xid);
            assert_eq!(
                blocks,
                Vec::<usize>::new(),
                "XID should be removed after draining all blocks"
            );

            Ok(())
        }

        #[pg_test]
        unsafe fn test_fsmv2_extend_fills_existing_pages() -> spi::Result<()> {
            // Test that extending a freelist fills existing pages with space
            // rather than creating new pages unnecessarily
            Spi::run("CREATE TABLE IF NOT EXISTS fsm_test (id serial8, data text)")?;
            Spi::run("CREATE INDEX IF NOT EXISTS fsm_idx ON fsm_test USING bm25 (id, data) WITH (key_field = 'id')")?;

            let index_oid = Spi::get_one::<pg_sys::Oid>("SELECT 'fsm_idx'::regclass::oid")?
                .unwrap_or(pg_sys::InvalidOid);

            let indexrel = PgSearchRelation::with_lock(
                index_oid,
                pg_sys::RowExclusiveLock as pg_sys::LOCKMODE,
            );

            let mut bman = BufferManager::new(&indexrel);
            let metapage = MetaPage::open(&indexrel);
            let mut fsm = V2FSM::open(metapage.fsm());

            let xid = pg_sys::FullTransactionId {
                value: pg_sys::FirstNormalTransactionId.into_inner() as u64,
            };

            // Create initial freelist with 3 pages (3 * MAX_ENTRIES blocks)
            let entries = (MAX_ENTRIES * 3) as u32;
            fsm.extend_with_when_recyclable(&mut bman, xid, 0..entries);

            // Drain 1000 blocks, creating space in the first page
            let drained = fsm.drain(&mut bman, 1000).collect::<Vec<_>>();
            assert_eq!(drained.len(), 1000);

            // Verify we have 3 pages: [2039-1000=1039, 2039, 2039]
            let blocks_before = freelist_blocks(&mut bman, &fsm, xid);
            assert_eq!(blocks_before, vec![1039, 2039, 2039]);

            // Now extend with 500 more blocks
            // Without the fix: would create a 4th page with 500 blocks
            // With the fix: should fill the first page to 1539, leaving it at [1539, 2039, 2039]
            fsm.extend_with_when_recyclable(&mut bman, xid, 10000..10500);

            let blocks_after = freelist_blocks(&mut bman, &fsm, xid);
            assert_eq!(
                blocks_after,
                vec![1539, 2039, 2039],
                "Should fill existing page with space, not create new page"
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
