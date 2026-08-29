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

//! What range partitioning takes from the component: the split points a partitioned build
//! recorded, and the segments a partition has to search.

use std::cmp::Ordering;
use std::ops::Bound;

use tantivy::Index;
use tantivy::index::SegmentId;

use super::SegmentStats;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::types::is_datetime_type;
use crate::scan::range_partitioning::RangePartitioning;
use crate::schema::SearchFieldType;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SegmentInclusion {
    FullyIncluded,
    PartiallyIncluded,
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
    pub(crate) fn total_scanned(&self) -> usize {
        self.included.len() + self.partially_included.len()
    }

    pub(crate) fn all_scanned(&self) -> impl Iterator<Item = &SegmentId> {
        self.included.iter().chain(self.partially_included.iter())
    }
}

/// The segments of `reader` that can hold a row of `partition`, classified by whether they
/// require a partition `RangeQuery` or are fully contained within the partition range.
pub(crate) fn segments_for_partition(
    reader: &SearchIndexReader,
    boundaries: &RangePartitioning,
    partition: usize,
) -> PartitionSegments {
    let all = reader.segment_ids();
    let Some(range) = boundaries.partition_range(partition) else {
        return PartitionSegments {
            included: all,
            partially_included: Vec::new(),
            pruned_count: 0,
        };
    };
    let field_name = boundaries.partition_by.as_ref();
    let Ok(field) = reader.schema().tantivy_schema().get_field(field_name) else {
        return PartitionSegments {
            included: Vec::new(),
            partially_included: all,
            pruned_count: 0,
        };
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

    let mut included = Vec::new();
    let mut partially_included = Vec::new();
    let mut pruned_count = 0;

    for segment_reader in reader.segment_readers() {
        let Ok(Some(stats)) = SegmentStats::of_reader(segment_reader) else {
            partially_included.push(segment_reader.segment_id());
            continue;
        };
        let Ok(logical) = stats.logical(field) else {
            partially_included.push(segment_reader.segment_id());
            continue;
        };
        let Ok(empirical) = stats.empirical(field) else {
            partially_included.push(segment_reader.segment_id());
            continue;
        };
        let empirical = match empirical {
            Some(empirical) if is_date => match empirical.into_dates() {
                Some(empirical) => Some(empirical),
                None => {
                    partially_included.push(segment_reader.segment_id());
                    continue;
                }
            },
            other => other,
        };
        let (lower, upper) = (&range.lower, &range.upper);
        let inclusion = match (logical, empirical) {
            (None, None) => SegmentInclusion::PartiallyIncluded,
            (Some(bounds), None) => {
                let intersects = (range.includes_nulls && bounds.may_hold_nulls())
                    || bounds.intersects(lower, upper);
                if !intersects {
                    SegmentInclusion::Excluded
                } else if (range.includes_nulls || !bounds.may_hold_nulls())
                    && bounds.is_subset_of(lower, upper)
                {
                    SegmentInclusion::FullyIncluded
                } else {
                    SegmentInclusion::PartiallyIncluded
                }
            }
            (None, Some(empirical)) => {
                let intersects = (range.includes_nulls && empirical.nullable)
                    || empirical.intersects(lower, upper);
                if !intersects {
                    SegmentInclusion::Excluded
                } else if (range.includes_nulls || !empirical.nullable)
                    && empirical.is_subset_of(lower, upper)
                {
                    SegmentInclusion::FullyIncluded
                } else {
                    SegmentInclusion::PartiallyIncluded
                }
            }
            // The box holds what the build routed here and the empirical range what the
            // segment holds now, so a partition has to reach both. The empirical range is
            // the tighter of the two once a partition cuts inside a box.
            (Some(bounds), Some(empirical)) => {
                let intersects = (range.includes_nulls && empirical.nullable)
                    || (bounds.intersects(lower, upper) && empirical.intersects(lower, upper));
                if !intersects {
                    SegmentInclusion::Excluded
                } else {
                    let logical_subset = (range.includes_nulls || !bounds.may_hold_nulls())
                        && bounds.is_subset_of(lower, upper);
                    let empirical_subset = (range.includes_nulls || !empirical.nullable)
                        && empirical.is_subset_of(lower, upper);
                    if logical_subset || empirical_subset {
                        SegmentInclusion::FullyIncluded
                    } else {
                        SegmentInclusion::PartiallyIncluded
                    }
                }
            }
        };

        match inclusion {
            SegmentInclusion::FullyIncluded => included.push(segment_reader.segment_id()),
            SegmentInclusion::PartiallyIncluded => {
                partially_included.push(segment_reader.segment_id())
            }
            SegmentInclusion::Excluded => pruned_count += 1,
        }
    }

    PartitionSegments {
        included,
        partially_included,
        pruned_count,
    }
}
