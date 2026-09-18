// Copyright (c) 2023-2026 ParadeDB, Inc.
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

use crate::api::CTID_FIELD_NAME;
use crate::index::mvcc::SegmentViewDocs;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::heap::VisibilityChecker;
use crate::postgres::rel::PgSearchRelation;
use pgrx::pg_sys;
use tantivy::columnar::Cardinality;

pub(super) fn reader_is_all_visible(
    reader: &SearchIndexReader,
    heaprel: &PgSearchRelation,
) -> tantivy::Result<bool> {
    let snapshot = unsafe { pg_sys::GetActiveSnapshot() };
    if snapshot.is_null()
        || unsafe { (*snapshot).snapshot_type != pg_sys::SnapshotType::SNAPSHOT_MVCC }
        || reader
            .segment_view()
            .entries()
            .iter()
            .any(|entry| !matches!(entry.docs, SegmentViewDocs::Immutable { .. }))
    {
        return Ok(false);
    }

    let nblocks = unsafe {
        pg_sys::RelationGetNumberOfBlocksInFork(heaprel.as_ptr(), pg_sys::ForkNumber::MAIN_FORKNUM)
    };
    let mut ranges = Vec::with_capacity(reader.segment_readers().len());
    for segment in reader.segment_readers() {
        pgrx::check_for_interrupts!();
        if segment.num_docs() == 0 {
            continue;
        }
        let ctids = segment.fast_fields().u64(CTID_FIELD_NAME)?;
        if ctids.get_cardinality() != Cardinality::Full || ctids.num_docs() != segment.max_doc() {
            return Ok(false);
        }
        let first = ctids.min_value() >> 16;
        let last = ctids.max_value() >> 16;
        if first > last || last >= u64::from(nblocks) {
            return Ok(false);
        }
        ranges.push((first as pg_sys::BlockNumber, last as pg_sys::BlockNumber));
    }

    // The reader and CTID bounds must precede the fresh VM reads. Its cleanup pin also
    // prevents VACUUM from marking removed index entries' heap pages all-visible.
    ranges.sort_unstable();
    let mut visibility = VisibilityChecker::with_rel_and_snap(heaprel, snapshot);
    let mut checked_through: Option<pg_sys::BlockNumber> = None;
    for (mut first, last) in ranges {
        if let Some(previous) = checked_through {
            if last <= previous {
                continue;
            }
            first = first.max(previous + 1);
        }
        for block in first..=last {
            if block % 4096 == 0 {
                pgrx::check_for_interrupts!();
            }
            if !visibility.is_block_all_visible(block) {
                return Ok(false);
            }
        }
        checked_through = Some(last);
    }
    Ok(true)
}
