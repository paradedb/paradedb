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

//! Top-N sorting by a Postgres range column, matching Postgres' own `range_cmp`.
//!
//! Range columns are indexed as a tantivy JSON object (see
//! [`crate::schema::range::TantivyRange`]) with a fixed set of keys, each of which becomes its
//! own fast field column: `lower`, `upper`, `empty`, `lower_inclusive`, `upper_inclusive`,
//! `lower_unbounded`, `upper_unbounded`. This module reads those columns and assembles a
//! composite sort key whose natural `Ord` reproduces `range_cmp`:
//!
//! - empty ranges sort before every non-empty range
//! - an unbounded lower bound sorts before any finite lower bound
//! - on equal lower bound values, inclusive sorts before exclusive (`[5,…` < `(5,…`)
//! - an unbounded upper bound sorts after any finite upper bound
//! - on equal upper bound values, exclusive sorts before inclusive (`…,10)` < `…,10]`)
//!
//! A SQL `NULL` range yields `None`, which the surrounding
//! [`tantivy::collector::sort_key::ComparatorEnum`] places according to the query's
//! NULLS FIRST/LAST direction.

use tantivy::collector::sort_key::ComparatorEnum;
use tantivy::collector::sort_key::shared_threshold::SharedThresholdArcOpt;
use tantivy::collector::{SegmentSortKeyComputer, SortKeyComputer};
use tantivy::columnar::{Column, ColumnType, StrColumn};
use tantivy::fastfield::FastFieldReaders;
use tantivy::termdict::TermOrdinal;
use tantivy::{DocId, Score, SegmentReader, TantivyError};

/// Ranks for the discriminant components of [`RangeSortKey`]. Named rather than inlined because
/// the whole correctness argument of this module is that these values are in `range_cmp` order.
const EMPTY: u8 = 0;
const NON_EMPTY: u8 = 1;
const LOWER_UNBOUNDED: u8 = 0;
const LOWER_FINITE: u8 = 1;
const LOWER_INCLUSIVE: u8 = 0;
const LOWER_EXCLUSIVE: u8 = 1;
const UPPER_FINITE: u8 = 0;
const UPPER_UNBOUNDED: u8 = 1;
const UPPER_EXCLUSIVE: u8 = 0;
const UPPER_INCLUSIVE: u8 = 1;

/// The composite sort key for one range value.
///
/// Field order *is* the comparison order, and the derived lexicographic `Ord` is what makes this
/// equal to `range_cmp`. Do not reorder the fields.
///
/// `V` is the bound representation: [`TermOrdinal`]-or-`u64` while collecting within a single
/// segment, and order-preserving bytes once keys have to be compared across segments.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RangeSortKey<V: Ord> {
    empty: u8,
    lower_bounded: u8,
    lower: Option<V>,
    lower_inclusive: u8,
    upper_bounded: u8,
    upper: Option<V>,
    upper_inclusive: u8,
}

impl<V: Ord> RangeSortKey<V> {
    /// The key every empty range collapses to. Bound components are fixed so that all empty
    /// ranges compare equal to each other regardless of what the bound columns happen to hold.
    fn empty() -> Self {
        Self {
            empty: EMPTY,
            lower_bounded: LOWER_UNBOUNDED,
            lower: None,
            lower_inclusive: LOWER_INCLUSIVE,
            upper_bounded: UPPER_FINITE,
            upper: None,
            upper_inclusive: UPPER_EXCLUSIVE,
        }
    }
}

/// Sorts by a Postgres range column, per `range_cmp`.
///
/// `column_name` is the name of the range field itself; the JSON sub-paths are derived from it.
#[derive(Clone, Debug)]
pub struct SortByRange {
    column_name: String,
}

impl SortByRange {
    pub fn for_field(column_name: impl ToString) -> Self {
        SortByRange {
            column_name: column_name.to_string(),
        }
    }

    fn path(&self, key: &str) -> String {
        format!("{}.{key}", self.column_name)
    }

    /// Opens the numeric column behind one bound, or none when the segment holds no finite value
    /// for it. Errors when the column exists under a type this computer can't order, so a stale
    /// or foreign encoding surfaces as an error instead of a wrong sort.
    fn numeric_bound(
        &self,
        fast_fields: &FastFieldReaders,
        key: &str,
    ) -> tantivy::Result<Option<Column<u64>>> {
        const ACCEPTED: &[ColumnType] = &[ColumnType::I64, ColumnType::DateTime];
        let path = self.path(key);
        let mut column = None;
        for handle in fast_fields.dynamic_column_handles(&path)? {
            if !ACCEPTED.contains(&handle.column_type()) {
                return Err(TantivyError::SchemaError(format!(
                    "range bound `{path}` is stored as {:?}; reindex to sort by this column",
                    handle.column_type()
                )));
            }
            column = handle.open_u64_lenient()?;
        }
        Ok(column)
    }
}

impl SortKeyComputer for SortByRange {
    type SortKey = Option<RangeSortKey<Vec<u8>>>;
    type Child = SegmentSortByRange;
    type Comparator = ComparatorEnum;

    fn shared_threshold(
        &self,
    ) -> SharedThresholdArcOpt<
        <<Self as SortKeyComputer>::Child as SegmentSortKeyComputer>::SegmentSortKey,
    > {
        // Same caveat as `SortByString`/`SortByBytes`: a text-bounded range compares by
        // `TermOrdinal` within a segment, and those are not comparable across segments, so there
        // is nothing safe to publish as a shared threshold.
        None
    }

    fn segment_sort_key_computer(
        &self,
        segment_reader: &SegmentReader,
    ) -> tantivy::Result<Self::Child> {
        let fast_fields = segment_reader.fast_fields();
        let flag = |key: &str| fast_fields.column_opt::<bool>(&self.path(key));

        // `numrange` bounds are indexed as hex-encoded sortable decimal strings (see
        // `SortableDecimal`); every other range type indexes its bounds as a numeric or date
        // column. Probe for the string form first and fall back to the numeric form.
        //
        // The `u64_lenient` mapping is monotonic within one column type, and a range column's
        // subtype fixes that type: integer bounds are `I64`, and date/timestamp bounds are `I64`
        // microseconds on current indexes or `DateTime` on ones built before the switch to `I64`
        // storage. `created_by_version` is fixed per index, so every segment of one index agrees,
        // and the same value maps to the same `u64` everywhere.
        //
        // A segment holding no finite bounds at all resolves to no columns, which is harmless:
        // its keys are all unbounded or empty, and the bounded/unbounded discriminant is compared
        // first. A bound column of any other type is a different matter. Reading it as absent
        // would make every finite range in the segment tie, so the scan would return a wrong
        // order instead of falling back to Postgres. Refuse it instead.
        let bounds = match (
            fast_fields.str(&self.path("lower"))?,
            fast_fields.str(&self.path("upper"))?,
        ) {
            (None, None) => SegmentBounds::Numeric {
                lower: self.numeric_bound(fast_fields, "lower")?,
                upper: self.numeric_bound(fast_fields, "upper")?,
            },
            (lower, upper) => SegmentBounds::Text { lower, upper },
        };

        Ok(SegmentSortByRange {
            empty: flag("empty")?,
            lower_unbounded: flag("lower_unbounded")?,
            upper_unbounded: flag("upper_unbounded")?,
            lower_inclusive: flag("lower_inclusive")?,
            upper_inclusive: flag("upper_inclusive")?,
            bounds,
        })
    }
}

/// The bound columns of a range field, in whichever representation the range's subtype uses.
enum SegmentBounds {
    Numeric {
        lower: Option<Column<u64>>,
        upper: Option<Column<u64>>,
    },
    Text {
        lower: Option<StrColumn>,
        upper: Option<StrColumn>,
    },
}

pub struct SegmentSortByRange {
    empty: Option<Column<bool>>,
    lower_unbounded: Option<Column<bool>>,
    upper_unbounded: Option<Column<bool>>,
    lower_inclusive: Option<Column<bool>>,
    upper_inclusive: Option<Column<bool>>,
    bounds: SegmentBounds,
}

impl SegmentSortByRange {
    /// Resolve a within-segment bound ordinal into order-preserving bytes.
    ///
    /// For text bounds this is the term itself: the terms are hex-encoded sortable decimals, so
    /// byte order is numeric order. For numeric bounds the `u64` is already monotonic in the
    /// underlying value, so big-endian bytes preserve it.
    fn bound_bytes(&self, is_lower: bool, ord: u64) -> Option<Vec<u8>> {
        match &self.bounds {
            SegmentBounds::Numeric { .. } => Some(ord.to_be_bytes().to_vec()),
            SegmentBounds::Text { lower, upper } => {
                let column = if is_lower { lower } else { upper }.as_ref()?;
                let mut bytes = Vec::new();
                column
                    .dictionary()
                    .ord_to_term(ord as TermOrdinal, &mut bytes)
                    .ok()?;
                Some(bytes)
            }
        }
    }

    fn bound(&self, is_lower: bool, doc: DocId) -> Option<u64> {
        match &self.bounds {
            SegmentBounds::Numeric { lower, upper } => {
                if is_lower { lower } else { upper }.as_ref()?.first(doc)
            }
            SegmentBounds::Text { lower, upper } => if is_lower { lower } else { upper }
                .as_ref()?
                .ords()
                .first(doc),
        }
    }

    fn flag(column: &Option<Column<bool>>, doc: DocId) -> bool {
        column
            .as_ref()
            .and_then(|column| column.first(doc))
            .unwrap_or(false)
    }
}

impl SegmentSortKeyComputer for SegmentSortByRange {
    type SortKey = Option<RangeSortKey<Vec<u8>>>;
    type SegmentSortKey = Option<RangeSortKey<u64>>;
    type SegmentComparator = ComparatorEnum;

    fn segment_sort_key(&mut self, doc: DocId, _score: Score) -> Self::SegmentSortKey {
        // `empty` is written for every non-NULL range, so its absence is how a SQL NULL range is
        // distinguished from an `'empty'` one.
        let is_empty = self.empty.as_ref()?.first(doc)?;
        if is_empty {
            return Some(RangeSortKey::empty());
        }

        let lower_unbounded = Self::flag(&self.lower_unbounded, doc);
        let upper_unbounded = Self::flag(&self.upper_unbounded, doc);

        Some(RangeSortKey {
            empty: NON_EMPTY,
            lower_bounded: if lower_unbounded {
                LOWER_UNBOUNDED
            } else {
                LOWER_FINITE
            },
            lower: (!lower_unbounded).then(|| self.bound(true, doc)).flatten(),
            lower_inclusive: if Self::flag(&self.lower_inclusive, doc) {
                LOWER_INCLUSIVE
            } else {
                LOWER_EXCLUSIVE
            },
            upper_bounded: if upper_unbounded {
                UPPER_UNBOUNDED
            } else {
                UPPER_FINITE
            },
            upper: (!upper_unbounded).then(|| self.bound(false, doc)).flatten(),
            upper_inclusive: if Self::flag(&self.upper_inclusive, doc) {
                UPPER_INCLUSIVE
            } else {
                UPPER_EXCLUSIVE
            },
        })
    }

    fn convert_segment_sort_key(&self, sort_key: Self::SegmentSortKey) -> Self::SortKey {
        let sort_key = sort_key?;
        // Resolve each bound independently: `lower` and `upper` have their own term dictionaries.
        Some(RangeSortKey {
            empty: sort_key.empty,
            lower_bounded: sort_key.lower_bounded,
            lower: sort_key.lower.and_then(|ord| self.bound_bytes(true, ord)),
            lower_inclusive: sort_key.lower_inclusive,
            upper_bounded: sort_key.upper_bounded,
            upper: sort_key.upper.and_then(|ord| self.bound_bytes(false, ord)),
            upper_inclusive: sort_key.upper_inclusive,
        })
    }

    fn supports_bm25_pruning(&self) -> bool {
        false
    }

    fn bm25_pruning_threshold(
        &self,
        _threshold: &Self::SegmentSortKey,
        _segment_ord: tantivy::SegmentOrdinal,
        _threshold_ord: tantivy::SegmentOrdinal,
    ) -> Option<Score> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds the key a non-empty range with `u64` bounds would get.
    fn key(
        lower: Option<u64>,
        lower_inclusive: bool,
        upper: Option<u64>,
        upper_inclusive: bool,
    ) -> RangeSortKey<u64> {
        RangeSortKey {
            empty: NON_EMPTY,
            lower_bounded: if lower.is_none() {
                LOWER_UNBOUNDED
            } else {
                LOWER_FINITE
            },
            lower,
            lower_inclusive: if lower_inclusive {
                LOWER_INCLUSIVE
            } else {
                LOWER_EXCLUSIVE
            },
            upper_bounded: if upper.is_none() {
                UPPER_UNBOUNDED
            } else {
                UPPER_FINITE
            },
            upper,
            upper_inclusive: if upper_inclusive {
                UPPER_INCLUSIVE
            } else {
                UPPER_EXCLUSIVE
            },
        }
    }

    /// The ordering asserted here is the one Postgres produces for
    /// `SELECT unnest(ARRAY['empty','(,10)','(,)','[4,10)','[5,10)','[5,10]','[5,11)','[5,)']::numrange[]) ORDER BY 1`.
    #[test]
    fn matches_postgres_range_cmp_order() {
        let ascending = vec![
            RangeSortKey::empty(),               // 'empty'
            key(None, true, Some(10), false),    // (,10)
            key(None, true, None, false),        // (,)
            key(Some(4), true, Some(10), false), // [4,10)
            key(Some(5), true, Some(10), false), // [5,10)
            key(Some(5), true, Some(10), true),  // [5,10]
            key(Some(5), true, Some(11), false), // [5,11)
            key(Some(5), true, None, false),     // [5,)
        ];

        let mut sorted = ascending.clone();
        sorted.sort();
        assert_eq!(sorted, ascending);
    }

    #[test]
    fn inclusive_lower_sorts_before_exclusive_lower() {
        // `[5,10) < (5,10)`
        assert!(key(Some(5), true, Some(10), false) < key(Some(5), false, Some(10), false));
    }

    #[test]
    fn exclusive_upper_sorts_before_inclusive_upper() {
        // `[5,10) < [5,10]`
        assert!(key(Some(5), true, Some(10), false) < key(Some(5), true, Some(10), true));
    }
}
