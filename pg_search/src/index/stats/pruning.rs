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

use super::{SegmentStats, comparable};
use crate::api::HashSet;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::postgres::rel::PgSearchRelation;
use crate::scan::range_partitioning::RangePartitioning;

/// The box a partitioned build stamped on a segment, projected onto one field, with the
/// segment's size: what a partition boundary costs when it lands inside the box.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SegmentBox {
    pub(crate) lower: Bound<PdbOwnedValue>,
    pub(crate) upper: Bound<PdbOwnedValue>,
    pub(crate) num_docs: u64,
}

impl SegmentBox {
    /// Whether a boundary at `value` lands inside the box, so that the partitions on either
    /// side of it each hold part of the segment. A boundary on an edge leaves the segment
    /// whole.
    pub(crate) fn is_cut_by(&self, value: &PdbOwnedValue) -> bool {
        let above_lower = match &self.lower {
            Bound::Unbounded => true,
            Bound::Included(lo) | Bound::Excluded(lo) => {
                comparable(lo, value) && lo.total_cmp(value) == Ordering::Less
            }
        };
        let below_upper = match &self.upper {
            Bound::Unbounded => true,
            Bound::Excluded(hi) => comparable(hi, value) && value.total_cmp(hi) == Ordering::Less,
            Bound::Included(hi) => {
                comparable(hi, value) && value.total_cmp(hi) != Ordering::Greater
            }
        };
        above_lower && below_upper
    }
}

/// The boxes of the visible segments on `partition_by`. `None` if no segment has one. A
/// segment without a box is not a problem: at execution, it is kept or skipped on its own
/// statistics.
pub(crate) fn persisted_segment_boxes(
    indexrel: &PgSearchRelation,
    partition_by: &str,
) -> anyhow::Result<Option<Vec<SegmentBox>>> {
    if indexrel.options().partition_by().is_empty() {
        return Ok(None);
    }
    let index = crate::index::open_index(MvccSatisfies::Snapshot.directory(indexrel))?;
    let Ok(field) = index.schema().get_field(partition_by) else {
        return Ok(None);
    };
    let mut boxes = Vec::new();
    for segment in index.searchable_segments()? {
        let Some(stats) = SegmentStats::of_segment(&segment)? else {
            continue;
        };
        let Some(bounds) = stats.logical(field)? else {
            continue;
        };
        boxes.push(SegmentBox {
            lower: bounds.lower,
            upper: bounds.upper,
            num_docs: u64::from(segment.meta().num_docs()),
        });
    }
    Ok((!boxes.is_empty()).then_some(boxes))
}

/// The distinct edges of `boxes`, ascending: the values a partition boundary can sit on
/// without cutting any of these segments.
pub(crate) fn box_edges(boxes: &[SegmentBox]) -> Vec<PdbOwnedValue> {
    let mut points: Vec<PdbOwnedValue> = boxes
        .iter()
        .flat_map(|b| [&b.lower, &b.upper])
        .filter_map(|bound| match bound {
            Bound::Included(v) | Bound::Excluded(v) => Some(v.clone()),
            Bound::Unbounded => None,
        })
        .collect();
    points.sort_unstable_by(PdbOwnedValue::total_cmp);
    points.dedup_by(|a, b| a.total_cmp(b) == Ordering::Equal);
    points
}

/// The documents of every box each of `cuts` lands inside, summed over the cuts: the rows a
/// range partitioning on those cuts has to filter one by one instead of taking whole.
pub(crate) fn cut_docs<'a>(
    boxes: impl IntoIterator<Item = &'a SegmentBox>,
    cuts: &[PdbOwnedValue],
) -> u64 {
    boxes
        .into_iter()
        .map(|b| b.num_docs * cuts.iter().filter(|cut| b.is_cut_by(cut)).count() as u64)
        .sum()
}

/// Split points for `partition_by`, collected from the boxes of the visible segments: every
/// box edge is a split point. `None` if no segment has a box.
pub(crate) fn persisted_split_points(
    indexrel: &PgSearchRelation,
    partition_by: &str,
) -> anyhow::Result<Option<Vec<PdbOwnedValue>>> {
    Ok(persisted_segment_boxes(indexrel, partition_by)?
        .map(|boxes| box_edges(&boxes))
        .filter(|points| !points.is_empty()))
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

#[cfg(test)]
mod cut_tests {
    use super::*;

    fn i64(v: i64) -> PdbOwnedValue {
        PdbOwnedValue::I64(v)
    }

    fn boxed(lower: Option<i64>, upper: Option<i64>, num_docs: u64) -> SegmentBox {
        SegmentBox {
            lower: lower.map_or(Bound::Unbounded, |v| Bound::Included(i64(v))),
            upper: upper.map_or(Bound::Unbounded, |v| Bound::Excluded(i64(v))),
            num_docs,
        }
    }

    #[test]
    fn a_cut_on_an_edge_leaves_the_segment_whole() {
        let b = boxed(Some(10), Some(20), 5);
        assert!(!b.is_cut_by(&i64(10)));
        assert!(!b.is_cut_by(&i64(20)));
        assert!(b.is_cut_by(&i64(11)));
        assert!(b.is_cut_by(&i64(19)));
        assert!(!b.is_cut_by(&i64(9)));
        assert!(!b.is_cut_by(&i64(21)));
        // A box that is unbounded on the field is cut by every value.
        assert!(boxed(None, None, 1).is_cut_by(&i64(0)));
        // Values of another kind say nothing about the box.
        assert!(!b.is_cut_by(&PdbOwnedValue::Str("15".into())));
    }

    #[test]
    fn nested_boxes_are_cheap_to_cut_around() {
        // Two fields, `x` first: four global `x` ranges, each cut once more on `y`, so every
        // `x` edge is shared by two segments and lands inside none.
        let nested: Vec<SegmentBox> = (0..4)
            .flat_map(|i| {
                let lower = (i > 0).then_some(i * 100);
                let upper = (i < 3).then_some((i + 1) * 100);
                [boxed(lower, upper, 50), boxed(lower, upper, 50)]
            })
            .collect();
        // One field on the other side, cut at quantiles that miss the `x` edges.
        let flat: Vec<SegmentBox> = [(None, Some(90)), (Some(90), Some(210)), (Some(210), None)]
            .into_iter()
            .map(|(lo, hi)| boxed(lo, hi, 100))
            .collect();

        assert_eq!(box_edges(&nested), vec![i64(100), i64(200), i64(300)]);
        assert_eq!(box_edges(&flat), vec![i64(90), i64(210)]);

        let all = || nested.iter().chain(flat.iter());
        // The nested edges cut one flat segment each; the flat edges cut both segments of the
        // `x` range they land in.
        assert_eq!(cut_docs(all(), &box_edges(&nested)), 300);
        assert_eq!(cut_docs(all(), &box_edges(&flat)), 200);
        assert_eq!(cut_docs(all(), &[i64(100), i64(200)]), 200);
        assert_eq!(cut_docs(all(), &[i64(90), i64(210)]), 200);
        assert_eq!(cut_docs(all(), &[i64(150)]), 200);
    }
}
