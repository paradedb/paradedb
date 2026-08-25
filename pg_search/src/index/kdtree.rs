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
    /// Each split goes on the dimension whose values inside the box still span the widest slice
    /// of that dimension's global distribution (measured by rank, so dimensions of different
    /// types are comparable); ties go to the earlier `partition_by` field. The cut sits at the
    /// quantile that gives both children a share of the sample proportional to the leaves they
    /// will hold, so a single wide dimension splits into roughly equal leaves even when
    /// `target_partitions` is not a power of two. Leaf sizes can still come out uneven when a
    /// low-cardinality dimension wins a split: cutting it buys pruning on that dimension, at the
    /// cost of balance. A box whose points are all equal on every dimension cannot be cut and
    /// stays a leaf, so the tree can end up with fewer partitions than requested when the sample
    /// has too few distinct values.
    ///
    /// Split values are never NULL, so every leaf's box is expressible as plain range bounds.
    pub fn from_sample(dims: Vec<FieldName>, sample: Vec<Point>, target_partitions: usize) -> Self {
        let ndims = dims.len();
        debug_assert!(
            sample.iter().all(|p| p.len() == ndims),
            "every sample point must have one value per partition_by field"
        );
        if target_partitions <= 1 || sample.len() < 2 || ndims == 0 {
            return Self::unpartitioned(dims);
        }

        let ranks = global_ranks(&sample, ndims);
        let mut idx: Vec<usize> = (0..sample.len()).collect();
        let mut builder = Builder {
            sample: &sample,
            ranks: &ranks,
            next_partition: 0,
        };
        let root = builder.build(&mut idx, target_partitions);
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
                    Bound::Included(v) => write!(f, " {dim}=[{}", v.plain_display())?,
                    _ => write!(f, " {dim}=[..")?,
                }
                match upper {
                    Bound::Excluded(v) => write!(f, ", {})", v.plain_display())?,
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
            write!(f, "\n{indent}{dim} < {}", value.plain_display())?;
            fmt_node(left, tree, depth + 1, f)?;
            write!(f, "\n{indent}{dim} >= {}", value.plain_display())?;
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

/// `ranks[dim][i]` is the position of point `i` in the ascending order of dimension `dim`,
/// with equal values sharing the position of their first occurrence. Rank spread inside a box
/// is a type-agnostic, distribution-normalized measure of how much of a dimension the box
/// still covers.
fn global_ranks(sample: &[Point], ndims: usize) -> Vec<Vec<usize>> {
    (0..ndims)
        .map(|dim| {
            let mut order: Vec<usize> = (0..sample.len()).collect();
            order.sort_by(|&a, &b| sample[a][dim].total_cmp(&sample[b][dim]));
            let mut ranks = vec![0; sample.len()];
            let mut rank = 0;
            for (pos, &i) in order.iter().enumerate() {
                if pos > 0
                    && sample[order[pos - 1]][dim].total_cmp(&sample[i][dim]) != Ordering::Equal
                {
                    rank = pos;
                }
                ranks[i] = rank;
            }
            ranks
        })
        .collect()
}

struct Builder<'a> {
    sample: &'a [Point],
    ranks: &'a [Vec<usize>],
    next_partition: usize,
}

impl Builder<'_> {
    fn leaf(&mut self) -> KdNode {
        let partition = self.next_partition;
        self.next_partition += 1;
        KdNode::Leaf { partition }
    }

    /// Cuts the box holding the points in `idx` into at most `k` leaves.
    fn build(&mut self, idx: &mut [usize], k: usize) -> KdNode {
        if k <= 1 || idx.len() < 2 {
            return self.leaf();
        }
        let Some(dim) = self.widest_dim(idx) else {
            return self.leaf();
        };

        idx.sort_by(|&a, &b| self.sample[a][dim].total_cmp(&self.sample[b][dim]));

        let k_left = k / 2;
        let target = idx.len() * k_left / k;
        let Some(cut) = self.nearest_cut(idx, dim, target) else {
            return self.leaf();
        };
        let value = self.sample[idx[cut]][dim].clone();

        let (left_idx, right_idx) = idx.split_at_mut(cut);
        let before = self.next_partition;
        let left = self.build(left_idx, k_left);
        // A left subtree that ran out of distinct values hands its unused leaves to the right.
        let k_right = k - (self.next_partition - before);
        let right = self.build(right_idx, k_right);
        KdNode::Split {
            dim,
            value,
            left: Box::new(left),
            right: Box::new(right),
        }
    }

    /// The dimension whose values inside the box span the largest rank range, or `None` when
    /// the box is a single point on every dimension.
    fn widest_dim(&self, idx: &[usize]) -> Option<usize> {
        let mut best: Option<(usize, usize)> = None;
        for (dim, ranks) in self.ranks.iter().enumerate() {
            let (min, max) = idx.iter().fold((usize::MAX, 0), |(lo, hi), &i| {
                (lo.min(ranks[i]), hi.max(ranks[i]))
            });
            let spread = max - min;
            if spread > 0 && best.is_none_or(|(_, s)| spread > s) {
                best = Some((dim, spread));
            }
        }
        best.map(|(dim, _)| dim)
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
        let tree = KdTree::from_sample(dims(&["id"]), sample.clone(), 4);
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
            let tree = KdTree::from_sample(dims(&["id"]), sample.clone(), target);
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
        let tree = KdTree::from_sample(dims(&["k"]), sample.clone(), 8);
        assert!(tree.partition_count() <= 8);
        assert!(tree.partition_count() >= 2, "{tree}");
        check_invariants(&tree, &sample);
        // The zeros cannot be split, so they all land in partition 0.
        assert_eq!(tree.route(&[PdbOwnedValue::I64(0)]), 0);
    }

    #[test]
    fn single_distinct_value_stays_one_partition() {
        let sample = i64s(std::iter::repeat_n(7, 50));
        let tree = KdTree::from_sample(dims(&["k"]), sample.clone(), 4);
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
            let tree = KdTree::from_sample(dims(&["k"]), sample, target);
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
        let tree = KdTree::from_sample(dims(&["k"]), sample.clone(), 4);
        assert_eq!(tree.partition_count(), 4);
        check_invariants(&tree, &sample);
        assert_eq!(tree.route(&[PdbOwnedValue::Null]), 0);
        let rp = tree.to_range_partitioning().unwrap();
        assert!(rp
            .split_points
            .iter()
            .all(|v| !matches!(v, PdbOwnedValue::Null)));
    }

    #[test]
    fn one_dim_tree_matches_range_partitioning_semantics() {
        let mut sample = i64s((0..500).map(|i| i * 3));
        sample.extend(std::iter::repeat_n(vec![PdbOwnedValue::Null], 20));
        let tree = KdTree::from_sample(dims(&["k"]), sample, 5);
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
    fn two_independent_dims_alternate() {
        // A 40x40 grid: both dimensions have identical marginals, so the tie-break picks `x`
        // at the root, after which `y` spans the full range in each half and gets cut next.
        let mut sample = Vec::new();
        for x in 0..40 {
            for y in 0..40 {
                sample.push(vec![PdbOwnedValue::I64(x), PdbOwnedValue::I64(y)]);
            }
        }
        let tree = KdTree::from_sample(dims(&["x", "y"]), sample.clone(), 4);
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
    }

    #[test]
    fn correlated_dims_are_cut_on_the_first_declared() {
        // `y` is a function of `x`, so cutting `x` also narrows `y`; the tie-break keeps
        // choosing `x`, and every leaf still has a disjoint `y` range through its `x` bounds.
        let sample: Vec<Point> = (0..1000)
            .map(|x| vec![PdbOwnedValue::I64(x), PdbOwnedValue::I64(x * 2)])
            .collect();
        let tree = KdTree::from_sample(dims(&["x", "y"]), sample.clone(), 8);
        assert_eq!(tree.partition_count(), 8);
        check_invariants(&tree, &sample);
        for p in 0..8 {
            let bounds = tree.partition_bounds(p).unwrap();
            assert_eq!(bounds[1], (Bound::Unbounded, Bound::Unbounded), "{tree}");
        }
    }

    #[test]
    fn skewed_dim_yields_to_the_wider_one() {
        // `id` is unique and wider, so the root cuts it; each half still holds both tenants,
        // so the next cut is on `tenant`, after which its spread is 0 and `id` takes the rest.
        let sample: Vec<Point> = (0..1000)
            .map(|i| vec![PdbOwnedValue::I64(i % 2), PdbOwnedValue::I64(i)])
            .collect();
        let tree = KdTree::from_sample(dims(&["tenant", "id"]), sample.clone(), 8);
        assert_eq!(tree.partition_count(), 8);
        check_invariants(&tree, &sample);
        let tenant_bounds: Vec<_> = (0..8)
            .map(|p| tree.partition_bounds(p).unwrap()[0].clone())
            .collect();
        assert!(tenant_bounds
            .iter()
            .all(|b| b != &(Bound::Unbounded, Bound::Unbounded)));
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
        let tree = KdTree::from_sample(
            dims(&["name", "score", "flag", "id", "day"]),
            sample.clone(),
            6,
        );
        assert_eq!(tree.partition_count(), 6, "{tree}");
        check_invariants(&tree, &sample);

        // The tree carries an `I64` split (`id` is unique, so it is the widest dimension); the
        // round trip must hand it back as `I64`, not as tantivy's untagged `U64`.
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
        let tree = KdTree::from_sample(dims(&["x"]), sample.clone(), 6);
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
        let tree = KdTree::from_sample(dims(&["id"]), i64s(0..100), 3);
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
        let tree = KdTree::from_sample(dims(&["x", "y"]), sample, 4);
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
        let tree = KdTree::from_sample(dims(&["day", "key"]), sample, 2);
        assert_eq!(
            tree.bounds_listing().to_string(),
            "partition 0: day=[.., 2000-02-20T00:00:00+00:00) key=[.., ..)\n\
             partition 1: day=[2000-02-20T00:00:00+00:00, ..) key=[.., ..)"
        );
    }
}
