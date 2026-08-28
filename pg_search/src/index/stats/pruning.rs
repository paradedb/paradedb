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
//! What the planner and the executor take from the component: the split points a partitioned
//! build stamped, and the segments a range partition has to search.

use std::cmp::Ordering;
use std::ops::Bound;

use tantivy::Index;
use tantivy::index::{SegmentId, SegmentReader};

use super::SegmentStats;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::types::is_datetime_type;
use crate::scan::range_partitioning::RangePartitioning;
use crate::schema::SearchFieldType;

/// The split points a partitioned build stamped on the index: the edges of every box a visible
/// segment carries for `partition_by`. `None` when no segment carries one. A segment without a
/// box does not veto the grid, since its own statistics place it at execution.
pub(crate) fn persisted_split_points(
    indexrel: &PgSearchRelation,
    partition_by: &str,
) -> anyhow::Result<Option<Vec<PdbOwnedValue>>> {
    if indexrel.options().partition_by().is_empty() {
        return Ok(None);
    }
    let directory = MvccSatisfies::Snapshot.directory(indexrel);
    let index = Index::open(directory.clone())?;
    let Ok(field) = index.schema().get_field(partition_by) else {
        return Ok(None);
    };
    let mut points = Vec::new();
    for segment in index.searchable_segments()? {
        // Opening a component of a mutable segment materializes the whole segment first, and its
        // entry already says it has no `.stats`.
        let has_stats = directory
            .segment_meta_entry(&segment.id())
            .is_some_and(|entry| entry.stats().is_some());
        if !has_stats {
            continue;
        }
        let Some(stats) = SegmentStats::of_segment(&segment)? else {
            continue;
        };
        let Some(bounds) = stats.logical(field)? else {
            continue;
        };
        for bound in [bounds.lower, bounds.upper] {
            if let Bound::Included(v) | Bound::Excluded(v) = bound {
                points.push(v);
            }
        }
    }
    points.sort_unstable_by(PdbOwnedValue::total_cmp);
    points.dedup_by(|a, b| a.total_cmp(b) == Ordering::Equal);
    Ok((!points.is_empty()).then_some(points))
}

/// The segments of `reader` that can hold a row of `partition`. A segment without statistics
/// it can be ranked against is kept, so the query the caller still applies stays the source of
/// truth.
pub(crate) fn segments_for_partition(
    reader: &SearchIndexReader,
    boundaries: &RangePartitioning,
    partition: usize,
) -> Vec<SegmentId> {
    let all = reader.segment_ids();
    let Some(range) = boundaries.partition_range(partition) else {
        return all;
    };
    let field_name = boundaries.partition_by.as_ref();
    let Ok(field) = reader.schema().tantivy_schema().get_field(field_name) else {
        return all;
    };
    // A recent index keeps datetimes in an `I64` column, so its statistics need the same lift
    // as its values; a legacy index stored them as `Date` already.
    let is_date = match reader
        .schema()
        .search_field(field_name)
        .map(|f| f.field_type())
    {
        Some(SearchFieldType::Date(_)) => true,
        Some(SearchFieldType::I64(oid)) => is_datetime_type(oid),
        _ => false,
    };

    reader
        .segment_readers()
        .iter()
        .filter(|segment_reader| {
            let Ok(Some(stats)) = SegmentStats::of_reader(segment_reader) else {
                return true;
            };
            match stats.logical(field) {
                Ok(Some(bounds)) => {
                    return (range.includes_nulls && bounds.may_hold_nulls())
                        || bounds.intersects(&range.lower, &range.upper);
                }
                Ok(None) => {}
                Err(_) => return true,
            }
            let Ok(Some(empirical)) = stats.empirical(field) else {
                return true;
            };
            let empirical = if is_date {
                match empirical.into_dates() {
                    Some(empirical) => empirical,
                    None => return true,
                }
            } else {
                empirical
            };
            (range.includes_nulls && empirical.nullable)
                || empirical.intersects(&range.lower, &range.upper)
        })
        .map(SegmentReader::segment_id)
        .collect()
}
