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
//! Logical split points stamped by a partitioned build, and the execution segments a range
//! partition has to search. Join range selection and segment ownership are resolved from
//! execution-visible statistics when the physical plan is built.

use std::cmp::Ordering;
use std::ops::Bound;

use tantivy::Index;
use tantivy::index::SegmentId;

use super::SegmentStats;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::postgres::rel::PgSearchRelation;
use crate::scan::range_partitioning::RangePartitioning;

/// Split points for `partition_by`, collected from the boxes of the visible segments: every
/// box edge is a split point. `None` if no segment has a box. A segment without a box is not
/// a problem: at execution, it is kept or skipped on its own statistics.
pub(crate) fn persisted_split_points(
    indexrel: &PgSearchRelation,
    partition_by: &str,
) -> anyhow::Result<Option<Vec<PdbOwnedValue>>> {
    if indexrel.options().partition_by().is_empty() {
        return Ok(None);
    }
    let index = Index::open(MvccSatisfies::Snapshot.directory(indexrel))?;
    let Ok(field) = index.schema().get_field(partition_by) else {
        return Ok(None);
    };
    let mut points = Vec::new();
    for segment in index.searchable_segments()? {
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

/// How a segment's bounds relate to a partition's range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SegmentInclusion {
    /// The segment is fully contained within the partition; no range filter needed.
    FullyIncluded,
    /// The segment overlaps the partition boundary; range filter must be applied.
    PartiallyIncluded,
    /// The segment does not intersect the partition; excluded from execution.
    Excluded,
}

/// The classified segments for a given partition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PartitionSegments {
    pub(crate) included: Vec<SegmentId>,
    pub(crate) partially_included: Vec<SegmentId>,
    pub(crate) pruned_count: usize,
}

#[cfg(any(test, feature = "pg_test"))]
impl PartitionSegments {
    pub(crate) fn len(&self) -> usize {
        self.included.len() + self.partially_included.len()
    }

    pub(crate) fn contains(&self, id: &SegmentId) -> bool {
        self.included.contains(id) || self.partially_included.contains(id)
    }
}

#[cfg(any(test, feature = "pg_test"))]
impl IntoIterator for PartitionSegments {
    type Item = SegmentId;
    type IntoIter = std::vec::IntoIter<SegmentId>;

    fn into_iter(self) -> Self::IntoIter {
        let mut all = self.included;
        all.extend(self.partially_included);
        all.into_iter()
    }
}

#[cfg(any(test, feature = "pg_test"))]
impl From<PartitionSegments> for Vec<SegmentId> {
    fn from(segments: PartitionSegments) -> Self {
        segments.into_iter().collect()
    }
}

/// The segments of `reader` that can hold a row of `partition`, classified by whether they
/// require a partition `RangeQuery` or are fully contained within the partition range.
pub(crate) fn segments_for_partition(
    reader: &SearchIndexReader,
    boundaries: &RangePartitioning,
    partition: usize,
) -> PartitionSegments {
    let all = || PartitionSegments {
        included: Vec::new(),
        partially_included: reader.segment_ids(),
        pruned_count: 0,
    };
    let Some(range) = boundaries.partition_range(partition) else {
        return PartitionSegments {
            included: reader.segment_ids(),
            partially_included: Vec::new(),
            pruned_count: 0,
        };
    };
    let Some(field) = reader
        .schema()
        .search_field(boundaries.partition_by.as_ref())
    else {
        return all();
    };
    if !field.stats_order_matches_values() {
        return all();
    }
    reader
        .segment_stats_snapshot()
        .classify_partition_segments(&field, &range)
}
