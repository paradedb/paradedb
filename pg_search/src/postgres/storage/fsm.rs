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
use crate::postgres::storage::buffer::{init_new_buffer, Buffer, BufferManager, BufferMut};
use crate::postgres::storage::metadata::MetaPage;
use pgrx::iter::TableIterator;
use pgrx::{name, pg_extern, pg_sys, AnyNumeric, PgRelation};
use std::option::Option;

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
struct ChainHeader {
    kind: FSMBlockKind,
    version: u64,
    xid: u32,
    count: i32,
}

const CHAIN_SPACE: usize = bm25_max_free_space() - size_of::<ChainHeader>();
const NENT: usize = CHAIN_SPACE / size_of::<pg_sys::BlockNumber>();
pub const NLIST: usize = 32;

#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct FSMChain {
    h: ChainHeader,
    entries: [pg_sys::BlockNumber; NENT],
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct FSMRoot {
    kind: FSMBlockKind,
    version: u64,
    pub partial: [pg_sys::BlockNumber; NLIST],
    pub filled: [pg_sys::BlockNumber; NLIST],
}

#[derive(Debug)]
pub struct FreeSpaceManager {
    last_slot: usize,
    root: pg_sys::BlockNumber,
}

enum XBuf {
    Ro(Buffer),
    Rw(BufferMut),
}

impl FreeSpaceManager {
    /// Create a new [`FreeSpaceManager`] in the block storage of the specified `indexrel`.
    pub unsafe fn create(indexrel: &PgSearchRelation) -> pg_sys::BlockNumber {
        let mut buf = init_new_buffer(indexrel);
        let mut page = buf.page_mut();
        *page.contents_mut::<FSMRoot>() = FSMRoot {
            kind: FSMBlockKind::v2_root,
            version: 0,
            partial: [pg_sys::InvalidBlockNumber; 32],
            filled: [pg_sys::InvalidBlockNumber; 32],
        };
        buf.number()
    }

    /// Open an existing [`FreeSpaceManager`] which is rooted at the specified blocks
    pub fn open(root: pg_sys::BlockNumber) -> Self {
        Self { last_slot: 0, root }
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
            unsafe { pg_sys::GetCurrentTransactionIdIfAny().max(pg_sys::FirstNormalTransactionId) };
        self.drain_at(bman, horizon, n)
    }

    pub fn drain_at(
        &mut self,
        bman: &mut BufferManager,
        horizon: pg_sys::TransactionId,
        n: usize,
    ) -> impl Iterator<Item = pg_sys::BlockNumber> {
        let mut v: Vec<pg_sys::BlockNumber> = Vec::new();
        while v.len() < n {
            let (retry, killed) = self.drain1(bman, horizon, n - v.len(), &mut v);
            if killed != pg_sys::InvalidBlockNumber {
                let next_xid = unsafe { pg_sys::ReadNextTransactionId() };
                self.extend_with_when_recyclable(bman, next_xid, std::iter::once(killed));
            }
            if !retry {
                break;
            }
        }
        v.into_iter()
    }

    pub fn drain1(
        &mut self,
        bman: &mut BufferManager,
        horizon: pg_sys::TransactionId,
        limit: usize,
        v: &mut Vec<pg_sys::BlockNumber>,
    ) -> (bool, pg_sys::BlockNumber) {
        let rbuf = bman.get_buffer(self.root);
        let root = rbuf.page().contents_ref::<FSMRoot>();
        let mut n = limit as i32;
        for i in 0..NLIST {
            let slot = (self.last_slot + i) % NLIST;
            if let Some(mut buf) = get_chain(bman, root.partial[slot], horizon, false) {
                let bno = buf.number();
                let next_bno = next(&buf);
                let mut killed = pg_sys::InvalidBlockNumber;
                let mut b = buf.page_mut().contents_mut::<FSMChain>();
                self.last_slot = slot;
                if n >= b.h.count {
                    let rvers = root.version;
                    let bvers = b.h.version;
                    drop(buf);
                    let mut mbuf = rbuf.upgrade(bman);
                    let mroot = mbuf.page().contents_ref::<FSMRoot>();
                    buf = bman.get_buffer_mut(bno);
                    if mroot.version != rvers
                        || buf.page().contents_ref::<FSMChain>().h.version != bvers
                    {
                        return (true, pg_sys::InvalidBlockNumber);
                    }
                    b = buf.page_mut().contents_mut::<FSMChain>();
                    n = b.h.count;
                    for i in 1..n + 1 {
                        v.push(b.entries[(b.h.count - i) as usize]);
                    }
                    b.h.count -= n;
                    b.h.version += 1;
                    if bno == mroot.partial[slot] {
                        killed = bno;
                        mbuf.page_mut().contents_mut::<FSMRoot>().partial[slot] = next_bno;
                    }
                } else {
                    for i in 1..n + 1 {
                        v.push(b.entries[(b.h.count - i) as usize]);
                    }
                    b.h.count -= n;
                    b.h.version += 1;
                }
                return (true, killed);
            }
            if let Some(mut buf) = get_chain(bman, root.filled[slot], horizon, false) {
                let bno = buf.number();
                let mut b = buf.page().contents_ref::<FSMChain>();
                let rvers = root.version;
                let bvers = b.h.version;
                drop(buf);

                let mut mbuf = rbuf.upgrade(bman);
                let mroot = mbuf.page().contents_ref::<FSMRoot>();
                buf = bman.get_buffer_mut(bno);
                if mroot.version != rvers
                    || buf.page_mut().contents_mut::<FSMChain>().h.version != bvers
                {
                    return (true, pg_sys::InvalidBlockNumber);
                }
                b = buf.page_mut().contents_mut::<FSMChain>();
                if n > b.h.count {
                    n = b.h.count;
                }
                for i in 1..n + 1 {
                    v.push(b.entries[(b.h.count - i) as usize]);
                }
                let b = buf.page_mut().contents_mut::<FSMChain>();
                b.h.count -= n;
                b.h.version += 1;
                if bno == mroot.filled[slot] {
                    let m = mbuf.page_mut().contents_mut::<FSMRoot>();
                    move_block(&mut m.filled[slot], &mut m.partial[slot], bman, &mut buf);
                }
                self.last_slot = slot;
                return (true, pg_sys::InvalidBlockNumber);
            }
        }
        (false, pg_sys::InvalidBlockNumber)
    }

    pub fn extend(
        &self,
        bman: &mut BufferManager,
        extend_with: impl Iterator<Item = pg_sys::BlockNumber>,
    ) {
        let xid =
            unsafe { pg_sys::GetCurrentTransactionIdIfAny().max(pg_sys::FirstNormalTransactionId) };
        self.extend_with_when_recyclable(bman, xid, extend_with)
    }

    pub fn extend_with_when_recyclable(
        &self,
        bman: &mut BufferManager,
        xid: pg_sys::TransactionId,
        extend_with: impl Iterator<Item = pg_sys::BlockNumber>,
    ) {
        let slot = (xid.into_inner() as usize) % NLIST;
        let mut rbuf = XBuf::Ro(load_root(self.root, bman));
        let mut iter = extend_with.peekable();
        let mut list = {
            match &rbuf {
                XBuf::Ro(b) => b.page().contents_ref::<FSMRoot>().partial[slot],
                XBuf::Rw(b) => b.page().contents_ref::<FSMRoot>().partial[slot],
            }
        };
        'l: loop {
            if iter.peek().is_none() {
                break;
            }
            let (mut buf, vers);
            (buf, vers, rbuf) = next_chain(bman, rbuf, slot, list, xid);
            let chainno = buf.number();
            let b = buf.page().contents_ref::<FSMChain>();
            if b.h.xid != xid.into_inner() {
                list = next(&buf);
                continue;
            }
            let cvers = b.h.version;
            loop {
                match iter.peek() {
                    None => {
                        return;
                    }
                    Some(bno) => {
                        let mut b = buf.page_mut().contents_mut::<FSMChain>();
                        if (b.h.count as usize) + 1 < NENT {
                            b.entries[b.h.count as usize] = *bno;
                            b.h.count += 1;
                            b.h.version += 1;
                            iter.next();
                        } else {
                            if let XBuf::Ro(r) = rbuf {
                                drop(buf);
                                rbuf = XBuf::Rw(r.upgrade(bman));
                                buf = bman.get_buffer_mut(chainno);
                                b = buf.page_mut().contents_mut::<FSMChain>();
                            }
                            if let XBuf::Rw(m) = &mut rbuf {
                                let root = m.page_mut().contents_mut::<FSMRoot>();
                                if vers != root.version || cvers != b.h.version {
                                    list = root.partial[slot];
                                    continue 'l;
                                }
                                b.entries[b.h.count as usize] = *bno;
                                b.h.count += 1;
                                b.h.version += 1;
                                iter.next();
                                root.version += 1;
                                if root.partial[slot] == chainno {
                                    move_block(
                                        &mut root.partial[slot],
                                        &mut root.filled[slot],
                                        bman,
                                        &mut buf,
                                    );
                                }
                                list = root.partial[slot];
                                continue 'l;
                            }
                            panic!("unreachable");
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
    let rbuf = load_root(fsm, &mut bman);
    let root = rbuf.page().contents_ref::<FSMRoot>();
    let mut mapping = Vec::<(u32, u32, u32)>::default();
    let xid = pg_sys::TransactionId::from((i32::MAX - 1) as u32);

    for i in 0..NLIST {
        if root.partial[i] == pg_sys::InvalidBlockNumber
            && root.filled[i] == pg_sys::InvalidBlockNumber
        {
            continue;
        }
        let mut b = root.partial[i];
        while let Some(buf) = get_chain(&mut bman, b, xid, true) {
            let node = buf.page().contents_ref::<FSMChain>();
            for i in 0..node.h.count {
                mapping.push((b, node.entries[i as usize], node.h.xid))
            }
            b = next(&buf);
        }
        b = root.filled[i];
        while let Some(buf) = get_chain(&mut bman, b, xid, true) {
            let node = buf.page().contents_ref::<FSMChain>();
            for i in 0..node.h.count {
                mapping.push((b, node.entries[i as usize], node.h.xid))
            }
            b = next(&buf);
        }
    }

    TableIterator::new(
        mapping
            .into_iter()
            .map(|(a, b, c)| (a.into(), b.into(), c.into())),
    )
}

#[allow(dead_code)]
pub fn fsm_dump(root: pg_sys::BlockNumber, bman: &mut BufferManager, msg: &str) {
    let xid = pg_sys::TransactionId::from((i32::MAX - 1) as u32);
    let mut count = 0;
    let rbuf = bman.get_buffer_mut(root);

    let root = rbuf.page().contents_ref::<FSMRoot>();
    eprintln!("---- BEGIN {msg} --------------------------");
    for i in 0..NLIST {
        if root.partial[i] == pg_sys::InvalidBlockNumber
            && root.filled[i] == pg_sys::InvalidBlockNumber
        {
            continue;
        }
        let mut b = root.partial[i];
        eprintln!("partial[{i}]");
        while let Some(buf) = get_chain(bman, b, xid, true) {
            let c = buf.page().contents_ref::<FSMChain>();
            eprintln!("\t{}@{} [{}/{}]", b, c.h.xid, c.h.count, c.entries.len());
            count += 1;
            b = next(&buf);
        }
        eprintln!("filled[{i}]");
        b = root.filled[i];
        while let Some(buf) = get_chain(bman, b, xid, true) {
            let c = buf.page().contents_ref::<FSMChain>();
            eprintln!("\t{}@{} [{}/{}]", b, c.h.xid, c.h.count, c.entries.len());
            count += 1;
            b = next(&buf);
        }
    }
    eprintln!("total size: {count}");
    eprintln!("---- END {msg} --------------------------");
}

fn next_chain(
    bman: &mut BufferManager,
    rbuf: XBuf,
    slot: usize,
    list: pg_sys::BlockNumber,
    xid: pg_sys::TransactionId,
) -> (BufferMut, u64, XBuf) {
    // technically, a list scan, but we shard things so that we should
    // rarely have overlap in transaction ids, usually the first block
    // we look at should match
    let mut bno = list;
    while bno != pg_sys::InvalidBlockNumber {
        let mut buf = bman.get_buffer_mut(bno);
        bno = buf.page().special::<BM25PageSpecialData>().next_blockno;
        if buf.page().contents_ref::<FSMChain>().h.count == NENT as i32 {
            continue;
        }
        let b = buf.page_mut().contents_mut::<FSMChain>();
        if b.h.xid == xid.into_inner()
            || b.h.count == 0
            || (bno == pg_sys::InvalidBlockNumber && (b.h.count as usize) < b.entries.len())
        {
            b.h.xid = xid.into_inner();
            let vers = match &rbuf {
                XBuf::Ro(b) => b.page().contents_ref::<FSMRoot>().version,
                XBuf::Rw(b) => b.page().contents_ref::<FSMRoot>().version,
            };
            return (buf, vers, rbuf);
        }
    }

    let rbufnum = match rbuf {
        XBuf::Ro(b) => b.number(),
        XBuf::Rw(b) => b.number(),
    };

    let mut buf = new_chain(bman, xid);
    let mut rbuf = XBuf::Rw(bman.get_buffer_mut(rbufnum));
    if let XBuf::Rw(ref mut b) = rbuf {
        let root = b.page_mut().contents_mut::<FSMRoot>();
        buf.page_mut()
            .special_mut::<BM25PageSpecialData>()
            .next_blockno = root.partial[slot];
        root.partial[slot] = buf.number();
        (buf, root.version, rbuf)
    } else {
        panic!("unreachable");
    }
}

// Gets a block containing entries for the current transaction number,
fn get_chain(
    bman: &mut BufferManager,
    list: pg_sys::BlockNumber,
    xid: pg_sys::TransactionId,
    empty_ok: bool,
) -> Option<BufferMut> {
    // technically, a list scan, but we shard things so that we should
    // rarely have overlap in transaction ids, usually the first block
    // we look at should match
    let mut bno = list;
    while bno != pg_sys::InvalidBlockNumber {
        let buf = bman.get_buffer_mut(bno);
        let b = buf.page().contents_ref::<FSMChain>();
        let blk_xid = pg_sys::TransactionId::from(b.h.xid);
        if crate::postgres::utils::TransactionIdPrecedesOrEquals(blk_xid, xid)
            && (empty_ok || b.h.count != 0)
        {
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
    mv: &mut BufferMut,
) {
    // list block may have been modified, so we need to walk it again.
    let mut prev: Option<BufferMut> = None;
    let mut bno = *src;
    loop {
        if bno == mv.number() {
            match prev {
                None => {
                    *src = next(mv);
                    mv.page_mut()
                        .special_mut::<BM25PageSpecialData>()
                        .next_blockno = *dst;
                    *dst = mv.number();
                }
                Some(mut b) => {
                    b.page_mut()
                        .special_mut::<BM25PageSpecialData>()
                        .next_blockno = next(mv);
                    mv.page_mut()
                        .special_mut::<BM25PageSpecialData>()
                        .next_blockno = *dst;
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

fn load_root(root: pg_sys::BlockNumber, bman: &mut BufferManager) -> Buffer {
    let buf = bman.get_buffer(root);
    let r = buf.page().contents_ref::<FSMRoot>();
    if FSMBlockKind::v2_root == r.kind {
        return buf;
    }

    let mut mbuf = buf.upgrade(bman);
    let mr = mbuf.page_mut().contents_mut::<FSMRoot>();
    *mr = FSMRoot {
        kind: FSMBlockKind::v2_root,
        version: 0,
        partial: [pg_sys::InvalidBlockNumber; 32],
        filled: [pg_sys::InvalidBlockNumber; 32],
    };
    drop(mbuf);
    load_root(root, bman)
}

fn new_chain(bman: &mut BufferManager, xid: pg_sys::TransactionId) -> BufferMut {
    // let mut buf = init_new_buffer(bman.buffer_access().rel());
    let mut buf = bman.new_buffer();
    let mut pg = buf.init_page();
    let chain = pg.contents_mut::<FSMChain>();
    *chain = FSMChain {
        h: ChainHeader {
            kind: FSMBlockKind::v2_ent,
            version: 0,
            xid: xid.into_inner(),
            count: 0,
        },
        entries: [pg_sys::InvalidBlockNumber; NENT],
    };
    buf
}

fn next(b: &BufferMut) -> pg_sys::BlockNumber {
    b.page().special::<BM25PageSpecialData>().next_blockno
}
