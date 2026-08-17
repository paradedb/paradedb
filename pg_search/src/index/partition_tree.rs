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

//! Ephemeral routing structure for a physically partitioned index build.
//!
//! The leader of a `CREATE INDEX` builds one [`PartitionTree`] from a sample of the heap and
//! hands it to every parallel worker, so all workers slice the `partition_by` space at exactly
//! the same boundaries. The tree is never persisted: workers record the bounds of the segments
//! they write, and any later merge derives a fresh tree from that segment metadata.

use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter};
use std::net::Ipv6Addr;
use std::ops::Bound;

use serde::{Deserialize, Serialize};

use crate::api::FieldName;
use crate::postgres::datetime::PostgresDateTime;
use crate::postgres::pdb_owned_value::PdbOwnedValue;

/// One sampled row, holding one value per tree dimension in `partition_by` order.
pub type SampleRow = Vec<PdbOwnedValue>;

/// A KD-tree over the `partition_by` fields of an index. Leaves are numbered left to right, so
/// with a single dimension the leaf order is the value order and NULLs land in partition 0,
/// matching `RangePartitioning::partition_bounds`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartitionTree {
    dimensions: Vec<FieldName>,
    root: PartitionNode,
    num_partitions: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum PartitionNode {
    Leaf {
        partition: usize,
    },
    Split {
        dimension: usize,
        /// Rows with a value below this point (and NULLs) go to `lower`, all others to `upper`.
        #[serde(with = "tagged_value")]
        split_point: PdbOwnedValue,
        lower: Box<PartitionNode>,
        upper: Box<PartitionNode>,
    },
}

/// The interval a partition covers along one dimension. `lower` is inclusive, `upper` is
/// exclusive. An unbounded `lower` also admits NULLs.
#[derive(Debug, Clone, PartialEq)]
pub struct DimensionBounds {
    pub lower: Bound<PdbOwnedValue>,
    pub upper: Bound<PdbOwnedValue>,
}

impl PartitionTree {
    /// Builds a tree with (at most) `target_partitions` leaves of roughly equal row mass.
    ///
    /// Splits cycle through the dimensions in `partition_by` order, one per level, and skip a
    /// dimension that has a single distinct value inside the node. A node that cannot be split
    /// on any dimension becomes a leaf, so heavily duplicated data can yield fewer partitions
    /// than requested. Every `rows` entry must have exactly `dimensions.len()` values.
    pub fn build(
        dimensions: Vec<FieldName>,
        rows: Vec<SampleRow>,
        target_partitions: usize,
    ) -> Self {
        let ndims = dimensions.len();
        debug_assert!(rows.iter().all(|row| row.len() == ndims));

        let mut next_partition = 0;
        let root = if ndims == 0 {
            PartitionNode::leaf(&mut next_partition)
        } else {
            PartitionNode::build(
                rows,
                target_partitions.max(1),
                0,
                ndims,
                &mut next_partition,
            )
        };
        Self {
            dimensions,
            root,
            num_partitions: next_partition,
        }
    }

    pub fn dimensions(&self) -> &[FieldName] {
        &self.dimensions
    }

    pub fn num_partitions(&self) -> usize {
        self.num_partitions
    }

    /// Returns the partition a row belongs to. `values` must be in dimension order.
    // The partitioned index writer routes every tuple through this; nothing calls it yet.
    #[allow(dead_code)]
    pub fn route(&self, values: &[PdbOwnedValue]) -> usize {
        debug_assert_eq!(values.len(), self.dimensions.len());
        let mut node = &self.root;
        loop {
            match node {
                PartitionNode::Leaf { partition } => return *partition,
                PartitionNode::Split {
                    dimension,
                    split_point,
                    lower,
                    upper,
                } => {
                    node = if goes_lower(&values[*dimension], split_point) {
                        lower
                    } else {
                        upper
                    };
                }
            }
        }
    }

    /// The hyper-rectangle covered by `partition`, one entry per dimension, or `None` for an
    /// unknown partition number.
    pub fn partition_bounds(&self, partition: usize) -> Option<Vec<DimensionBounds>> {
        let mut bounds = vec![
            DimensionBounds {
                lower: Bound::Unbounded,
                upper: Bound::Unbounded,
            };
            self.dimensions.len()
        ];
        let mut node = &self.root;
        loop {
            match node {
                PartitionNode::Leaf { partition: p } => {
                    return (*p == partition).then_some(bounds);
                }
                PartitionNode::Split {
                    dimension,
                    split_point,
                    lower,
                    upper,
                } => {
                    // Leaves are numbered in order, so the leaf we want is on the lower side iff
                    // its number is below the first leaf number of the upper subtree.
                    if partition < upper.first_partition() {
                        bounds[*dimension].upper = Bound::Excluded(split_point.clone());
                        node = lower;
                    } else {
                        bounds[*dimension].lower = Bound::Included(split_point.clone());
                        node = upper;
                    }
                }
            }
        }
    }
}

impl PartitionNode {
    fn leaf(next_partition: &mut usize) -> Self {
        let partition = *next_partition;
        *next_partition += 1;
        PartitionNode::Leaf { partition }
    }

    fn first_partition(&self) -> usize {
        let mut node = self;
        loop {
            match node {
                PartitionNode::Leaf { partition } => return *partition,
                PartitionNode::Split { lower, .. } => node = lower,
            }
        }
    }

    fn build(
        mut rows: Vec<SampleRow>,
        target: usize,
        depth: usize,
        ndims: usize,
        next_partition: &mut usize,
    ) -> Self {
        if target <= 1 || rows.len() < 2 {
            return Self::leaf(next_partition);
        }

        // Aim the split at the quantile that gives each child its share of the leaves, so an
        // odd target still ends in equal-mass partitions.
        let lower_target = target / 2;
        let upper_target = target - lower_target;

        for offset in 0..ndims {
            let dimension = (depth + offset) % ndims;
            rows.sort_by(|a, b| compare_values(&a[dimension], &b[dimension]));

            let Some(split_idx) = choose_split_index(&rows, dimension, lower_target, target) else {
                continue;
            };

            let split_point = rows[split_idx][dimension].clone();
            let upper_rows = rows.split_off(split_idx);
            let lower = Self::build(rows, lower_target, depth + 1, ndims, next_partition);
            let upper = Self::build(upper_rows, upper_target, depth + 1, ndims, next_partition);
            return PartitionNode::Split {
                dimension,
                split_point,
                lower: Box::new(lower),
                upper: Box::new(upper),
            };
        }

        Self::leaf(next_partition)
    }
}

/// Picks the index at which `rows` (sorted on `dimension`) is cut so the value at that index
/// becomes the split point. Both sides must be non-empty and every row equal to the split point
/// must sit on the upper side, so the cut snaps to the nearest run boundary around the target
/// quantile. Returns `None` when the dimension has one distinct value (NULLs count as one).
fn choose_split_index(
    rows: &[SampleRow],
    dimension: usize,
    lower_target: usize,
    target: usize,
) -> Option<usize> {
    let n = rows.len();
    let k = (n * lower_target / target).clamp(1, n - 1);
    let at = |i: usize| &rows[i][dimension];

    let run_start = (0..k)
        .rev()
        .find(|&i| compare_values(at(i), at(k)) != Ordering::Equal)
        .map(|i| i + 1)
        .unwrap_or(0);
    let run_end = (k..n).find(|&i| compare_values(at(i), at(k)) != Ordering::Equal);

    // A split point can never be NULL: NULLs sort first, so a cut inside the NULL run has
    // `run_start == 0` and only the run end (the first non-NULL value) is a valid cut.
    match (run_start > 0, run_end) {
        (true, Some(end)) if k - run_start <= end - k => Some(run_start),
        (_, Some(end)) => Some(end),
        (true, None) => Some(run_start),
        (false, None) => None,
    }
}

fn goes_lower(value: &PdbOwnedValue, split_point: &PdbOwnedValue) -> bool {
    matches!(value, PdbOwnedValue::Null) || compare_values(value, split_point) == Ordering::Less
}

/// Total order over sample values: NULLs first, then the value order of the field type. Uses
/// `total_cmp` for floats so NaNs sort deterministically instead of comparing as equal.
pub fn compare_values(a: &PdbOwnedValue, b: &PdbOwnedValue) -> Ordering {
    match (a, b) {
        (PdbOwnedValue::F64(x), PdbOwnedValue::F64(y)) => x.total_cmp(y),
        _ => a.partial_cmp(b).unwrap_or(Ordering::Equal),
    }
}

impl Display for PartitionTree {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for partition in 0..self.num_partitions {
            let bounds = self
                .partition_bounds(partition)
                .expect("partition numbers are contiguous");
            write!(f, "partition {partition}:")?;
            for (dimension, DimensionBounds { lower, upper }) in self.dimensions.iter().zip(bounds)
            {
                write!(f, " {dimension}=")?;
                match lower {
                    Bound::Included(v) => write!(f, "[{}", DisplayValue(&v))?,
                    _ => write!(f, "[..")?,
                }
                match upper {
                    Bound::Excluded(v) => write!(f, ", {})", DisplayValue(&v))?,
                    _ => write!(f, ", ..)")?,
                }
            }
            if partition + 1 < self.num_partitions {
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

struct DisplayValue<'a>(&'a PdbOwnedValue);

impl Display for DisplayValue<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.0 {
            PdbOwnedValue::Str(s) => write!(f, "{s:?}"),
            PdbOwnedValue::U64(v) => write!(f, "{v}"),
            PdbOwnedValue::I64(v) => write!(f, "{v}"),
            PdbOwnedValue::F64(v) => write!(f, "{v}"),
            PdbOwnedValue::Bool(v) => write!(f, "{v}"),
            PdbOwnedValue::Date(d) => write!(f, "{}us", d.into_inner()),
            PdbOwnedValue::IpAddr(ip) => write!(f, "{ip}"),
            other => write!(f, "{other:?}"),
        }
    }
}

/// `PdbOwnedValue`'s own serde form goes through tantivy's `OwnedValue`, which cannot tell a
/// non-negative `I64` from a `U64` on the way back in. Split points must survive the trip to
/// the workers exactly, so they are serialized with an explicit variant tag instead.
mod tagged_value {
    use super::*;

    #[derive(Serialize, Deserialize)]
    enum TaggedValue {
        Str(String),
        U64(u64),
        I64(i64),
        F64(f64),
        Bool(bool),
        Date(i64),
        Bytes(Vec<u8>),
        IpAddr(Ipv6Addr),
    }

    pub fn serialize<S: serde::Serializer>(
        value: &PdbOwnedValue,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let tagged = match value {
            PdbOwnedValue::Str(s) => TaggedValue::Str(s.clone()),
            PdbOwnedValue::U64(v) => TaggedValue::U64(*v),
            PdbOwnedValue::I64(v) => TaggedValue::I64(*v),
            PdbOwnedValue::F64(v) => TaggedValue::F64(*v),
            PdbOwnedValue::Bool(v) => TaggedValue::Bool(*v),
            PdbOwnedValue::Date(d) => TaggedValue::Date(d.into_inner()),
            PdbOwnedValue::Bytes(b) => TaggedValue::Bytes(b.clone()),
            PdbOwnedValue::IpAddr(ip) => TaggedValue::IpAddr(*ip),
            other => {
                return Err(serde::ser::Error::custom(format!(
                    "unsupported partition split point: {other:?}"
                )));
            }
        };
        tagged.serialize(serializer)
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<PdbOwnedValue, D::Error> {
        Ok(match TaggedValue::deserialize(deserializer)? {
            TaggedValue::Str(s) => PdbOwnedValue::Str(s),
            TaggedValue::U64(v) => PdbOwnedValue::U64(v),
            TaggedValue::I64(v) => PdbOwnedValue::I64(v),
            TaggedValue::F64(v) => PdbOwnedValue::F64(v),
            TaggedValue::Bool(v) => PdbOwnedValue::Bool(v),
            TaggedValue::Date(raw) => PdbOwnedValue::Date(
                PostgresDateTime::try_from_raw(raw).map_err(serde::de::Error::custom)?,
            ),
            TaggedValue::Bytes(b) => PdbOwnedValue::Bytes(b),
            TaggedValue::IpAddr(ip) => PdbOwnedValue::IpAddr(ip),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dims(names: &[&str]) -> Vec<FieldName> {
        names
            .iter()
            .map(|n| FieldName::from(n.to_string()))
            .collect()
    }

    fn i64_rows(values: impl IntoIterator<Item = i64>) -> Vec<SampleRow> {
        values
            .into_iter()
            .map(|v| vec![PdbOwnedValue::I64(v)])
            .collect()
    }

    fn contains(bounds: &DimensionBounds, value: &PdbOwnedValue) -> bool {
        if matches!(value, PdbOwnedValue::Null) {
            return matches!(bounds.lower, Bound::Unbounded);
        }
        let above_lower = match &bounds.lower {
            Bound::Unbounded => true,
            Bound::Included(lo) => compare_values(value, lo) != Ordering::Less,
            Bound::Excluded(_) => unreachable!(),
        };
        let below_upper = match &bounds.upper {
            Bound::Unbounded => true,
            Bound::Excluded(hi) => compare_values(value, hi) == Ordering::Less,
            Bound::Included(_) => unreachable!(),
        };
        above_lower && below_upper
    }

    /// Every row must route to the leaf whose bounds contain it, and the leaf mass must be
    /// within `tolerance` rows of even.
    fn check_tree(tree: &PartitionTree, rows: &[SampleRow], tolerance: usize) {
        let mut counts = vec![0usize; tree.num_partitions()];
        for row in rows {
            let partition = tree.route(row);
            counts[partition] += 1;
            let bounds = tree.partition_bounds(partition).unwrap();
            for (dim, value) in row.iter().enumerate() {
                assert!(
                    contains(&bounds[dim], value),
                    "row {row:?} routed to partition {partition} with bounds {bounds:?}"
                );
            }
        }
        let expected = rows.len() / tree.num_partitions();
        for (partition, count) in counts.iter().enumerate() {
            assert!(
                count.abs_diff(expected) <= tolerance,
                "partition {partition} holds {count} rows, expected about {expected}"
            );
        }
        assert!(tree.partition_bounds(tree.num_partitions()).is_none());
    }

    #[test]
    fn one_dimension_matches_value_order() {
        let rows = i64_rows(0..1000);
        let tree = PartitionTree::build(dims(&["id"]), rows.clone(), 8);
        assert_eq!(tree.num_partitions(), 8);
        check_tree(&tree, &rows, 0);
        for (i, row) in rows.iter().enumerate() {
            assert_eq!(tree.route(row), i / 125);
        }
        assert_eq!(
            tree.partition_bounds(3).unwrap()[0],
            DimensionBounds {
                lower: Bound::Included(PdbOwnedValue::I64(375)),
                upper: Bound::Excluded(PdbOwnedValue::I64(500)),
            }
        );
    }

    #[test]
    fn odd_target_still_balances() {
        let rows = i64_rows(0..900);
        let tree = PartitionTree::build(dims(&["id"]), rows.clone(), 3);
        assert_eq!(tree.num_partitions(), 3);
        check_tree(&tree, &rows, 1);
    }

    #[test]
    fn two_dimensions_alternate_and_balance() {
        let rows: Vec<SampleRow> = (0..1024i64)
            .map(|i| vec![PdbOwnedValue::I64(i), PdbOwnedValue::I64((i * 7919) % 1024)])
            .collect();
        let tree = PartitionTree::build(dims(&["a", "b"]), rows.clone(), 8);
        assert_eq!(tree.num_partitions(), 8);
        check_tree(&tree, &rows, 8);

        // The root splits `a` at its median, so partitions 0..4 sit below it and 4..8 at or
        // above it (the third level refines `a` further), and the second level bounds `b` on
        // at least one side in every partition.
        let root_split = PdbOwnedValue::I64(512);
        let bounds: Vec<_> = (0..8).map(|p| tree.partition_bounds(p).unwrap()).collect();
        assert!(bounds[..4].iter().all(|b| match &b[0].upper {
            Bound::Excluded(v) => compare_values(v, &root_split) != Ordering::Greater,
            _ => false,
        }));
        assert!(bounds[4..].iter().all(|b| match &b[0].lower {
            Bound::Included(v) => compare_values(v, &root_split) != Ordering::Less,
            _ => false,
        }));
        assert!(bounds.iter().all(|b| b[1]
            != DimensionBounds {
                lower: Bound::Unbounded,
                upper: Bound::Unbounded
            }));
    }

    #[test]
    fn nulls_route_to_the_lowest_partition() {
        let mut rows = i64_rows(0..100);
        rows.extend((0..10).map(|_| vec![PdbOwnedValue::Null]));
        let tree = PartitionTree::build(dims(&["id"]), rows.clone(), 4);
        assert_eq!(tree.num_partitions(), 4);
        assert_eq!(tree.route(&[PdbOwnedValue::Null]), 0);
        check_tree(&tree, &rows, 4);
        // A NULL is never chosen as a split point.
        for p in 0..4 {
            let b = &tree.partition_bounds(p).unwrap()[0];
            assert!(!matches!(b.lower, Bound::Included(PdbOwnedValue::Null)));
            assert!(!matches!(b.upper, Bound::Excluded(PdbOwnedValue::Null)));
        }
    }

    #[test]
    fn duplicates_stay_on_the_upper_side() {
        // 90 rows of value 1, 10 rows of value 2: the only cut is between the runs.
        let mut rows = i64_rows(std::iter::repeat_n(1, 90));
        rows.extend(i64_rows(std::iter::repeat_n(2, 10)));
        let tree = PartitionTree::build(dims(&["id"]), rows.clone(), 4);
        assert_eq!(tree.num_partitions(), 2);
        assert_eq!(tree.route(&[PdbOwnedValue::I64(1)]), 0);
        assert_eq!(tree.route(&[PdbOwnedValue::I64(2)]), 1);
        assert_eq!(tree.route(&[PdbOwnedValue::I64(3)]), 1);
        assert_eq!(tree.route(&[PdbOwnedValue::I64(0)]), 0);
    }

    #[test]
    fn unsplittable_dimension_is_skipped() {
        let rows: Vec<SampleRow> = (0..100i64)
            .map(|i| vec![PdbOwnedValue::Bool(true), PdbOwnedValue::I64(i)])
            .collect();
        let tree = PartitionTree::build(dims(&["flag", "id"]), rows.clone(), 4);
        assert_eq!(tree.num_partitions(), 4);
        check_tree(&tree, &rows, 0);
        for p in 0..4 {
            let bounds = tree.partition_bounds(p).unwrap();
            assert_eq!(bounds[0].lower, Bound::Unbounded);
            assert_eq!(bounds[0].upper, Bound::Unbounded);
        }
    }

    #[test]
    fn degenerate_inputs_give_one_partition() {
        for rows in [vec![], i64_rows([7]), i64_rows(std::iter::repeat_n(7, 50))] {
            let tree = PartitionTree::build(dims(&["id"]), rows, 8);
            assert_eq!(tree.num_partitions(), 1);
            assert_eq!(tree.route(&[PdbOwnedValue::I64(1)]), 0);
        }
        let tree = PartitionTree::build(vec![], vec![vec![], vec![]], 8);
        assert_eq!(tree.num_partitions(), 1);
        let tree = PartitionTree::build(dims(&["id"]), i64_rows(0..100), 0);
        assert_eq!(tree.num_partitions(), 1);
    }

    #[test]
    fn strings_and_floats_order_naturally() {
        let rows: Vec<SampleRow> = ["apple", "banana", "cherry", "date", "elder", "fig"]
            .iter()
            .enumerate()
            .map(|(i, s)| {
                vec![
                    PdbOwnedValue::Str(s.to_string()),
                    PdbOwnedValue::F64(i as f64),
                ]
            })
            .collect();
        let tree = PartitionTree::build(dims(&["name", "score"]), rows.clone(), 2);
        assert_eq!(tree.num_partitions(), 2);
        assert_eq!(
            tree.route(&[
                PdbOwnedValue::Str("aardvark".into()),
                PdbOwnedValue::F64(9.0)
            ]),
            0
        );
        assert_eq!(
            tree.route(&[PdbOwnedValue::Str("zebra".into()), PdbOwnedValue::F64(0.0)]),
            1
        );
        check_tree(&tree, &rows, 0);
    }

    #[test]
    fn json_roundtrip_keeps_signed_split_points() {
        let rows: Vec<SampleRow> = (0..64i64)
            .map(|i| vec![PdbOwnedValue::I64(i), PdbOwnedValue::U64(i as u64 * 3)])
            .collect();
        let tree = PartitionTree::build(dims(&["a", "b"]), rows.clone(), 8);
        let bytes = serde_json::to_vec(&Some(tree.clone())).unwrap();
        let back: Option<PartitionTree> = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back.as_ref(), Some(&tree));
        for row in &rows {
            assert_eq!(back.as_ref().unwrap().route(row), tree.route(row));
        }
        let none: Option<PartitionTree> = serde_json::from_slice(b"null").unwrap();
        assert!(none.is_none());
    }

    #[test]
    fn display_lists_every_partition() {
        let tree = PartitionTree::build(dims(&["id"]), i64_rows(0..100), 4);
        let text = tree.to_string();
        assert_eq!(text.lines().count(), 4);
        assert!(text.starts_with("partition 0: id=[.., 25)"));
        assert!(text.ends_with("partition 3: id=[75, ..)"));
    }
}
