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

//! Recursive partitioning of the space spanned by an index's `partition_by` fields.
//!
//! A [`KdTree`] is an ephemeral routing structure. `CREATE INDEX` builds one on the leader from
//! a sample of the heap and hands it to every parallel worker, so that all workers cut their
//! output segments on the same global boundaries. It is never persisted: workers record the
//! bounding box of each segment they write, and later merges rebuild a routing tree from that
//! per-segment metadata.

use std::cmp::Ordering;
use std::fmt;
use std::ops::Bound;

use serde::{Deserialize, Serialize};

use crate::api::FieldName;
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::scan::range_partitioning::RangePartitioning;

/// One row projected onto the `partition_by` fields, in the same order as [`KdTree::dims`].
pub type Point = Vec<PdbOwnedValue>;

/// A binary tree of single-dimension splits whose leaves are the partitions.
///
/// Routing rules, shared with [`RangePartitioning::partition_bounds`] so that a one-dimensional
/// tree and a `RangePartitioning` over the same split points agree on every row:
///
/// - at a split on `dim` with value `v`, a row goes left when its value is NULL or `< v`,
///   and right when it is `>= v`;
/// - partitions are numbered in left-to-right (in-order) leaf order, so partition 0 holds
///   the NULLs of the dimension split at the root.
///
/// Every leaf therefore covers a half-open bounding box, lower-inclusive and upper-exclusive
/// on each dimension it was split on, and unbounded on the rest.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KdTree {
    dims: Vec<FieldName>,
    root: KdNode,
    partitions: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum KdNode {
    Leaf {
        partition: usize,
    },
    Split {
        dim: usize,
        #[serde(with = "crate::postgres::pdb_owned_value::exact_scalar_wire")]
        value: PdbOwnedValue,
        left: Box<KdNode>,
        right: Box<KdNode>,
    },
}

/// The half-open range a partition covers on one dimension.
pub type DimBounds = (Bound<PdbOwnedValue>, Bound<PdbOwnedValue>);

impl KdTree {
    /// A tree with a single partition that every row routes to.
    pub fn unpartitioned(dims: Vec<FieldName>) -> Self {
        Self {
            dims,
            root: KdNode::Leaf { partition: 0 },
            partitions: 1,
        }
    }

    /// Builds a tree with at most `target_partitions` leaves from a sample of the data.
    ///
    /// The cuts follow the order of `dims`. The first dimension is cut into ranges that span
    /// every value of the later dimensions, each of those ranges is then cut on the second
    /// dimension, and so on. A dimension's ranges inside one box are therefore disjoint
    /// intervals of it, and the first dimension's ranges are disjoint over the whole tree, so a
    /// one-dimensional partitioning of the first field lines up with whole leaves. A later
    /// dimension is cut independently inside each range of the ones before it, so its intervals
    /// overlap across those ranges.
    ///
    /// `ranges[dim]` is the most ranges that dimension is cut into inside each box of the
    /// dimensions before it, so counts on every dimension cap the leaf count at their product.
    /// A `None` shares the leaves left over by the dimensions with a count evenly, in levels of
    /// binary cuts, between that dimension and the later ones without a count, rounding up in
    /// favor of the earlier dimension. The cut sits at the quantile that gives both children a
    /// share of the sample proportional to the leaves they will hold, and the leaves then follow
    /// the rows that landed on each side, so they come out balanced even when a count is not a
    /// power of two or the cut had to move to a value change, at the cost of ranges of unequal
    /// width.
    ///
    /// A box whose points are all equal on a dimension cannot be cut on it and passes its leaves
    /// on to the next dimension; a box that is a single point on every remaining dimension stays
    /// a leaf. The tree can therefore end up with fewer partitions than requested when the
    /// sample has too few distinct values, and a count on an earlier dimension is never exceeded
    /// to make up for a later one that ran out.
    ///
    /// Split values are never NULL, so every leaf's box is expressible as plain range bounds.
    pub fn from_sample(
        dims: Vec<FieldName>,
        ranges: &[Option<usize>],
        sample: Vec<Point>,
        target_partitions: usize,
    ) -> Self {
        let ndims = dims.len();
        debug_assert_eq!(ranges.len(), ndims, "one range count slot per dimension");
        debug_assert!(
            ranges.iter().flatten().all(|&n| n > 0),
            "a range count must be positive"
        );
        debug_assert!(
            sample.iter().all(|p| p.len() == ndims),
            "every sample point must have one value per partition_by field"
        );
        if target_partitions <= 1 || sample.len() < 2 || ndims == 0 {
            return Self::unpartitioned(dims);
        }

        let mut idx: Vec<usize> = (0..sample.len()).collect();
        let mut builder = Builder {
            sample: &sample,
            ranges,
            next_partition: 0,
        };
        let root = builder.build(&mut idx, target_partitions, 0);
        Self {
            dims,
            root,
            partitions: builder.next_partition,
        }
    }

    /// The `partition_by` fields, in the order [`Point`]s must be laid out.
    pub fn dims(&self) -> &[FieldName] {
        &self.dims
    }

    pub fn partition_count(&self) -> usize {
        self.partitions
    }

    /// The partition a row belongs to. `values` must be laid out like [`Self::dims`].
    pub fn route(&self, values: &[PdbOwnedValue]) -> usize {
        debug_assert_eq!(values.len(), self.dims.len());
        let mut node = &self.root;
        loop {
            match node {
                KdNode::Leaf { partition } => return *partition,
                KdNode::Split {
                    dim,
                    value,
                    left,
                    right,
                } => {
                    node = if goes_left(&values[*dim], value) {
                        left
                    } else {
                        right
                    };
                }
            }
        }
    }

    /// The bounding box of `partition`, one half-open range per dimension of [`Self::dims`].
    ///
    /// Returns `None` when `partition` is out of range.
    pub fn partition_bounds(&self, partition: usize) -> Option<Vec<DimBounds>> {
        let mut bounds = vec![(Bound::Unbounded, Bound::Unbounded); self.dims.len()];
        let mut node = &self.root;
        loop {
            match node {
                KdNode::Leaf { partition: p } => {
                    return (*p == partition).then_some(bounds);
                }
                KdNode::Split {
                    dim,
                    value,
                    left,
                    right,
                } => {
                    // Leaves are numbered in-order, so the highest partition in the left
                    // subtree is one below the lowest in the right subtree.
                    if partition < first_partition(right) {
                        bounds[*dim].1 = Bound::Excluded(value.clone());
                        node = left;
                    } else {
                        bounds[*dim].0 = Bound::Included(value.clone());
                        node = right;
                    }
                }
            }
        }
    }

    /// One line per partition with its bounding box, for logs: `partition 3: id=[150, 300)
    /// tenant_id=[.., 42)`. [`Display`](fmt::Display) shows the same tree by its splits instead.
    pub fn bounds_listing(&self) -> impl fmt::Display + '_ {
        BoundsListing(self)
    }

    /// For a tree over a single field, the equivalent [`RangePartitioning`]: its split points
    /// are this tree's split values in ascending order, and both route every row identically.
    #[allow(dead_code)]
    pub fn to_range_partitioning(&self) -> Option<RangePartitioning> {
        if self.dims.len() != 1 {
            return None;
        }
        let mut split_points = Vec::with_capacity(self.partitions.saturating_sub(1));
        collect_split_values(&self.root, &mut split_points);
        Some(RangePartitioning {
            partition_by: self.dims[0].clone(),
            split_points,
        })
    }
}

impl fmt::Display for KdTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let dims = self
            .dims
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "partition_by=[{dims}], partitions={}", self.partitions)?;
        fmt_node(&self.root, self, 0, f)
    }
}

struct DimValueDisplay<'a> {
    dim: &'a FieldName,
    value: &'a PdbOwnedValue,
}

impl fmt::Display for DimValueDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.dim.is_ctid()
            && let PdbOwnedValue::U64(v) = self.value
        {
            return write!(f, "{}", crate::postgres::utils::format_u64_ctid(*v));
        }
        write!(f, "{}", self.value.plain_display())
    }
}

struct BoundsListing<'a>(&'a KdTree);

impl fmt::Display for BoundsListing<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tree = self.0;
        for partition in 0..tree.partitions {
            if partition > 0 {
                writeln!(f)?;
            }
            write!(f, "partition {partition}:")?;
            let bounds = tree
                .partition_bounds(partition)
                .expect("partitions are numbered contiguously");
            for (dim, (lower, upper)) in tree.dims.iter().zip(bounds) {
                match lower {
                    Bound::Included(ref v) => {
                        write!(f, " {dim}=[{}", DimValueDisplay { dim, value: v })?
                    }
                    _ => write!(f, " {dim}=[..")?,
                }
                match upper {
                    Bound::Excluded(ref v) => {
                        write!(f, ", {})", DimValueDisplay { dim, value: v })?
                    }
                    _ => write!(f, ", ..)")?,
                }
            }
        }
        Ok(())
    }
}

fn fmt_node(node: &KdNode, tree: &KdTree, depth: usize, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match node {
        KdNode::Leaf { partition } => write!(f, " -> {partition}"),
        KdNode::Split {
            dim,
            value,
            left,
            right,
        } => {
            let indent = "  ".repeat(depth);
            let dim = &tree.dims[*dim];
            write!(f, "\n{indent}{dim} < {}", DimValueDisplay { dim, value })?;
            fmt_node(left, tree, depth + 1, f)?;
            write!(f, "\n{indent}{dim} >= {}", DimValueDisplay { dim, value })?;
            fmt_node(right, tree, depth + 1, f)
        }
    }
}

#[allow(dead_code)] // reached through `route`
fn goes_left(value: &PdbOwnedValue, split: &PdbOwnedValue) -> bool {
    matches!(value, PdbOwnedValue::Null) || value.total_cmp(split) == Ordering::Less
}

fn first_partition(mut node: &KdNode) -> usize {
    loop {
        match node {
            KdNode::Leaf { partition } => return *partition,
            KdNode::Split { left, .. } => node = left,
        }
    }
}

#[allow(dead_code)] // reached through `to_range_partitioning`
fn collect_split_values(node: &KdNode, out: &mut Vec<PdbOwnedValue>) {
    if let KdNode::Split {
        value, left, right, ..
    } = node
    {
        collect_split_values(left, out);
        out.push(value.clone());
        collect_split_values(right, out);
    }
}

/// The ranges a dimension without a count is cut into inside a box that has `budget` leaves
/// to fill and `sharing` dimensions (this one included) still to cut. The levels of binary
/// cuts the budget affords are shared evenly, rounding up for this dimension, so the earlier
/// field takes the larger share and the later ones divide what is left.
fn default_ranges(budget: usize, sharing: usize) -> usize {
    if sharing <= 1 || budget <= 1 {
        return budget;
    }
    let levels = budget.next_power_of_two().trailing_zeros() as usize;
    (1usize << levels.div_ceil(sharing)).min(budget)
}

struct Builder<'a> {
    sample: &'a [Point],
    ranges: &'a [Option<usize>],
    next_partition: usize,
}

impl Builder<'_> {
    fn leaf(&mut self) -> KdNode {
        let partition = self.next_partition;
        self.next_partition += 1;
        KdNode::Leaf { partition }
    }

    /// Fills the box holding the points in `idx` with at most `k` leaves, cutting on `dim` and
    /// the dimensions after it.
    fn build(&mut self, idx: &mut [usize], k: usize, dim: usize) -> KdNode {
        // Counts on every remaining dimension cap the leaves at their product. Capping before
        // the cut keeps the leaves a count does not use from passing to a sibling as if the
        // box had run out of distinct values.
        let k = self.max_leaves_from(dim).map_or(k, |max| k.min(max));
        if k <= 1 || idx.len() < 2 || dim >= self.ranges.len() {
            return self.leaf();
        }
        let ranges = self.ranges_for(dim, k);
        self.cut(idx, k, dim, ranges).0
    }

    /// The most leaves a box can hold from the cuts on `dim` and the dimensions after it, or
    /// `None` when one of them has no count.
    fn max_leaves_from(&self, dim: usize) -> Option<usize> {
        self.ranges[dim.min(self.ranges.len())..]
            .iter()
            .copied()
            .product()
    }

    /// Cuts the box on `dim` into at most `ranges` ranges, `ranges <= k`, and hands each range
    /// with its share of the `k` leaves to the next dimension. Returns the node and the number
    /// of ranges of `dim` it made.
    fn cut(&mut self, idx: &mut [usize], k: usize, dim: usize, ranges: usize) -> (KdNode, usize) {
        debug_assert!(ranges <= k, "a box holds at least one leaf per range");
        if ranges <= 1 {
            return (self.build(idx, k, dim + 1), 1);
        }
        if idx.len() < 2 {
            return (self.leaf(), 1);
        }
        idx.sort_by(|&a, &b| self.sample[a][dim].total_cmp(&self.sample[b][dim]));

        let ranges_left = ranges / 2;
        let target = idx.len() * (k * ranges_left / ranges) / k;
        let Some(cut) = self.nearest_cut(idx, dim, target) else {
            // Every point is equal on this dimension, so the later ones take the whole budget.
            return (self.build(idx, k, dim + 1), 1);
        };
        let value = self.sample[idx[cut]][dim].clone();

        // The cut sits where the value changes, which can be far from the target on a field
        // with few distinct values, so the leaves follow the rows that landed on each side.
        // Each side keeps at least one leaf per range it has to make.
        let k_left = ((2 * k * cut + idx.len()) / (2 * idx.len()))
            .clamp(ranges_left, k - (ranges - ranges_left));

        let (left_idx, right_idx) = idx.split_at_mut(cut);
        let before = self.next_partition;
        let (left, made_left) = self.cut(left_idx, k_left, dim, ranges_left);
        let produced_left = self.next_partition - before;
        // A left subtree that ran out of distinct values hands its unused leaves to the right.
        // Without a count the ranges follow the leaves, which keeps a one-field tree at its
        // target. With a count the right gets only the ranges the left could not make, so the
        // field never exceeds the count when a later field is the one that ran out.
        let k_right = k - produced_left;
        let ranges_right = match self.ranges[dim] {
            Some(_) => (ranges - made_left).min(k_right),
            None => ranges - ranges_left + (k_left - produced_left),
        };
        let (right, made_right) = self.cut(right_idx, k_right, dim, ranges_right);
        (
            KdNode::Split {
                dim,
                value,
                left: Box::new(left),
                right: Box::new(right),
            },
            made_left + made_right,
        )
    }

    /// How many ranges `dim` is cut into inside a box with `k` leaves to fill: the index's
    /// count when it gave one, otherwise a share of the leaves the later dimensions with a
    /// count leave over.
    fn ranges_for(&self, dim: usize, k: usize) -> usize {
        if let Some(n) = self.ranges[dim] {
            return n.min(k);
        }
        let later = &self.ranges[dim + 1..];
        let counted_later: usize = later.iter().flatten().product();
        let uncounted_later = later.iter().filter(|n| n.is_none()).count();
        default_ranges((k / counted_later).max(1), uncounted_later + 1)
    }

    /// The position in `idx` (sorted on `dim`) closest to `target` where the value changes,
    /// so that both sides of the cut are non-empty and the split value is the smallest value
    /// on the right. NULLs sort first and are all equal, so the value at a cut is never NULL.
    fn nearest_cut(&self, idx: &[usize], dim: usize, target: usize) -> Option<usize> {
        let changes = |pos: usize| {
            self.sample[idx[pos - 1]][dim].total_cmp(&self.sample[idx[pos]][dim]) != Ordering::Equal
        };
        let target = target.clamp(1, idx.len() - 1);
        let below = (1..=target).rev().find(|&pos| changes(pos));
        let above = (target + 1..idx.len()).find(|&pos| changes(pos));
        match (below, above) {
            (Some(b), Some(a)) => Some(if target - b <= a - target { b } else { a }),
            (Some(b), None) => Some(b),
            (None, Some(a)) => Some(a),
            (None, None) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postgres::datetime::PostgresDateTime;

    fn dims(names: &[&str]) -> Vec<FieldName> {
        names
            .iter()
            .map(|n| FieldName::from(n.to_string()))
            .collect()
    }

    fn i64s(values: impl IntoIterator<Item = i64>) -> Vec<Point> {
        values
            .into_iter()
            .map(|v| vec![PdbOwnedValue::I64(v)])
            .collect()
    }

    /// A 40x40 grid: both dimensions have identical, independent marginals.
    fn grid() -> Vec<Point> {
        let mut sample = Vec::new();
        for x in 0..40 {
            for y in 0..40 {
                sample.push(vec![PdbOwnedValue::I64(x), PdbOwnedValue::I64(y)]);
            }
        }
        sample
    }

    /// A tree that leaves every range count to the builder.
    fn build(names: &[&str], sample: Vec<Point>, target: usize) -> KdTree {
        build_with(names, &vec![None; names.len()], sample, target)
    }

    fn build_with(
        names: &[&str],
        ranges: &[Option<usize>],
        sample: Vec<Point>,
        target: usize,
    ) -> KdTree {
        KdTree::from_sample(dims(names), ranges, sample, target)
    }

    /// The distinct split values on `dim`, ascending.
    fn split_values(tree: &KdTree, dim: usize) -> Vec<PdbOwnedValue> {
        let mut values = Vec::new();
        fn walk(node: &KdNode, dim: usize, out: &mut Vec<PdbOwnedValue>) {
            if let KdNode::Split {
                dim: d,
                value,
                left,
                right,
            } = node
            {
                if *d == dim {
                    out.push(value.clone());
                }
                walk(left, dim, out);
                walk(right, dim, out);
            }
        }
        walk(&tree.root, dim, &mut values);
        values.sort_by(PdbOwnedValue::total_cmp);
        values.dedup_by(|a, b| a.total_cmp(b) == Ordering::Equal);
        values
    }

    /// The number of sample points routed to each partition.
    fn counts(tree: &KdTree, sample: &[Point]) -> Vec<usize> {
        let mut counts = vec![0usize; tree.partition_count()];
        for p in sample {
            counts[tree.route(p)] += 1;
        }
        counts
    }

    fn contains(bounds: &[DimBounds], point: &[PdbOwnedValue]) -> bool {
        bounds.iter().zip(point).all(|((lo, hi), v)| {
            let above_lo = match lo {
                Bound::Unbounded => true,
                Bound::Included(l) => {
                    !matches!(v, PdbOwnedValue::Null) && v.total_cmp(l) != Ordering::Less
                }
                Bound::Excluded(_) => unreachable!("lower bounds are inclusive"),
            };
            let below_hi = match hi {
                Bound::Unbounded => true,
                Bound::Excluded(h) => {
                    matches!(v, PdbOwnedValue::Null) || v.total_cmp(h) == Ordering::Less
                }
                Bound::Included(_) => unreachable!("upper bounds are exclusive"),
            };
            above_lo && below_hi
        })
    }

    /// Every sample point routes to a leaf whose box contains it, and every partition id is
    /// reachable.
    fn check_invariants(tree: &KdTree, sample: &[Point]) {
        let mut seen = vec![0usize; tree.partition_count()];
        for point in sample {
            let p = tree.route(point);
            let bounds = tree.partition_bounds(p).expect("partition in range");
            assert!(
                contains(&bounds, point),
                "{point:?} not in bounds of partition {p}: {bounds:?}"
            );
            seen[p] += 1;
        }
        assert!(
            seen.iter().all(|&n| n > 0),
            "empty partitions: {seen:?}\n{tree}"
        );
        assert!(tree.partition_bounds(tree.partition_count()).is_none());
    }

    #[test]
    fn one_dim_uniform_hits_target_and_balances() {
        let sample = i64s(0..1000);
        let tree = build(&["id"], sample.clone(), 4);
        assert_eq!(tree.partition_count(), 4);
        check_invariants(&tree, &sample);

        let rp = tree.to_range_partitioning().unwrap();
        assert_eq!(
            rp.split_points,
            vec![
                PdbOwnedValue::I64(250),
                PdbOwnedValue::I64(500),
                PdbOwnedValue::I64(750)
            ]
        );
    }

    #[test]
    fn non_power_of_two_targets_are_exact_and_balanced() {
        let sample = i64s(0..3000);
        for target in [2, 3, 5, 6, 7, 11, 16, 30] {
            let tree = build(&["id"], sample.clone(), target);
            assert_eq!(tree.partition_count(), target, "{tree}");
            check_invariants(&tree, &sample);

            let mut counts = vec![0usize; target];
            for p in &sample {
                counts[tree.route(p)] += 1;
            }
            let ideal = sample.len() / target;
            for c in counts {
                assert!(
                    c.abs_diff(ideal) <= 1,
                    "target={target}: counts off ideal {ideal}: {c}"
                );
            }
        }
    }

    #[test]
    fn heavy_duplicates_never_produce_empty_partitions() {
        // 90% of the rows share one value; the rest are spread out.
        let mut sample = i64s(std::iter::repeat_n(0, 900));
        sample.extend(i64s(1..101));
        let tree = build(&["k"], sample.clone(), 8);
        assert!(tree.partition_count() <= 8);
        assert!(tree.partition_count() >= 2, "{tree}");
        check_invariants(&tree, &sample);
        // The zeros cannot be split, so they all land in partition 0.
        assert_eq!(tree.route(&[PdbOwnedValue::I64(0)]), 0);
    }

    #[test]
    fn single_distinct_value_stays_one_partition() {
        let sample = i64s(std::iter::repeat_n(7, 50));
        let tree = build(&["k"], sample.clone(), 4);
        assert_eq!(tree.partition_count(), 1);
        check_invariants(&tree, &sample);
    }

    #[test]
    fn degenerate_inputs_are_unpartitioned() {
        for (sample, target) in [
            (vec![], 4),
            (i64s(0..1), 4),
            (i64s(0..100), 1),
            (i64s(0..100), 0),
        ] {
            let tree = build(&["k"], sample, target);
            assert_eq!(tree.partition_count(), 1);
            assert_eq!(tree.route(&[PdbOwnedValue::I64(42)]), 0);
            assert_eq!(
                tree.partition_bounds(0).unwrap(),
                vec![(Bound::Unbounded, Bound::Unbounded)]
            );
        }
    }

    #[test]
    fn nulls_route_to_the_lowest_partition_and_never_split() {
        let mut sample = i64s(0..800);
        sample.extend(std::iter::repeat_n(vec![PdbOwnedValue::Null], 200));
        let tree = build(&["k"], sample.clone(), 4);
        assert_eq!(tree.partition_count(), 4);
        check_invariants(&tree, &sample);
        assert_eq!(tree.route(&[PdbOwnedValue::Null]), 0);
        let rp = tree.to_range_partitioning().unwrap();
        assert!(
            rp.split_points
                .iter()
                .all(|v| !matches!(v, PdbOwnedValue::Null))
        );
    }

    #[test]
    fn one_dim_tree_matches_range_partitioning_semantics() {
        let mut sample = i64s((0..500).map(|i| i * 3));
        sample.extend(std::iter::repeat_n(vec![PdbOwnedValue::Null], 20));
        let tree = build(&["k"], sample, 5);
        let rp = tree.to_range_partitioning().unwrap();
        assert_eq!(rp.split_points.len(), tree.partition_count() - 1);

        // Probe values on, around, and away from every split point, plus NULL.
        let mut probes = vec![
            PdbOwnedValue::Null,
            PdbOwnedValue::I64(-1),
            PdbOwnedValue::I64(10_000),
        ];
        for sp in &rp.split_points {
            let PdbOwnedValue::I64(v) = sp else {
                unreachable!()
            };
            probes.extend([v - 1, *v, v + 1].map(PdbOwnedValue::I64));
        }
        for probe in probes {
            let expected = if matches!(probe, PdbOwnedValue::Null) {
                0
            } else {
                // Partition i holds [split[i-1], split[i]).
                rp.split_points
                    .iter()
                    .take_while(|sp| probe.total_cmp(sp) != Ordering::Less)
                    .count()
            };
            assert_eq!(
                tree.route(std::slice::from_ref(&probe)),
                expected,
                "probe {probe:?}"
            );
        }
    }

    #[test]
    fn two_dims_share_the_levels_first_field_first() {
        // Four leaves afford two levels of cuts: one on `x` at the root, then one on `y` in
        // each half.
        let sample = grid();
        let tree = build(&["x", "y"], sample.clone(), 4);
        assert_eq!(tree.partition_count(), 4);
        check_invariants(&tree, &sample);
        assert!(tree.to_range_partitioning().is_none());

        // Each quadrant is bounded on both dimensions.
        for p in 0..4 {
            let bounds = tree.partition_bounds(p).unwrap();
            let bounded = |b: &DimBounds| !matches!(b, (Bound::Unbounded, Bound::Unbounded));
            assert!(
                bounds.iter().all(bounded),
                "partition {p}: {bounds:?}\n{tree}"
            );
        }
        assert_eq!(
            tree.partition_bounds(0).unwrap(),
            vec![
                (Bound::Unbounded, Bound::Excluded(PdbOwnedValue::I64(20))),
                (Bound::Unbounded, Bound::Excluded(PdbOwnedValue::I64(20))),
            ]
        );

        // Thirty-two leaves afford five levels: `x` takes three of them as global ranges, and
        // `y` is cut into four inside each of the eight.
        let tree = build(&["x", "y"], grid(), 32);
        assert_eq!(tree.partition_count(), 32);
        check_invariants(&tree, &sample);
        assert_eq!(split_values(&tree, 0).len(), 7, "{tree}");
        assert_eq!(split_values(&tree, 1).len(), 3, "{tree}");
        assert!(counts(&tree, &sample).iter().all(|&c| c == 50), "{tree}");
    }

    #[test]
    fn first_dim_ranges_are_disjoint_across_the_tree() {
        // Every leaf's `x` range is one of the eight global ranges, whatever its `y` cut, so a
        // one-dimensional partitioning of `x` on those edges holds whole leaves.
        let sample = grid();
        let tree = build(&["x", "y"], sample.clone(), 32);
        let edges = split_values(&tree, 0);
        for p in 0..tree.partition_count() {
            let (lo, hi) = tree.partition_bounds(p).unwrap()[0].clone();
            if let Bound::Included(v) = &lo {
                assert!(edges.contains(v), "{tree}");
            }
            if let Bound::Excluded(v) = &hi {
                assert!(edges.contains(v), "{tree}");
            }
            let lo_pos = match lo {
                Bound::Unbounded => 0,
                Bound::Included(v) => edges.iter().position(|e| *e == v).unwrap() + 1,
                Bound::Excluded(_) => unreachable!(),
            };
            let hi_pos = match hi {
                Bound::Unbounded => edges.len(),
                Bound::Excluded(v) => edges.iter().position(|e| *e == v).unwrap(),
                Bound::Included(_) => unreachable!(),
            };
            assert_eq!(hi_pos, lo_pos, "partition {p} spans several ranges\n{tree}");
        }
    }

    #[test]
    fn later_dims_are_cut_inside_each_range_of_the_earlier() {
        // `y` is a function of `x`, and `y` is still cut inside every `x` range: the order of
        // the fields decides the cuts, not how much of a dimension a box covers.
        let sample: Vec<Point> = (0..1000)
            .map(|x| vec![PdbOwnedValue::I64(x), PdbOwnedValue::I64(x * 2)])
            .collect();
        let tree = build(&["x", "y"], sample.clone(), 8);
        assert_eq!(tree.partition_count(), 8);
        check_invariants(&tree, &sample);
        assert_eq!(split_values(&tree, 0).len(), 3, "{tree}");
        assert_eq!(split_values(&tree, 1).len(), 4, "{tree}");
        for p in 0..8 {
            let bounds = tree.partition_bounds(p).unwrap();
            assert_ne!(bounds[0], (Bound::Unbounded, Bound::Unbounded), "{tree}");
            assert_ne!(bounds[1], (Bound::Unbounded, Bound::Unbounded), "{tree}");
        }
    }

    #[test]
    fn exhausted_dim_passes_its_leaves_on() {
        // `tenant` has two values, so its four ranges collapse to two, and `id` takes the four
        // leaves of each tenant.
        let sample: Vec<Point> = (0..1000)
            .map(|i| vec![PdbOwnedValue::I64(i % 2), PdbOwnedValue::I64(i)])
            .collect();
        let tree = build(&["tenant", "id"], sample.clone(), 8);
        assert_eq!(tree.partition_count(), 8);
        check_invariants(&tree, &sample);
        assert_eq!(
            split_values(&tree, 0),
            vec![PdbOwnedValue::I64(1)],
            "{tree}"
        );
        for p in 0..8 {
            let bounds = tree.partition_bounds(p).unwrap();
            assert_ne!(bounds[0], (Bound::Unbounded, Bound::Unbounded), "{tree}");
        }
        assert!(counts(&tree, &sample).iter().all(|&c| c == 125), "{tree}");
    }

    #[test]
    fn explicit_ranges_shape_the_tree() {
        let sample = grid();

        // A count on the first field fixes its global ranges; the last field takes the rest.
        let tree = build_with(&["x", "y"], &[Some(2), None], sample.clone(), 32);
        assert_eq!(tree.partition_count(), 32);
        check_invariants(&tree, &sample);
        assert_eq!(
            split_values(&tree, 0),
            vec![PdbOwnedValue::I64(20)],
            "{tree}"
        );
        assert_eq!(split_values(&tree, 1).len(), 15, "{tree}");

        // A count on a later field leaves the first one the leaves that count does not use.
        let tree = build_with(&["x", "y"], &[None, Some(2)], sample.clone(), 32);
        assert_eq!(tree.partition_count(), 32);
        assert_eq!(split_values(&tree, 0).len(), 15, "{tree}");

        // Counts on every field fix the leaf count below the target.
        let tree = build_with(&["x", "y"], &[Some(2), Some(4)], sample.clone(), 32);
        assert_eq!(tree.partition_count(), 8, "{tree}");
        check_invariants(&tree, &sample);
        assert!(counts(&tree, &sample).iter().all(|&c| c == 200), "{tree}");

        // The target caps a count.
        let tree = build_with(&["x", "y"], &[Some(16), None], sample.clone(), 4);
        assert_eq!(tree.partition_count(), 4, "{tree}");
        assert_eq!(split_values(&tree, 1).len(), 0, "{tree}");
    }

    #[test]
    fn non_power_of_two_targets_balance_across_dims() {
        // Twelve leaves: `x` takes four ranges, `y` three inside each, and the ranges of `y`
        // get a share of the sample proportional to their leaves.
        let sample = grid();
        let tree = build(&["x", "y"], sample.clone(), 12);
        assert_eq!(tree.partition_count(), 12, "{tree}");
        check_invariants(&tree, &sample);
        assert_eq!(split_values(&tree, 0).len(), 3, "{tree}");
        // A cut lands on a value change, and every `y` value has ten points inside a range of
        // ten `x` values, so a leaf can miss its ideal share by up to one such plateau.
        let ideal = sample.len() / 12;
        for c in counts(&tree, &sample) {
            assert!(
                c.abs_diff(ideal) <= 10,
                "counts off ideal {ideal}: {c}\n{tree}"
            );
        }
    }

    #[test]
    fn one_dim_hands_unused_leaves_to_the_right() {
        // The left range cannot be cut, so its three unused leaves, and the ranges they were
        // for, move to the right, which still comes out at the target.
        let mut sample = i64s(std::iter::repeat_n(0, 500));
        sample.extend(i64s(1..501));
        let tree = build(&["k"], sample.clone(), 8);
        assert_eq!(tree.partition_count(), 8, "{tree}");
        check_invariants(&tree, &sample);
        assert_eq!(tree.route(&[PdbOwnedValue::I64(0)]), 0);
        let per_partition = counts(&tree, &sample);
        assert_eq!(per_partition[0], 500);
        assert!(
            per_partition[1..].iter().all(|&c| c.abs_diff(500 / 7) <= 1),
            "{per_partition:?}"
        );
    }

    #[test]
    fn mixed_types_and_serde_round_trip() {
        let sample: Vec<Point> = (0..300)
            .map(|i| {
                vec![
                    PdbOwnedValue::Str(format!("user-{:04}", i % 37)),
                    PdbOwnedValue::F64(i as f64 / 7.0),
                    PdbOwnedValue::Bool(i % 3 == 0),
                    PdbOwnedValue::I64(i),
                    // One day apart, starting at the Postgres epoch (2000-01-01).
                    PdbOwnedValue::Date(
                        PostgresDateTime::try_from_raw(i * 86_400_000_000).unwrap(),
                    ),
                ]
            })
            .collect();
        let tree = build(&["id", "name", "score", "flag", "day"], sample.clone(), 6);
        assert_eq!(tree.partition_count(), 6, "{tree}");
        check_invariants(&tree, &sample);

        // The tree carries an `I64` split (`id` comes first, so the root cuts it); the round
        // trip must hand it back as `I64`, not as tantivy's untagged `U64`.
        assert_eq!(split_values(&tree, 0).len(), 1, "{tree}");
        let bytes = postcard::to_allocvec(&tree).unwrap();
        let back: KdTree = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(back, tree);
        for p in &sample {
            assert_eq!(back.route(p), tree.route(p));
        }
    }

    #[test]
    fn non_finite_float_split_values_round_trip() {
        // A float column can hold `Infinity`/`NaN`, and `total_cmp` sorts them to the ends, so
        // they can become split values. The wire form must reproduce them bit for bit.
        let mut sample: Vec<Point> = (0..200)
            .map(|i| vec![PdbOwnedValue::F64(i as f64)])
            .collect();
        sample.extend(std::iter::repeat_n(
            vec![PdbOwnedValue::F64(f64::INFINITY)],
            50,
        ));
        sample.extend(std::iter::repeat_n(
            vec![PdbOwnedValue::F64(f64::NEG_INFINITY)],
            50,
        ));
        sample.extend(std::iter::repeat_n(vec![PdbOwnedValue::F64(f64::NAN)], 50));
        let tree = build(&["x"], sample.clone(), 6);
        check_invariants(&tree, &sample);

        // `PartialEq` treats `NaN != NaN`, so compare the re-serialized bytes rather than the
        // trees: equal bytes means every split value, `NaN` included, round-tripped bit for bit.
        let bytes = postcard::to_allocvec(&tree).unwrap();
        let back: KdTree = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(postcard::to_allocvec(&back).unwrap(), bytes);
        for probe in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN, 0.0, 199.0] {
            assert_eq!(
                back.route(&[PdbOwnedValue::F64(probe)]),
                tree.route(&[PdbOwnedValue::F64(probe)]),
                "probe {probe}"
            );
        }
    }

    #[test]
    fn display_lists_every_split_and_leaf() {
        let tree = build(&["id"], i64s(0..100), 3);
        let text = tree.to_string();
        assert!(
            text.starts_with("partition_by=[id], partitions=3"),
            "{text}"
        );
        for p in 0..3 {
            assert!(text.contains(&format!("-> {p}")), "{text}");
        }
        assert_eq!(text.matches("id <").count(), 2, "{text}");
    }

    #[test]
    fn bounds_listing_shows_one_box_per_partition() {
        let mut sample = Vec::new();
        for x in 0..40 {
            for y in 0..40 {
                sample.push(vec![PdbOwnedValue::I64(x), PdbOwnedValue::I64(y)]);
            }
        }
        let tree = build(&["x", "y"], sample, 4);
        assert_eq!(
            tree.bounds_listing().to_string(),
            "partition 0: x=[.., 20) y=[.., 20)\n\
             partition 1: x=[.., 20) y=[20, ..)\n\
             partition 2: x=[20, ..) y=[.., 20)\n\
             partition 3: x=[20, ..) y=[20, ..)"
        );

        // Dates and strings render readably, without a Postgres backend.
        let sample: Vec<Point> = (0..100)
            .map(|i| {
                vec![
                    PdbOwnedValue::Date(
                        PostgresDateTime::try_from_raw(i * 86_400_000_000).unwrap(),
                    ),
                    PdbOwnedValue::Str(format!("k{:02}", i / 10)),
                ]
            })
            .collect();
        let tree = build(&["day", "key"], sample, 2);
        assert_eq!(
            tree.bounds_listing().to_string(),
            "partition 0: day=[.., 2000-02-20T00:00:00+00:00) key=[.., ..)\n\
             partition 1: day=[2000-02-20T00:00:00+00:00, ..) key=[.., ..)"
        );

        // Ctids render as (block, offset) tuples.
        let sample: Vec<Point> = (0..100)
            .map(|i| {
                let block = i as u64;
                let offset = 1u64;
                vec![PdbOwnedValue::U64((block << 16) | offset)]
            })
            .collect();
        let tree = build(&["ctid"], sample, 2);
        assert_eq!(
            tree.bounds_listing().to_string(),
            "partition 0: ctid=[.., (50,1))\n\
             partition 1: ctid=[(50,1), ..)"
        );
        assert_eq!(
            tree.to_string(),
            "partition_by=[ctid], partitions=2\n\
             ctid < (50,1) -> 0\n\
             ctid >= (50,1) -> 1"
        );
    }

    #[test]
    fn a_count_holds_when_a_later_field_runs_out() {
        // `flag` has two values, so each `id` range can only be cut in two. The leaves the
        // count of four left for `flag` stay unused instead of turning into more `id` ranges.
        let sample: Vec<Point> = (0..1000)
            .map(|i| vec![PdbOwnedValue::I64(i), PdbOwnedValue::Bool(i % 2 == 0)])
            .collect();
        let tree = build_with(&["id", "flag"], &[Some(4), None], sample.clone(), 32);
        check_invariants(&tree, &sample);
        assert_eq!(split_values(&tree, 0).len(), 3, "{tree}");
        assert_eq!(tree.partition_count(), 8, "{tree}");

        // `y` is constant in the lowest third of `x`, so that range holds one leaf, and the
        // other two ranges are still cut on `y`: three `x` ranges, five leaves.
        let sample: Vec<Point> = (0..900)
            .map(|x| {
                vec![
                    PdbOwnedValue::I64(x),
                    PdbOwnedValue::I64(if x < 300 { 0 } else { x }),
                ]
            })
            .collect();
        let tree = build_with(&["x", "y"], &[Some(3), Some(2)], sample.clone(), 6);
        check_invariants(&tree, &sample);
        assert_eq!(split_values(&tree, 0).len(), 2, "{tree}");
        assert_eq!(tree.partition_count(), 5, "{tree}");
    }

    #[test]
    fn a_skewed_first_field_keeps_the_leaves_balanced() {
        // Nine rows in ten belong to one tenant, so the tenant cut lands far from the quantile
        // its share of the leaves asked for; the leaves follow the rows instead.
        let sample: Vec<Point> = (0..1000)
            .map(|i| {
                vec![
                    PdbOwnedValue::I64(i64::from(i >= 900)),
                    PdbOwnedValue::I64(i),
                ]
            })
            .collect();
        let tree = build(&["tenant", "id"], sample.clone(), 32);
        check_invariants(&tree, &sample);
        assert_eq!(tree.partition_count(), 32, "{tree}");
        let per_partition = counts(&tree, &sample);
        let (min, max) = (
            per_partition.iter().min().unwrap(),
            per_partition.iter().max().unwrap(),
        );
        assert!(max <= &(2 * min), "{per_partition:?}\n{tree}");
    }

    #[test]
    fn three_fields_share_the_levels_in_order() {
        // Eight leaves over three fields: one level each.
        let sample: Vec<Point> = (0..512)
            .map(|i| {
                vec![
                    PdbOwnedValue::I64(i / 64),
                    PdbOwnedValue::I64((i / 8) % 8),
                    PdbOwnedValue::I64(i % 8),
                ]
            })
            .collect();
        let tree = build(&["a", "b", "c"], sample.clone(), 8);
        check_invariants(&tree, &sample);
        assert_eq!(tree.partition_count(), 8, "{tree}");
        for dim in 0..3 {
            assert_eq!(split_values(&tree, dim).len(), 1, "{tree}");
        }
        assert!(counts(&tree, &sample).iter().all(|&c| c == 64), "{tree}");
    }
}
