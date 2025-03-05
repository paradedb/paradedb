// Copyright (c) 2023-2025 Retake, Inc.
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

use crate::postgres::storage::block::SegmentMetaEntry;
use crate::postgres::utils::u64_to_item_pointer;
use pgrx::{pg_sys, Spi};
use std::ffi::CStr;
use std::sync::atomic::{AtomicBool, Ordering};
use tantivy::index::{InnerSegmentMeta, SegmentId};
use tantivy::indexer::MergeCandidate;
use tantivy::SegmentMeta;

static CREATED: AtomicBool = AtomicBool::new(false);

fn appname() -> &'static str {
    unsafe {
        CStr::from_ptr(pg_sys::application_name)
            .to_str()
            .expect("appname should be UTF8")
    }
}

pub fn track_segment_meta_entry(event: &str, entry: &SegmentMetaEntry) {
    // create_table();
    //
    // let sql = r#"
    //     INSERT INTO public.segment_tracker (
    //         appname,
    //         event,
    //         segment_id,
    //         max_doc,
    //         entry_xmin,
    //         entry_xmax,
    //         postings_block,
    //         postings_bytes,
    //         positions_block,
    //         positions_bytes,
    //         fast_fields_block,
    //         fast_fields_bytes,
    //         norms_block,
    //         norms_bytes,
    //         terms_block,
    //         terms_bytes,
    //         store_block,
    //         store_bytes,
    //         temp_block,
    //         temp_bytes,
    //         delete_block,
    //         delete_bytes,
    //         num_deleted
    //     ) VALUES (
    //         $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23
    //     )"#;
    //
    // Spi::run_with_args(
    //     sql,
    //     &[
    //         appname().into(),
    //         event.into(),
    //         entry.segment_id.short_uuid_string().into(),
    //         (entry.max_doc as i32).into(),
    //         (entry.xmin as i64).into(),
    //         (entry.xmax as i64).into(),
    //         entry.postings.map(|x| x.starting_block as i64).into(),
    //         entry.postings.map(|x| x.total_bytes as i64).into(),
    //         entry.positions.map(|x| x.starting_block as i64).into(),
    //         entry.positions.map(|x| x.total_bytes as i64).into(),
    //         entry.fast_fields.map(|x| x.starting_block as i64).into(),
    //         entry.fast_fields.map(|x| x.total_bytes as i64).into(),
    //         entry.fast_fields.map(|x| x.starting_block as i64).into(),
    //         entry.field_norms.map(|x| x.total_bytes as i64).into(),
    //         entry.terms.map(|x| x.starting_block as i64).into(),
    //         entry.terms.map(|x| x.total_bytes as i64).into(),
    //         entry.store.map(|x| x.starting_block as i64).into(),
    //         entry.store.map(|x| x.total_bytes as i64).into(),
    //         entry.temp_store.map(|x| x.starting_block as i64).into(),
    //         entry.temp_store.map(|x| x.total_bytes as i64).into(),
    //         entry
    //             .delete
    //             .map(|x| x.file_entry.starting_block as i64)
    //             .into(),
    //         entry.delete.map(|x| x.file_entry.total_bytes as i64).into(),
    //         entry.delete.map(|x| x.num_deleted_docs as i64).into(),
    //     ],
    // )
    // .expect("failed to insert SegmentMetaEntry")
}

pub fn track_segment_meta(event: &str, segment: &SegmentMeta) {
    // let tracked = &segment.tracked;
    // track_inner_segment_meta(event, tracked);
}

pub fn track_inner_segment_meta(event: &str, segment: &InnerSegmentMeta) {
    // create_table();
    // let sql = r#"
    //     INSERT INTO public.segment_tracker (
    //         appname,
    //         event,
    //         segment_id,
    //         max_doc,
    //         num_deleted
    //     ) VALUES (
    //         $1, $2, $3, $4, $5
    //     )
    // "#;
    //
    // Spi::run_with_args(
    //     sql,
    //     &[
    //         appname().into(),
    //         event.into(),
    //         segment.segment_id.short_uuid_string().into(),
    //         (segment.max_doc as i32).into(),
    //         segment
    //             .deletes
    //             .as_ref()
    //             .map(|x| x.num_deleted_docs as i64)
    //             .into(),
    //     ],
    // )
    // .expect("failed to insert SegmentMeta");
}

pub fn track_ctid(event: &str, segment_id: Option<&SegmentId>, ctid: u64) {
    // create_table();
    //
    // let sql = r#"
    //     INSERT INTO public.segment_tracker (
    //         appname,
    //         event,
    //         segment_id,
    //         source_ctid
    //     ) VALUES (
    //         $1, $2, $3, $4
    //     )
    // "#;
    //
    // Spi::run_with_args(
    //     sql,
    //     &[
    //         appname().into(),
    //         event.into(),
    //         segment_id.map(|s| s.short_uuid_string()).into(),
    //         {
    //             let mut tid = pg_sys::ItemPointerData::default();
    //             u64_to_item_pointer(ctid, &mut tid);
    //             tid.into()
    //         },
    //     ],
    // )
    // .unwrap();
}

pub fn track_merge_candidates(event: &str, candidates: &[Vec<InnerSegmentMeta>]) {
    // for (i, candidate) in candidates.iter().enumerate() {
    //     let event = format!("{event}:{i}");
    //
    //     for segment in candidate {
    //         track_inner_segment_meta(&event, segment);
    //     }
    // }
}

fn create_table() {
    // if CREATED.load(Ordering::Relaxed) {
    //     return;
    // }
    //
    // let sql = r#"
    //     CREATE TABLE IF NOT EXISTS public.segment_tracker(
    //         id SERIAL8 NOT NULL PRIMARY KEY,
    //         ts TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT clock_timestamp(),
    //         appname TEXT,
    //         event TEXT,
    //         segment_id TEXT,
    //         source_ctid tid,
    //         max_doc int,
    //         entry_xmin bigint,
    //         entry_xmax bigint,
    //         postings_block bigint,
    //         postings_bytes bigint,
    //         positions_block bigint,
    //         positions_bytes bigint,
    //         fast_fields_block bigint,
    //         fast_fields_bytes bigint,
    //         norms_block bigint,
    //         norms_bytes bigint,
    //         terms_block bigint,
    //         terms_bytes bigint,
    //         store_block bigint,
    //         store_bytes bigint,
    //         temp_block bigint,
    //         temp_bytes bigint,
    //         delete_block bigint,
    //         delete_bytes bigint,
    //         num_deleted bigint
    //     );
    // "#;
    // Spi::run(sql).expect("failed to create segment_tracker table");
    // CREATED.store(true, Ordering::Relaxed);
}
