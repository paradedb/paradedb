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
//! What the planner and the executor take from the component: logical split points stamped
//! by a partitioned build, and the execution segments a range partition has to search. The
//! planner keeps only the values; segment ownership is resolved at execution.

use std::cmp::Ordering;
use std::ops::Bound;

use tantivy::index::SegmentId;

use super::SegmentStats;
use crate::api::HashSet;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::postgres::rel::PgSearchRelation;
use crate::scan::range_partitioning::RangePartitioning;

/// A segment's 1D bounding box projection, empirical value bounds, and document count along a field.
///
/// When an index is built with `partition_by` (e.g. via a 1D or multi-dimensional KD-tree), each
/// segment is assigned logical bounding coordinates (`lower`, `upper`) representing its assigned cell
/// in partition space. The segment also records empirical observed `[min, max]` values for the field
/// and its total document count.
///
/// This metadata allows the range partition optimizer to determine which segments are bisected by a
/// candidate cut point and estimate how documents are distributed across partition boundaries.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Segment1DBounds {
    pub(crate) segment_id: SegmentId,
    pub(crate) lower: Bound<PdbOwnedValue>,
    pub(crate) upper: Bound<PdbOwnedValue>,
    pub(crate) empirical_min: Option<PdbOwnedValue>,
    pub(crate) empirical_max: Option<PdbOwnedValue>,
    pub(crate) num_docs: u64,
}

/// Reads the 1D logical bounds, empirical stats, and document counts for all visible segments of `indexrel`.
///
/// Opens the index snapshot directory and extracts segment statistics for `partition_by`.
/// Returns `None` if:
/// - `indexrel` was not built with `partition_by`,
/// - `partition_by` does not exist in the index schema, or
/// - no searchable segments carry valid bounds for `partition_by`.
pub(crate) fn persisted_segment_bounds(
    indexrel: &PgSearchRelation,
    partition_by: &str,
) -> anyhow::Result<Option<Vec<Segment1DBounds>>> {
    if indexrel.options().partition_by().is_empty() {
        return Ok(None);
    }
    let index = crate::index::open_index(MvccSatisfies::Snapshot.directory(indexrel))?;
    let Ok(field) = index.schema().get_field(partition_by) else {
        return Ok(None);
    };
    let mut segments = Vec::new();
    for segment in index.searchable_segments()? {
        let Some(stats) = SegmentStats::of_segment(&segment)? else {
            continue;
        };
        let Some(bounds) = stats.logical(field)? else {
            continue;
        };
        let empirical = stats.empirical(field).ok().flatten();
        let (empirical_min, empirical_max) = match empirical {
            Some(e) => (Some(e.min), Some(e.max)),
            None => (None, None),
        };
        segments.push(Segment1DBounds {
            segment_id: segment.id(),
            lower: bounds.lower,
            upper: bounds.upper,
            empirical_min,
            empirical_max,
            num_docs: segment.meta().num_docs() as u64,
        });
    }
    Ok((!segments.is_empty()).then_some(segments))
}

/// Split points for `partition_by`, collected from the boxes of the visible segments: every
/// box edge is a split point. `None` if no segment has a box. A segment without a box is not
/// a problem: at execution, it is kept or skipped on its own statistics.
pub(crate) fn persisted_split_points(
    indexrel: &PgSearchRelation,
    partition_by: &str,
) -> anyhow::Result<Option<Vec<PdbOwnedValue>>> {
    let Some(segments) = persisted_segment_bounds(indexrel, partition_by)? else {
        return Ok(None);
    };
    let mut points = Vec::new();
    for segment in segments {
        for bound in [segment.lower, segment.upper] {
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

/// The classified segments for a given partition. Pruned segments are listed by ID, not
/// counted, so a receiver can prove the decisions were made over its own segment view.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(crate) struct PartitionSegments {
    pub(crate) included: Vec<SegmentId>,
    pub(crate) partially_included: Vec<SegmentId>,
    pub(crate) pruned: Vec<SegmentId>,
}

impl PartitionSegments {
    /// Whether these decisions were made over exactly `reader`'s segment view: the three lists
    /// name every segment of the view once and nothing else. Mutable segments keep their ID
    /// across views and are always partially included, so ID identity is sufficient.
    pub(crate) fn covers_view(&self, reader: &SearchIndexReader) -> bool {
        let snapshot = reader.segment_stats_snapshot();
        let listed: HashSet<SegmentId> = self
            .included
            .iter()
            .chain(&self.partially_included)
            .chain(&self.pruned)
            .copied()
            .collect();
        listed.len() == self.included.len() + self.partially_included.len() + self.pruned.len()
            && listed.len() == reader.segment_readers().len()
            && listed
                .iter()
                .all(|id| snapshot.segment_index(*id).is_some())
    }
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
        pruned: Vec::new(),
    };
    let Some(range) = boundaries.partition_range(partition) else {
        return PartitionSegments {
            included: reader.segment_ids(),
            partially_included: Vec::new(),
            pruned: Vec::new(),
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
