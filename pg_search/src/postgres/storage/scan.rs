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

use std::collections::HashSet;
use std::cmp::min;
use pgrx::{pg_extern, pg_sys, name, AnyNumeric, PgRelation, iter::TableIterator};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::metadata::MetaPage;
use crate::postgres::storage::buffer::BufferManager;
use crate::postgres::storage::block::{SegmentMetaEntry, FileEntry, BM25PageSpecialData, LinkedList};
use crate::postgres::storage::fsm::FSMRoot;

#[pg_extern]
unsafe fn used_blocks(
    index: PgRelation,
) -> TableIterator<'static, (
    name!(meta, AnyNumeric),
    name!(fixed, AnyNumeric),
    name!(fsm, AnyNumeric),
    name!(schema, AnyNumeric),
    name!(settings, AnyNumeric),
    name!(segmeta, AnyNumeric),
    name!(garbage, AnyNumeric),
    name!(segfile, AnyNumeric),
    name!(vaclist, AnyNumeric),
    name!(mergelist, AnyNumeric),
)> {
    let index = PgSearchRelation::from_pg(index.as_ptr());
    let mut bman = BufferManager::new(&index);
    let mp = MetaPage::open(&index);

    let meta = HashSet::<pg_sys::BlockNumber>::new();
    let fixed = HashSet::<pg_sys::BlockNumber>::new();
    let mut fsm = HashSet::<pg_sys::BlockNumber>::new();
    let mut schema = HashSet::<pg_sys::BlockNumber>::new();
    let mut settings = HashSet::<pg_sys::BlockNumber>::new();
    let mut segmeta = HashSet::<pg_sys::BlockNumber>::new();
    let mut garbage = HashSet::<pg_sys::BlockNumber>::new();
    let mut segfile = HashSet::<pg_sys::BlockNumber>::new();
    let vaclist = HashSet::<pg_sys::BlockNumber>::new();
    let mergelist = HashSet::<pg_sys::BlockNumber>::new();

    scan_fsm(&mut bman, mp.fsm(), &mut fsm);
    schema.extend(mp.schema_bytes().freeable_blocks());
    settings.extend(mp.settings_bytes().freeable_blocks());
    scan_chain(&mut bman, mp.segment_metas().get_header_blockno(), &mut segmeta);
//    let vacuum_list = LinkedBytesList::open(&index, mp.vacuum_list().start_block_number);
//    vaclist.extend(vacuum_list.freeable_blocks());
//    vaclist.insert(mp.vacuum_list().ambulkdelete_sentinel);

//    let merge_lock = mp.acquire_merge_lock();
//    let merge_list = LinkedBytesList::open(&index, merge_lock.merge_list().entries.get_header_blockno());
//    mergelist.extend(merge_list.freeable_blocks());
//    drop(merge_lock);

    if let Some(g) = mp.segment_metas_garbage() {
        scan_chain(&mut bman, g.get_header_blockno(), &mut garbage);
    }
    for m in mp.segment_metas().list() {
        scan_meta(&mut bman, &m, &mut segfile);
    }

    TableIterator::new(vec![(
        (meta.len() as i64).into(),
        (fixed.len() as i64).into(),
        (fsm.len() as i64).into(),
        (schema.len() as i64).into(),
        (settings.len() as i64).into(),
        (segmeta.len() as i64).into(),
        (garbage.len() as i64).into(),
        (segfile.len() as i64).into(),
        (vaclist.len() as i64).into(),
        (mergelist.len() as i64).into(),
    )])
}

fn scan_meta(
    bman: &mut BufferManager,
    m: &SegmentMetaEntry,
    live: &mut HashSet<pg_sys::BlockNumber>
) {
    scan_file(bman, &m.postings, live);
    scan_file(bman, &m.positions, live);
    scan_file(bman, &m.fast_fields, live);
    scan_file(bman, &m.field_norms, live);
    scan_file(bman, &m.terms, live);
    scan_file(bman, &m.store, live);
    scan_file(bman, &m.temp_store, live);
    if let Some(de) = &m.delete {
        scan_file(bman, &Some(de.file_entry), live);
    }
}

fn scan_file(
    bman: &mut BufferManager,
    file: &Option<FileEntry>,
    live: &mut HashSet<pg_sys::BlockNumber>
) {
    if let Some(e) = file {
        let mut b = e.starting_block;
        let mut n = e.total_bytes;

        while b != pg_sys::InvalidBlockNumber && n > 0 {
            live.insert(b);
            let buf = bman.get_buffer(b);
            let pg = buf.page();
            n -= min(n, pg_sys::BLCKSZ as usize - pg.free_space());
            b = pg.special::<BM25PageSpecialData>().next_blockno;
        }
    }
}

fn scan_fsm(
    bman: &mut BufferManager,
    fsm_root: pg_sys::BlockNumber,
    live: &mut HashSet<pg_sys::BlockNumber>
) {
    live.insert(fsm_root);
    let buf = bman.get_buffer(fsm_root);
    let pg = buf.page();
    let root = pg.contents::<FSMRoot>();

    for i in 0..32 {
        scan_chain(bman, root.partial[i], live);
        scan_chain(bman, root.filled[i], live);
    }
}

fn scan_chain(
    bman: &mut BufferManager,
    mut b: pg_sys::BlockNumber,
    live: &mut HashSet<pg_sys::BlockNumber>
) {
    while b != pg_sys::InvalidBlockNumber {
        live.insert(b);
        let buf = bman.get_buffer(b);
        let pg = buf.page();
        b = pg.special::<BM25PageSpecialData>().next_blockno;
    }
}


