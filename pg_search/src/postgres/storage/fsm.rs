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
use crate::postgres::storage::buffer::{init_new_buffer, BufferManager, BufferMut};
use crate::postgres::storage::metadata::MetaPage;
use pgrx::iter::TableIterator;
use pgrx::{name, pg_extern, pg_sys, AnyNumeric, PgRelation};


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
    v1_uncompressed = 1,

    /// This represents the current FSM format and is the default for new FSM pages
    v2_root = 2,
    v2_ent = 3,
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct RootHeader {
    kind:   FSMBlockKind,
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct ChainHeader {
    kind:   FSMBlockKind,
    maxtx:  pg_sys::TransactionId,
    count:   i32,
}

const ROOT_SPACE: usize = bm25_max_free_space() - size_of::<RootHeader>();
const CHAIN_SPACE: usize = bm25_max_free_space() - size_of::<ChainHeader>();
const NENT: usize = CHAIN_SPACE / size_of::<pg_sys::BlockNumber>();
const NLIST: usize = 32;


#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct FSMChain {
    kind:    FSMBlockKind,
    xid:     u32,
    count:   i32,
    entries: [pg_sys::BlockNumber; NENT],
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct FSMRoot {
    kind:       FSMBlockKind,
    min_xid:    u32,
    partial:    [pg_sys::BlockNumber; NLIST],
    filled:     [pg_sys::BlockNumber; NLIST],
}

#[derive(Debug)]
pub struct FreeSpaceManager {
    root: pg_sys::BlockNumber,
}

impl FreeSpaceManager {
    /// Create a new [`FreeSpaceManager`] in the block storage of the specified `indexrel`.
    pub unsafe fn create(indexrel: &PgSearchRelation) -> pg_sys::BlockNumber {
        let mut buf = init_new_buffer(indexrel);
        let mut page = buf.page_mut();
        *page.contents_mut::<FSMRoot>() = FSMRoot{
            kind    : FSMBlockKind::v2_root,
            min_xid : pg_sys::FirstNormalTransactionId.into_inner(),
            partial : [pg_sys::InvalidBlockNumber; 32],
            filled  : [pg_sys::InvalidBlockNumber; 32],
        };
        buf.number()
    }

    /// Open an existing [`FreeSpaceManager`] which is rooted at the specified blocks
    pub fn open(root: pg_sys::BlockNumber) -> Self {
        Self { root: root, }
    }

    pub fn pop(&mut self, bman: &mut BufferManager) -> Option<pg_sys::BlockNumber> {
        self.drain(bman, 1).next()
    }

    pub fn drain(
        &mut self,
        bman: &mut BufferManager,
        n: usize,
    ) -> impl Iterator<Item = pg_sys::BlockNumber> {
        FSMDrainIter::new(self, bman).take(n)
    }

    pub fn extend(
        &self,
        bman: &mut BufferManager,
        extend_with: impl Iterator<Item = pg_sys::BlockNumber>,
    ) {
         let xid = unsafe {
            pg_sys::GetCurrentTransactionIdIfAny().max(pg_sys::FirstNormalTransactionId)
        };
        self.extend_with_when_recyclable(bman, xid, extend_with)
    }

    pub fn extend_with_when_recyclable(
        &self,
        bman: &mut BufferManager,
        xid: pg_sys::TransactionId,
        mut extend_with: impl Iterator<Item = pg_sys::BlockNumber>,
    ) {
        let slot = (xid.into_inner() as usize) % NLIST;
        let mut rbuf = load_root(self.root, bman);
        let root = rbuf.page_mut().contents_mut::<FSMRoot>();
        
        loop {
            let list = root.partial[slot];
            let mut buf = get_chain(bman, list, xid).unwrap_or(new_chain(bman));
            let b = buf.page_mut().contents_mut::<FSMChain>();
            if b.xid != xid.into_inner() {
                continue;
            }
            match extend_with.next() {
                None => {
                    return;
                }
                Some(bno) => {
                    b.entries[b.count as usize] = bno;
                    b.count += 1;
                    if b.count as usize == NENT {
                        move_block(self.root, bman, buf, xid, true);
                        continue;
                    }
                }
            }
        }
    }
}

/// Draining iterator over FSM entries. As entries are yielded, they are
/// removed from the FSM (on-disk)
struct FSMDrainIter {
    bman    : BufferManager,
    root    : pg_sys::BlockNumber,
    horizon : pg_sys::TransactionId,
}

impl FSMDrainIter {
    fn new(fsm: &FreeSpaceManager, bman: &BufferManager) -> Self {
        let horizon =
            unsafe { pg_sys::GetCurrentTransactionIdIfAny().max(pg_sys::FirstNormalTransactionId) };
        Self {
            bman    : bman.clone(),
            root    : fsm.root,
            horizon : horizon,
        }
    }
}

impl Iterator for FSMDrainIter {
    type Item = pg_sys::BlockNumber;

    fn next(&mut self) -> Option<Self::Item> {
        let mut rbuf = load_root(self.root, &mut self.bman);
        let root = rbuf.page_mut().contents_mut::<FSMRoot>();
        loop {
            let slot = (root.min_xid as usize) % NLIST;
            let min_xid = pg_sys::TransactionId::from(root.min_xid);
            if !quiesced(min_xid, self.horizon) {
                break;
            }
            if let Some(mut buf) = get_chain(&mut self.bman, root.partial[slot], min_xid) {
                let b = buf.page_mut().contents_mut::<FSMChain>();
                let ret = b.entries[(b.count-1) as usize];
                b.count -= 1;
                if b.count == 0 {
                    // TODO: free block
                    root.partial[slot] = next(&buf);
                }
                return Some(ret)
            }
            if let Some(mut buf) = get_chain(&mut self.bman, root.filled[slot], min_xid) {
                let b = buf.page_mut().contents_mut::<FSMChain>();
                let ret = b.entries[(b.count-1) as usize];
                b.count -= 1;
                move_block(self.root, &mut self.bman, buf, min_xid, false);
                return Some(ret)
            } 
            root.min_xid += 1;
        }
        return None;
    }
}

#[inline(always)]
fn quiesced(
    fsm_xid: pg_sys::TransactionId,
    xid_horizon: pg_sys::TransactionId,
) -> bool {
    crate::postgres::utils::TransactionIdPrecedesOrEquals(fsm_xid, xid_horizon)
}

// Gets a block containing entries for the current transaction number,
fn get_chain(
    bman: &mut BufferManager,
    list : pg_sys::BlockNumber,
    xid : pg_sys::TransactionId
) -> Option<BufferMut> {
    // technically, a list scan, but we shard things so that we should
    // rarely have overlap in transaction ids, usually the first block
    // we look at should match
    let mut bno = list;
    while bno != pg_sys::InvalidBlockNumber {
        let buf = bman.get_buffer_mut(bno);
        let b = buf.page().contents::<FSMChain>();
        if b.xid == xid.into_inner() {
            return Some(buf);
        }
        bno = next(&buf);
    }
    None
}

fn move_block(
    root: pg_sys::BlockNumber,
    bman: &mut BufferManager,
    mv : BufferMut,
    xid : pg_sys::TransactionId,
    partial_to_filled : bool
) {
    // list block may have been modified, so we need to walk it again.
    let mut buf = load_root(root, bman);
    let slot = (xid.into_inner() as usize) % NLIST;
    let root = buf.page_mut().contents_mut::<FSMRoot>();
    let lp = if partial_to_filled { &root.partial[slot] } else { &root.filled[slot] };
    if *lp == mv.number() {
        root.partial[slot] = next(&mv);
        // TODO: Implement set_next and filled list management
        return;
    }
    
    panic!("partial block not on partial list");
}

fn load_root(root : pg_sys::BlockNumber, bman : &mut BufferManager) -> BufferMut {
    let mut buf = bman.get_buffer_mut(root);
    let mut pg = buf.page_mut();
    let r = pg.contents_mut::<FSMRoot>();
    if let FSMBlockKind::v2_root = r.kind {
        // already correct format
    } else {
        *r = FSMRoot{
            kind    : FSMBlockKind::v2_root,
            min_xid : pg_sys::FirstNormalTransactionId.into_inner(),
            partial : [pg_sys::InvalidBlockNumber; 32],
            filled  : [pg_sys::InvalidBlockNumber; 32],
        };
    }
    buf
}

fn new_chain(bman : &mut BufferManager) -> BufferMut {
    let mut buf = init_new_buffer(bman.buffer_access().rel());
    let mut pg = buf.page_mut();
    let chain = pg.contents_mut::<FSMChain>();
    *chain = FSMChain { 
        kind: FSMBlockKind::v2_ent,
        xid: unsafe {
            pg_sys::GetCurrentTransactionIdIfAny().max(pg_sys::FirstNormalTransactionId).into_inner()
        },
        count: 0,
        entries: [pg_sys::InvalidBlockNumber; NENT]
    };
    buf
}

fn next(b : &BufferMut) -> pg_sys::BlockNumber {
    b.page().special::<BM25PageSpecialData>().next_blockno
}
