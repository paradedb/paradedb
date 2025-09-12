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
use std::option::Option;
use pgrx::iter::TableIterator;
use pgrx::{name, pg_extern, pg_sys, AnyNumeric, PgRelation};


/// Denotes what the data on an FSM block looks like
#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(u32)]
enum FSMBlockKind {
    /// This variant represents the original FSM format in pg_search versions 0.17.0 through 0.17.3
    /// It is not meant to be used for making new pages, only for detecting old pages so they can
    /// be converted
    #[doc(hidden)]
    #[allow(dead_code)]
    v0 = 0,
    #[allow(dead_code)]
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
pub struct FSMRoot {
    kind:       FSMBlockKind,
    version:    u64,
    pub partial:    [pg_sys::BlockNumber; NLIST],
    pub filled:     [pg_sys::BlockNumber; NLIST],
}

#[derive(Debug)]
pub struct FreeSpaceManager {
    last_slot : usize,
    root: pg_sys::BlockNumber,
}

impl FreeSpaceManager {
    /// Create a new [`FreeSpaceManager`] in the block storage of the specified `indexrel`.
    pub unsafe fn create(indexrel: &PgSearchRelation) -> pg_sys::BlockNumber {
        let mut buf = init_new_buffer(indexrel);
        let mut page = buf.page_mut();
        *page.contents_mut::<FSMRoot>() = FSMRoot{
            kind    : FSMBlockKind::v2_root,
            version : 0,
            partial : [pg_sys::InvalidBlockNumber; 32],
            filled  : [pg_sys::InvalidBlockNumber; 32],
        };
        buf.number()
    }

    /// Open an existing [`FreeSpaceManager`] which is rooted at the specified blocks
    pub fn open(root: pg_sys::BlockNumber) -> Self {
        Self { last_slot: 0, root: root, }
    }

    pub fn pop(&mut self, bman: &mut BufferManager) -> Option<pg_sys::BlockNumber> {
        self.drain(bman, 1).next()
    }

    pub fn drain(
        &mut self,
        bman: &mut BufferManager,
        n: usize,
    ) -> impl Iterator<Item = pg_sys::BlockNumber> {
        let horizon =
            unsafe { pg_sys::GetCurrentTransactionIdIfAny().max(pg_sys::FirstNormalTransactionId)
        };
        self.drain_at(bman, horizon, n)
    }

    pub fn drain_at(
        &mut self,
        bman: &mut BufferManager,
        horizon : pg_sys::TransactionId,
        n: usize,
    ) -> impl Iterator<Item = pg_sys::BlockNumber> {
        let mut v : Vec<pg_sys::BlockNumber> = Vec::new();
        while v.len() < n {
            if !self.drain1(bman, horizon, n, &mut v) {
                break;
            }
        }
        v.into_iter()
    }

    pub fn drain1(
        &mut self,
        bman : &mut BufferManager,
        horizon : pg_sys::TransactionId,
        limit : usize, 
        v : &mut Vec<pg_sys::BlockNumber>,
    ) -> bool {

        let rbuf = bman.get_buffer(self.root);
        let pg = rbuf.page();
        let root = pg.contents::<FSMRoot>();
        let mut n = limit as i32;
        for i in 0..NLIST {
            let slot = (self.last_slot + i) % NLIST;
            if let Some(mut buf) = get_chain(bman, root.partial[slot], horizon, false) {
                let mut pg = buf.page_mut();
                let b = pg.contents_mut::<FSMChain>();
                self.last_slot = slot;
                if n >= b.count {
//                    drop(buf);
//                    drop(rbuf);
//                    self.fsm.extend(root.partials[slot]);
                    let vers = root.version;
                    let mut mbuf = rbuf.upgrade(bman);
                    let mroot = mbuf.page_mut().contents_mut::<FSMRoot>();
                    if mroot.version != vers {
                        return true;
                    }
                    n = b.count;
                    mroot.version += 1;
                    mroot.partial[slot] = pg.special_mut::<BM25PageSpecialData>().next_blockno;
                }
                for i in 1..n+1 {
                    v.push(b.entries[(b.count-i) as usize]);
                }
                b.count -= n;
                return true;
            }
            if let Some(mut buf) = get_chain(bman, root.filled[slot], horizon, false) {
               let b = buf.page_mut().contents_mut::<FSMChain>();

                let vers = root.version;
                let mut mbuf = rbuf.upgrade(bman);
                let mroot = mbuf.page_mut().contents_mut::<FSMRoot>();
                if mroot.version != vers {
                    return true;
                }
                if n > b.count {
                    n = b.count;
                }
                for i in 1..n+1 {
                    v.push(b.entries[(b.count-i) as usize]);
                }
                b.count -= n;
                move_block(&mut mroot.filled[slot], &mut mroot.partial[slot], bman, &mut buf);
                mroot.version += 1;
                self.last_slot = slot;
                return true;
            } 
        }
        false
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
        extend_with: impl Iterator<Item = pg_sys::BlockNumber>,
    ) {
        let slot = (xid.into_inner() as usize) % NLIST;
        let mut rbuf = load_root(self.root, bman);
        let mut iter = extend_with.peekable();
        let mut list = {
            let root = rbuf.page().contents::<FSMRoot>();
            root.partial[slot]
        };
        'l: loop {
            if iter.peek().is_none() {
                break;
            }
            let mut buf = {
                let root = rbuf.page_mut().contents_mut::<FSMRoot>();
                next_chain(bman, &mut root.partial[slot], list, xid)
            };
let num = buf.number();
            let b = buf.page_mut().contents_mut::<FSMChain>();
            if b.xid != xid.into_inner() {
                list = next(&buf);
                continue;
            }
            loop {
                match iter.next() {
                    None => {
                        return;
                    }
                    Some(bno) => {
let cnt = b.count;
pgrx::warning!("extending {}[{}] = {}", num, cnt, bno);
                        b.entries[b.count as usize] = bno;
                        b.count += 1;
                        if b.count as usize == NENT {
                            let root = rbuf.page_mut().contents_mut::<FSMRoot>();
                            root.version += 1;
                            move_block(&mut root.partial[slot], &mut root.filled[slot], bman, &mut buf);
                            list =  root.partial[slot];
                            continue 'l;
                        }
                    }
                }
            }
        }
    }
}


#[pg_extern]
unsafe fn fsm_info(
    index: PgRelation,
) -> TableIterator<
    'static,
    (
        name!(fsm_blockno, AnyNumeric),
        name!(free_blockno, AnyNumeric),
        name!(freed_xid, AnyNumeric),
    ),
> {
    let index = PgSearchRelation::from_pg(index.as_ptr());

    let meta = MetaPage::open(&index);
    let fsm = meta.fsm();
    let mut bman = BufferManager::new(&index);
    let mut rbuf = load_root(fsm, &mut bman);
    let root = rbuf.page_mut().contents_mut::<FSMRoot>();
    let mut mapping = Vec::<(
        u32,
        u32,
        u32,
    )>::default();
    let xid = pg_sys::TransactionId::from((i32::MAX-1) as u32);

    // TODO: remove these
    for i in 0..NLIST {
        if root.partial[i] == pg_sys::InvalidBlockNumber
        && root.filled[i] == pg_sys::InvalidBlockNumber {
            continue;
        }
        let mut b = root.partial[i];
        loop {
            match get_chain(&mut bman, b, xid, true){
                Some(buf) => {
                    let node = buf.page().contents::<FSMChain>();
                    for i in 0..node.count {
                        mapping.push((b, node.entries[i as usize], node.xid))
                    }
                    b = next(&buf);
                }
                None	=> {
                    break;
                }
            }
        }
        b = root.filled[i];
        loop {
            match get_chain(&mut bman, b, xid, true){
                Some(buf) => {
                    let node = buf.page().contents::<FSMChain>();
                    for i in 0..node.count {
                        mapping.push((b, node.entries[i as usize], node.xid))
                    }
                    b = next(&buf);
                }
                None	=> {
                    break;
                }
            }
        }
    }

    TableIterator::new(mapping.into_iter().map(|(a, b, c)| (a.into(), b.into(), c.into())))
}

fn next_chain(
    bman: &mut BufferManager,
    pslot : &mut pg_sys::BlockNumber,
    list : pg_sys::BlockNumber,
    xid : pg_sys::TransactionId
) -> BufferMut {
    // technically, a list scan, but we shard things so that we should
    // rarely have overlap in transaction ids, usually the first block
    // we look at should match
    let mut bno = list;
    while bno != pg_sys::InvalidBlockNumber {
        let buf = bman.get_buffer_mut(bno);
        let b = buf.page().contents::<FSMChain>();
        if b.xid == xid.into_inner() {
            return buf;
        }
        bno = next(&buf);
    }
    let mut buf = new_chain(bman, xid);
    buf.page_mut().special_mut::<BM25PageSpecialData>().next_blockno = list;
    *pslot = buf.number();
    buf
}

// Gets a block containing entries for the current transaction number,
fn get_chain(
    bman: &mut BufferManager,
    list : pg_sys::BlockNumber,
    xid : pg_sys::TransactionId,
    empty_ok : bool,
) -> Option<BufferMut> {
    // technically, a list scan, but we shard things so that we should
    // rarely have overlap in transaction ids, usually the first block
    // we look at should match
    let mut bno = list;
    while bno != pg_sys::InvalidBlockNumber {
        let buf = bman.get_buffer_mut(bno);
        let b = buf.page().contents_ref::<FSMChain>();
        let blk_xid = pg_sys::TransactionId::from(b.xid);
        if crate::postgres::utils::TransactionIdPrecedesOrEquals(blk_xid, xid)
        && (empty_ok || b.count != 0) {
            return Some(buf);
        }
        bno = next(&buf);
    }
    None
}

fn move_block(
    src: &mut pg_sys::BlockNumber,
    dst: &mut pg_sys::BlockNumber,
    bman: &mut BufferManager,
    mv : &mut BufferMut,
) {
    // list block may have been modified, so we need to walk it again.
    let mut prev : Option<BufferMut> = None;
    let mut bno = *src;
    loop {
        if bno == mv.number() {
            match prev {
                None => {
                    *src = next(mv);
                    mv.page_mut().special_mut::<BM25PageSpecialData>().next_blockno = *dst;
                    *dst = mv.number();
                }
                Some(mut b) =>  {
                    b.page_mut().special_mut::<BM25PageSpecialData>().next_blockno = next(mv);
                    mv.page_mut().special_mut::<BM25PageSpecialData>().next_blockno = *dst;
                    *dst = mv.number();
                }
            }
            return;
        }
        let buf = bman.get_buffer_mut(bno);
        bno = next(&buf);
        if bno == pg_sys::InvalidBlockNumber {
            panic!("partial block not on partial list");
        }
        prev = Some(buf);
    }
}

fn load_root(root : pg_sys::BlockNumber, bman : &mut BufferManager) -> BufferMut {
    let mut buf = bman.get_buffer_mut(root);
    let mut pg = buf.page_mut();
    let r = pg.contents_mut::<FSMRoot>();
    if FSMBlockKind::v2_root == r.kind {
        // already correct format
    } else {
        *r = FSMRoot{
            kind    : FSMBlockKind::v2_root,
            version : 0,
            partial : [pg_sys::InvalidBlockNumber; 32],
            filled  : [pg_sys::InvalidBlockNumber; 32],
        };
    }
    buf
}

fn new_chain(bman : &mut BufferManager, xid : pg_sys::TransactionId) -> BufferMut {
    let mut buf = init_new_buffer(bman.buffer_access().rel());
    let mut pg = buf.page_mut();
    let chain = pg.contents_mut::<FSMChain>();
    *chain = FSMChain { 
        kind: FSMBlockKind::v2_ent,
        xid: xid.into_inner(),
        count: 0,
        entries: [pg_sys::InvalidBlockNumber; NENT]
    };
    buf
}

fn next(b : &BufferMut) -> pg_sys::BlockNumber {
    b.page().special::<BM25PageSpecialData>().next_blockno
}
