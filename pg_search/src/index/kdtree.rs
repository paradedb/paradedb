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

use crate::api::{CTID_FIELD_NAME, FieldName};
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

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum DimHandle {
    Index(usize),
    Ctid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum KdNode {
    Leaf {
        partition: usize,
    },
    Split {
        dim: DimHandle,
        #[serde(with = "crate::postgres::pdb_owned_value::exact_scalar_wire")]
        value: PdbOwnedValue,
        left: Box<KdNode>,
        right: Box<KdNode>,
    },
}

/// The half-open range a partition covers on one dimension.
pub type DimBounds = (Bound<PdbOwnedValue>, Bound<PdbOwnedValue>);

#[derive(Debug, PartialEq)]
pub struct PartitionBounds {
    pub dims: Vec<DimBounds>,
    pub ctid_fallback: Option<DimBounds>,
}
impl PartitionBounds {
    pub fn new(dims: Vec<DimBounds>) -> Self {
        Self {
            dims,
            ctid_fallback: None,
        }
    }

    pub fn with_ctid_fallback(mut self, ctid_fallback: DimBounds) -> Self {
        self.ctid_fallback = Some(ctid_fallback);
        self
    }
}

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
    /// `target_partitions` is not a power of two.
    ///
    /// If `ctid_fallback` is `None`, leaf sizes can still come out uneven when a
    /// low-cardinality dimension wins a split: cutting it buys pruning on that dimension, at the
    /// cost of balance. A box whose points are all equal on every dimension cannot be cut and
    /// stays a leaf, so the tree can end up with fewer partitions than requested when the sample
    /// has too few distinct values.
    ///
    /// If `ctid_fallback` is `Some(_)`, then any box that should be cut but has dimensions to cut
    /// on will fall back to cutting on `ctid`, which will result in roughly balanced leaves
    ///
    /// Split values are never NULL, so every leaf's box is expressible as plain range bounds.
    pub fn from_sample(
        dims: Vec<FieldName>,
        sample: Vec<Point>,
        target_partitions: usize,
        ctid_fallback: Option<Vec<u64>>,
    ) -> Self {
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
            ctid_fallback: ctid_fallback.as_deref(),
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

    /// The dimension of every split, root first then left before right (pre-order), so tests
    /// can tell a declared dimension from the ctid fallback.
    #[cfg(any(test, feature = "pg_test"))]
    pub(crate) fn split_dims(&self) -> Vec<DimHandle> {
        fn walk(node: &KdNode, out: &mut Vec<DimHandle>) {
            if let KdNode::Split {
                dim, left, right, ..
            } = node
            {
                out.push(*dim);
                walk(left, out);
                walk(right, out);
            }
        }
        let mut out = Vec::new();
        walk(&self.root, &mut out);
        out
    }

    /// The partition a row belongs to. `values` must be laid out like [`Self::dims`].
    pub fn route(&self, values: &[PdbOwnedValue], ctid: u64) -> usize {
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
                    let choose_left = match dim {
                        DimHandle::Index(i) => goes_left(&values[*i], value),
                        DimHandle::Ctid => goes_left(&ctid.into(), value),
                    };
                    node = if choose_left { left } else { right };
                }
            }
        }
    }

    /// The bounding box of `partition`, one half-open range per dimension of [`Self::dims`].
    ///
    /// Returns `None` when `partition` is out of range.
    pub fn partition_bounds(&self, partition: usize) -> Option<PartitionBounds> {
        let mut bounds =
            PartitionBounds::new(vec![(Bound::Unbounded, Bound::Unbounded); self.dims.len()]);
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
                    let bound_handle = match dim {
                        DimHandle::Index(i) => &mut bounds.dims[*i],
                        DimHandle::Ctid => {
                            if bounds.ctid_fallback.is_none() {
                                bounds =
                                    bounds.with_ctid_fallback((Bound::Unbounded, Bound::Unbounded));
                            }
                            bounds.ctid_fallback.as_mut().expect("just added it")
                        }
                    };

                    // Leaves are numbered in-order, so the highest partition in the left
                    // subtree is one below the lowest in the right subtree.
                    if partition < first_partition(right) {
                        bound_handle.1 = Bound::Excluded(value.clone());
                        node = left;
                    } else {
                        bound_handle.0 = Bound::Included(value.clone());
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
            let bounds = bounds.dims.iter().chain(bounds.ctid_fallback.iter());
            for (dim, (lower, upper)) in tree.dims.iter().zip(bounds) {
                match lower {
                    Bound::Included(v) => {
                        write!(f, " {dim}=[{}", DimValueDisplay { dim, value: v })?
                    }
                    _ => write!(f, " {dim}=[..")?,
                }
                match upper {
                    Bound::Excluded(v) => write!(f, ", {})", DimValueDisplay { dim, value: v })?,
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
            let dim = match dim {
                DimHandle::Index(i) => &tree.dims[*i],
                DimHandle::Ctid => &FieldName::from(CTID_FIELD_NAME),
            };
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
    ctid_fallback: Option<&'a [u64]>,
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

        match dim {
            DimHandle::Index(i) => {
                idx.sort_by(|&a, &b| self.sample[a][i].total_cmp(&self.sample[b][i]))
            }
            DimHandle::Ctid => {
                let ctids = self
                    .ctid_fallback
                    .expect("should always exist if given a ctid handle");
                idx.sort_by(|&a, &b| ctids[a].cmp(&ctids[b]))
            }
        }

        let target = idx.len() * (k / 2) / k;
        let Some(cut) = self.nearest_cut(idx, dim, target) else {
            return self.leaf();
        };
        let k_left = {
            let rows = idx.len();
            let left_rows = cut;

            // Give each side leaves in proportion to its share of the rows:
            // left_rows / rows of the k leaves, rounded to the nearest whole leaf.
            // Multiply before dividing so a small `rows` doesn't truncate to zero,
            // and add half the divisor first so the integer division rounds instead of floors.
            let proportional_leaves = (left_rows * k + rows / 2) / rows;
            proportional_leaves.clamp(1, k - 1)
        };
        let value = match dim {
            DimHandle::Index(i) => self.sample[idx[cut]][i].clone(),
            DimHandle::Ctid => self
                .ctid_fallback
                .map(|ctids| ctids[idx[cut]].into())
                .expect("should exist"),
        };

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
    /// the box is a single point on every dimension. All non-ctid dimensions will be tried first,
    /// with ctid only being considered as a fallback if every other dimension holds a single point.
    fn widest_dim(&self, idx: &[usize]) -> Option<DimHandle> {
        assert!(idx.len() > 1, "should only ever run with cuttable idx");
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
        match best {
            Some((dim_idx, _)) => Some(DimHandle::Index(dim_idx)),
            None => self.ctid_fallback.map(|_| DimHandle::Ctid),
        }
    }

    /// The position in `idx` (sorted on `dim`) closest to `target` where the value changes,
    /// so that both sides of the cut are non-empty and the split value is the smallest value
    /// on the right. NULLs sort first and are all equal, so the value at a cut is never NULL.
    fn nearest_cut(&self, idx: &[usize], dim: DimHandle, target: usize) -> Option<usize> {
        let target = target.clamp(1, idx.len() - 1);
        let below = self.find_next_change(idx, dim, (1..=target).rev());
        let above = self.find_next_change(idx, dim, target + 1..idx.len());
        match (below, above) {
            (Some(b), Some(a)) => Some(if target - b <= a - target { b } else { a }),
            (Some(b), None) => Some(b),
            (None, Some(a)) => Some(a),
            (None, None) => None,
        }
    }

    /// helper method for `nearest_cut`
    fn find_next_change(
        &self,
        idx: &[usize],
        dim: DimHandle,
        mut r: impl Iterator<Item = usize>,
    ) -> Option<usize> {
        let changes = |pos: usize, dim_idx: usize| {
            self.sample[idx[pos - 1]][dim_idx].total_cmp(&self.sample[idx[pos]][dim_idx])
                != Ordering::Equal
        };

        match dim {
            DimHandle::Index(i) => r.find(|pos| changes(*pos, i)),
            // ctids are guaranteed to change, so always the first item
            DimHandle::Ctid => r.next(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ops::RangeBounds;

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

    /// An `n` by `n` grid of `(x, y)` points.
    fn grid(n: i64) -> Vec<Point> {
        (0..n)
            .flat_map(|x| (0..n).map(move |y| vec![PdbOwnedValue::I64(x), PdbOwnedValue::I64(y)]))
            .collect()
    }

    /// Unique ctids in heap order, one per sample row, 50 rows to a block.
    fn heap_ctids(n: usize) -> Vec<u64> {
        (0..n as u64)
            .map(|i| ((i / 50) << 16) | (i % 50 + 1))
            .collect()
    }

    /// Rows per partition when every sample row is routed with its ctid.
    fn route_counts(tree: &KdTree, sample: &[Point], ctids: &[u64]) -> Vec<usize> {
        let mut counts = vec![0usize; tree.partition_count()];
        for (point, &ctid) in sample.iter().zip(ctids) {
            counts[tree.route(point, ctid)] += 1;
        }
        counts
    }

    fn contains(bounds: &PartitionBounds, point: &[PdbOwnedValue], ctid: u64) -> bool {
        bounds.dims.iter().zip(point).all(|((lo, hi), v)| {
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
        }) && bounds
            .ctid_fallback
            .as_ref()
            .map(|b| b.contains(&ctid.into()))
            .unwrap_or(true)
    }

    /// Every sample point routes to a leaf whose box contains it, and every partition id is
    /// reachable. With a ctid fallback, each row's ctid must also fall in its partition's ctid
    /// range, the extra entry after the declared dimensions, when the partition has one.
    fn check_invariants(tree: &KdTree, sample: &[Point], ctids: Option<&[u64]>) {
        let mut seen = vec![0usize; tree.partition_count()];
        for (i, point) in sample.iter().enumerate() {
            let ctid = ctids.map_or(i as u64, |c| c[i]);
            let p = tree.route(point, ctid);
            let bounds = tree.partition_bounds(p).expect("partition in range");
            assert!(
                contains(&bounds, point, ctid),
                "{point:?} or ctid {ctid} not in bounds of partition {p}: {bounds:?}"
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
        let tree = KdTree::from_sample(dims(&["id"]), sample.clone(), 4, None);
        assert_eq!(tree.partition_count(), 4);
        check_invariants(&tree, &sample, None);

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
            let tree = KdTree::from_sample(dims(&["id"]), sample.clone(), target, None);
            assert_eq!(tree.partition_count(), target, "{tree}");
            check_invariants(&tree, &sample, None);

            let mut counts = vec![0usize; target];
            for p in &sample {
                counts[tree.route(p, 0)] += 1;
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
        let tree = KdTree::from_sample(dims(&["k"]), sample.clone(), 8, None);
        assert!(tree.partition_count() <= 8);
        assert!(tree.partition_count() >= 2, "{tree}");
        check_invariants(&tree, &sample, None);
        // The zeros cannot be split, so they all land in partition 0.
        assert_eq!(tree.route(&[PdbOwnedValue::I64(0)], 0), 0);
    }

    #[test]
    fn single_distinct_value_stays_one_partition() {
        let sample = i64s(std::iter::repeat_n(7, 50));
        let tree = KdTree::from_sample(dims(&["k"]), sample.clone(), 4, None);
        assert_eq!(tree.partition_count(), 1);
        check_invariants(&tree, &sample, None);
    }

    #[test]
    fn degenerate_inputs_are_unpartitioned() {
        for (sample, target) in [
            (vec![], 4),
            (i64s(0..1), 4),
            (i64s(0..100), 1),
            (i64s(0..100), 0),
        ] {
            let tree = KdTree::from_sample(dims(&["k"]), sample, target, None);
            assert_eq!(tree.partition_count(), 1);
            assert_eq!(tree.route(&[PdbOwnedValue::I64(42)], 0), 0);
            assert_eq!(
                tree.partition_bounds(0).unwrap(),
                PartitionBounds::new(vec![(Bound::Unbounded, Bound::Unbounded)]),
            );
        }
    }

    #[test]
    fn nulls_route_to_the_lowest_partition_and_never_split() {
        let mut sample = i64s(0..800);
        sample.extend(std::iter::repeat_n(vec![PdbOwnedValue::Null], 200));
        let tree = KdTree::from_sample(dims(&["k"]), sample.clone(), 4, None);
        assert_eq!(tree.partition_count(), 4);
        check_invariants(&tree, &sample, None);
        assert_eq!(tree.route(&[PdbOwnedValue::Null], 0), 0);
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
        let tree = KdTree::from_sample(dims(&["k"]), sample, 5, None);
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
                tree.route(std::slice::from_ref(&probe), 0),
                expected,
                "probe {probe:?}"
            );
        }
    }

    #[test]
    fn two_independent_dims_alternate() {
        // A 40x40 grid: both dimensions have identical marginals, so the tie-break picks `x`
        // at the root, after which `y` spans the full range in each half and gets cut next.
        let sample = grid(40);
        let tree = KdTree::from_sample(dims(&["x", "y"]), sample.clone(), 4, None);
        assert_eq!(tree.partition_count(), 4);
        check_invariants(&tree, &sample, None);
        assert!(tree.to_range_partitioning().is_none());

        // Each quadrant is bounded on both dimensions.
        for p in 0..4 {
            let bounds = tree.partition_bounds(p).unwrap();
            let bounded = |b: &DimBounds| !matches!(b, (Bound::Unbounded, Bound::Unbounded));
            assert!(
                bounds.dims.iter().all(bounded),
                "partition {p}: {bounds:?}\n{tree}"
            );
        }
        assert_eq!(
            tree.partition_bounds(0).unwrap(),
            PartitionBounds::new(vec![
                (Bound::Unbounded, Bound::Excluded(PdbOwnedValue::I64(20))),
                (Bound::Unbounded, Bound::Excluded(PdbOwnedValue::I64(20))),
            ])
        );
    }

    #[test]
    fn correlated_dims_are_cut_on_the_first_declared() {
        // `y` is a function of `x`, so cutting `x` also narrows `y`; the tie-break keeps
        // choosing `x`, and every leaf still has a disjoint `y` range through its `x` bounds.
        let sample: Vec<Point> = (0..1000)
            .map(|x| vec![PdbOwnedValue::I64(x), PdbOwnedValue::I64(x * 2)])
            .collect();
        let tree = KdTree::from_sample(dims(&["x", "y"]), sample.clone(), 8, None);
        assert_eq!(tree.partition_count(), 8);
        check_invariants(&tree, &sample, None);
        for p in 0..8 {
            let bounds = tree.partition_bounds(p).unwrap();
            assert_eq!(
                bounds.dims[1],
                (Bound::Unbounded, Bound::Unbounded),
                "{tree}"
            );
        }
    }

    #[test]
    fn skewed_dim_yields_to_the_wider_one() {
        // `id` is unique and wider, so the root cuts it; each half still holds both tenants,
        // so the next cut is on `tenant`, after which its spread is 0 and `id` takes the rest.
        let sample: Vec<Point> = (0..1000)
            .map(|i| vec![PdbOwnedValue::I64(i % 2), PdbOwnedValue::I64(i)])
            .collect();
        let tree = KdTree::from_sample(dims(&["tenant", "id"]), sample.clone(), 8, None);
        assert_eq!(tree.partition_count(), 8);
        check_invariants(&tree, &sample, None);
        let tenant_bounds: Vec<_> = (0..8)
            .map(|p| tree.partition_bounds(p).unwrap().dims[0].clone())
            .collect();
        assert!(
            tenant_bounds
                .iter()
                .all(|b| b != &(Bound::Unbounded, Bound::Unbounded))
        );
    }

    #[test]
    fn declared_ctid_competes_as_an_equal_but_the_fallback_waits() {
        // Two tenants over unique ctids, as in `skewed_dim_yields_to_the_wider_one`: ctid is the
        // wider dimension, so when it is declared it takes the root cut.
        let ctids = heap_ctids(1000);
        let tenant = |i: usize| PdbOwnedValue::I64((i % 2) as i64);

        // Declared: ctid is an ordinary dimension carried in each point, with no fallback.
        // The root cuts ctid, and each half still holds both tenants, so it cuts tenant next.
        let sample: Vec<Point> = ctids
            .iter()
            .enumerate()
            .map(|(i, &ctid)| vec![tenant(i), PdbOwnedValue::U64(ctid)])
            .collect();
        let tree = KdTree::from_sample(dims(&["tenant", "ctid"]), sample.clone(), 4, None);
        assert_eq!(tree.partition_count(), 4, "{tree}");
        check_invariants(&tree, &sample, None);
        assert_eq!(
            tree.split_dims(),
            vec![
                DimHandle::Index(1),
                DimHandle::Index(0),
                DimHandle::Index(0)
            ],
            "{tree}"
        );

        // Fallback: the same rows, but ctid only as the fallback. Tenant is cut first even
        // though ctid is wider, and ctid only splits the halves once each holds one tenant.
        let sample: Vec<Point> = (0..ctids.len()).map(|i| vec![tenant(i)]).collect();
        let tree = KdTree::from_sample(dims(&["tenant"]), sample.clone(), 4, Some(ctids.clone()));
        assert_eq!(tree.partition_count(), 4, "{tree}");
        check_invariants(&tree, &sample, Some(&ctids));
        assert_eq!(
            tree.split_dims(),
            vec![DimHandle::Index(0), DimHandle::Ctid, DimHandle::Ctid],
            "{tree}"
        );
    }

    #[test]
    fn ctid_fallback_is_unused_while_declared_dims_can_still_cut() {
        // Ctids scattered across the heap, unique and uncorrelated with the declared values
        // (7919 is prime, so it permutes 0..n), and always wider than any declared dimension.
        let scattered_ctids =
            |n: u64| -> Vec<u64> { (0..n).map(|i| (((i * 7919) % n) << 16) | 1).collect() };
        let check = |names: &[&str], sample: Vec<Point>, target: usize| {
            let ctids = scattered_ctids(sample.len() as u64);
            let tree =
                KdTree::from_sample(dims(names), sample.clone(), target, Some(ctids.clone()));
            assert_eq!(tree.partition_count(), target, "{tree}");
            check_invariants(&tree, &sample, Some(&ctids));
            let splits = tree.split_dims();
            assert_eq!(splits.len(), target - 1, "{tree}");
            assert!(
                !splits.contains(&DimHandle::Ctid),
                "the fallback cut while a declared dimension could: {splits:?}\n{tree}"
            );
        };

        // A unique key.
        check(&["id"], i64s(0..1000), 8);
        // Many duplicates, but every leaf still holds several distinct values.
        check(&["tenant"], i64s((0..1000).map(|i| i % 100)), 8);
        // Two declared dimensions: they share the cuts between them, and ctid takes none.
        check(&["x", "y"], grid(40), 4);
    }

    #[test]
    fn ctid_fallback_partitions_share_their_declared_box() {
        // 990 rows on tenant 0 and ten cold tenants 1..=10. Tenant only cuts at 1, so the ctid
        // fallback splits the hot rows across the remaining partitions.
        let mut sample = i64s(std::iter::repeat_n(0, 990));
        sample.extend(i64s(1..=10));
        let ctids = heap_ctids(sample.len());
        let tree = KdTree::from_sample(dims(&["tenant"]), sample.clone(), 8, Some(ctids.clone()));
        assert_eq!(tree.partition_count(), 8, "{tree}");
        // This also checks that every row's ctid falls in its partition's ctid range.
        check_invariants(&tree, &sample, Some(&ctids));

        // On the declared dimension, every hot partition has the same box: a segment's box
        // can't tell them apart, so pruning on `tenant` must keep all of them together.
        let hot_box = (Bound::Unbounded, Bound::Excluded(PdbOwnedValue::I64(1)));
        let cold_box = (Bound::Included(PdbOwnedValue::I64(1)), Bound::Unbounded);
        for p in 0..7 {
            let bounds = tree.partition_bounds(p).unwrap();
            assert_eq!(bounds.dims[0], hot_box, "partition {p}\n{tree}");
            // The fallback's cut shows up as in the ctid_fallback
            assert!(
                bounds.ctid_fallback.is_some(),
                "partition {p}: {bounds:?}\n{tree}"
            );
        }
        assert_eq!(
            tree.partition_bounds(7).unwrap(),
            PartitionBounds::new(vec![cold_box]),
            "{tree}"
        );
    }

    #[test]
    fn ctid_fallback_splits_a_mostly_null_key() {
        // A heap of 1000 rows, 50 to a block, with ctids encoded as the build encodes them.
        // Nine rows in ten have a NULL key. NULLs are all equal and a split value is never NULL,
        // so the key alone can only cut them off from the rest; the ctid fallback has to split
        // the NULLs themselves, by where they sit in the heap.
        let tid = |block: u64, offset: u64| (block << 16) | offset;
        let ctids = heap_ctids(1000);
        let sample: Vec<Point> = (0..1000i64)
            .map(|i| {
                vec![if i % 10 == 0 {
                    PdbOwnedValue::I64(i)
                } else {
                    PdbOwnedValue::Null
                }]
            })
            .collect();
        let tree = KdTree::from_sample(dims(&["k"]), sample.clone(), 8, Some(ctids.clone()));
        assert_eq!(tree.partition_count(), 8, "{tree}");
        check_invariants(&tree, &sample, Some(&ctids));

        // The root separates the NULLs from the values at the smallest value. The 900 NULLs get
        // seven of the eight partitions, by their share of the rows, and the fallback cuts them.
        assert_eq!(
            tree.split_dims(),
            [vec![DimHandle::Index(0)], vec![DimHandle::Ctid; 6]].concat(),
            "{tree}"
        );
        let null_box = (Bound::Unbounded, Bound::Excluded(PdbOwnedValue::I64(0)));
        let is_null = |b: &Bound<PdbOwnedValue>| {
            matches!(
                b,
                Bound::Included(PdbOwnedValue::Null) | Bound::Excluded(PdbOwnedValue::Null)
            )
        };
        for p in 0..7 {
            let bounds = tree.partition_bounds(p).unwrap();
            assert_eq!(bounds.dims[0], null_box, "{tree}");
            assert!(
                bounds
                    .ctid_fallback
                    .filter(|(lo, hi)| !is_null(lo) && !is_null(hi))
                    .is_some()
            )
        }
        // No split value, on the key or on ctid, is NULL.
        assert!(
            (0..8)
                .flat_map(|p| tree.partition_bounds(p).unwrap().dims)
                .all(|(lo, hi)| !is_null(&lo) && !is_null(&hi)),
            "{tree}"
        );
        assert!(
            (0..8)
                .flat_map(|p| tree.partition_bounds(p).unwrap().ctid_fallback)
                .all(|(lo, hi)| !is_null(&lo) && !is_null(&hi)),
            "{tree}"
        );

        // Walking the heap in ctid order, the NULL rows fill partitions 0 through 6 in turn:
        // each partition is one contiguous stretch of the heap, of about 900 / 7 rows.
        let mut null_counts = vec![0usize; 7];
        let mut last = 0;
        for (point, &ctid) in sample.iter().zip(&ctids) {
            let p = tree.route(point, ctid);
            if matches!(point[0], PdbOwnedValue::Null) {
                assert!(
                    p < 7,
                    "NULL at ctid {ctid} routed to the values' partition\n{tree}"
                );
                assert!(
                    p >= last,
                    "NULL at ctid {ctid} went back to partition {p}\n{tree}"
                );
                last = p;
                null_counts[p] += 1;
            } else {
                assert_eq!(p, 7, "value {point:?} at ctid {ctid}\n{tree}");
            }
        }
        for count in &null_counts {
            assert!((127..=130).contains(count), "{null_counts:?}\n{tree}");
        }

        // Rows the sample never saw route by the same rules: a NULL at the very start of the
        // heap goes first, a NULL past its end (a block added after sampling) goes to the last
        // NULL partition, and a value goes to the values' partition wherever it sits.
        let null = [PdbOwnedValue::Null];
        assert_eq!(tree.route(&null, tid(0, 1)), 0);
        assert_eq!(tree.route(&null, tid(10_000, 1)), 6);
        assert_eq!(tree.route(&[PdbOwnedValue::I64(0)], tid(0, 1)), 7);
        assert_eq!(tree.route(&[PdbOwnedValue::I64(5)], tid(10, 7)), 7);
        // A value below the smallest sampled one falls in the NULLs' box, so it is split by
        // ctid alongside them: it lands wherever a NULL at the same ctid would.
        for ctid in [tid(0, 1), tid(10, 7), tid(19, 50)] {
            assert_eq!(
                tree.route(&[PdbOwnedValue::I64(-5)], ctid),
                tree.route(&null, ctid),
                "ctid {ctid}\n{tree}"
            );
        }
    }

    #[test]
    fn proportional_leaves_handle_lopsided_cuts() {
        let expected_splits = [vec![DimHandle::Index(0)], vec![DimHandle::Ctid; 6]].concat();
        let ctids = heap_ctids(1000);

        // The only cut is one row from the end: rounding would give the left all eight leaves,
        // so the clamp keeps one for the single row on the right (partition 7).
        let hot_then_cold = i64s(std::iter::repeat_n(0, 999).chain([1]));
        // The mirror image, one row from the start: rounding would give the left no leaf at
        // all, so the clamp gives it one (partition 0) and the other seven go right.
        let cold_then_hot = i64s(std::iter::once(0).chain(std::iter::repeat_n(1, 999)));
        for (sample, lone) in [(&hot_then_cold, 7), (&cold_then_hot, 0)] {
            let tree = KdTree::from_sample(dims(&["k"]), sample.clone(), 8, Some(ctids.clone()));
            assert_eq!(tree.partition_count(), 8, "{tree}");
            check_invariants(&tree, sample, Some(&ctids));
            assert_eq!(tree.split_dims(), expected_splits, "{tree}");
            let counts = route_counts(&tree, sample, &ctids);
            assert_eq!(counts[lone], 1, "{counts:?}\n{tree}");
            assert!(
                (0..8)
                    .filter(|&p| p != lone)
                    .all(|p| (141..=144).contains(&counts[p])),
                "{counts:?}\n{tree}"
            );
        }

        // Without the fallback the hot side cannot be cut, and its unused leaves do not turn
        // into extra partitions: one per side, never more than the target.
        let sample = cold_then_hot;
        let tree = KdTree::from_sample(dims(&["k"]), sample.clone(), 8, None);
        assert_eq!(tree.partition_count(), 2, "{tree}");
        check_invariants(&tree, &sample, None);
    }

    #[test]
    fn proportional_leaves_with_fewer_rows_than_leaves() {
        // Fewer rows than requested partitions: no division by zero, and at most one
        // partition per row.
        for n in [2usize, 3, 7] {
            let ctids = heap_ctids(n);

            let distinct = i64s(0..n as i64);
            for fallback in [None, Some(ctids.clone())] {
                let tree = KdTree::from_sample(dims(&["k"]), distinct.clone(), 8, fallback.clone());
                assert_eq!(tree.partition_count(), n, "{tree}");
                check_invariants(&tree, &distinct, fallback.as_deref());
            }

            // All equal: only the fallback can cut, and it cuts down to one row per partition.
            let equal = i64s(std::iter::repeat_n(7, n));
            let tree = KdTree::from_sample(dims(&["k"]), equal.clone(), 8, Some(ctids.clone()));
            assert_eq!(tree.partition_count(), n, "{tree}");
            check_invariants(&tree, &equal, Some(&ctids));
            let tree = KdTree::from_sample(dims(&["k"]), equal, 8, None);
            assert_eq!(tree.partition_count(), 1, "{tree}");
        }
    }

    #[test]
    fn proportional_leaves_keep_even_cuts_even() {
        // A cut that lands on its target gets the same leaves `k / 2` gave it, odd `k`
        // included, so a unique key still splits into equal partitions.
        let sample = i64s(0..1000);
        let ctids = heap_ctids(sample.len());
        for k in [2, 3, 5, 7, 8, 13] {
            let tree = KdTree::from_sample(dims(&["k"]), sample.clone(), k, Some(ctids.clone()));
            assert_eq!(tree.partition_count(), k, "{tree}");
            assert!(!tree.split_dims().contains(&DimHandle::Ctid), "{tree}");
            let ideal = sample.len() / k;
            let counts = route_counts(&tree, &sample, &ctids);
            assert!(
                counts.iter().all(|&c| c == ideal || c == ideal + 1),
                "k = {k}: {counts:?}\n{tree}"
            );
        }
    }

    #[test]
    fn proportional_leaves_never_exceed_the_target() {
        // Hot values at either end of the key, across skews and targets. With unique ctids
        // and more rows than leaves, the fallback always reaches the target exactly; without
        // it, the tree may fall short but never goes over.
        let n = 1000usize;
        let ctids = heap_ctids(n);
        for hot_rows in [500, 900, 990, 999] {
            let cold = (1..=(n - hot_rows) as i64).map(|v| v * 10);
            let hot_low = i64s(std::iter::repeat_n(0, hot_rows).chain(cold.clone()));
            let hot_high = i64s(cold.chain(std::iter::repeat_n(1_000_000, hot_rows)));
            for sample in [hot_low, hot_high] {
                for k in 2..=16 {
                    let tree =
                        KdTree::from_sample(dims(&["k"]), sample.clone(), k, Some(ctids.clone()));
                    assert_eq!(tree.partition_count(), k, "hot {hot_rows}, k {k}\n{tree}");
                    check_invariants(&tree, &sample, Some(&ctids));

                    let tree = KdTree::from_sample(dims(&["k"]), sample.clone(), k, None);
                    assert!(tree.partition_count() <= k, "hot {hot_rows}, k {k}\n{tree}");
                    check_invariants(&tree, &sample, None);
                }
            }
        }
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
            None,
        );
        assert_eq!(tree.partition_count(), 6, "{tree}");
        check_invariants(&tree, &sample, None);

        // The tree carries an `I64` split (`id` is unique, so it is the widest dimension); the
        // round trip must hand it back as `I64`, not as tantivy's untagged `U64`.
        let bytes = postcard::to_allocvec(&tree).unwrap();
        let back: KdTree = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(back, tree);
        for p in &sample {
            assert_eq!(back.route(p, 0), tree.route(p, 0));
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
        let tree = KdTree::from_sample(dims(&["x"]), sample.clone(), 6, None);
        check_invariants(&tree, &sample, None);

        // `PartialEq` treats `NaN != NaN`, so compare the re-serialized bytes rather than the
        // trees: equal bytes means every split value, `NaN` included, round-tripped bit for bit.
        let bytes = postcard::to_allocvec(&tree).unwrap();
        let back: KdTree = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(postcard::to_allocvec(&back).unwrap(), bytes);
        for probe in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN, 0.0, 199.0] {
            assert_eq!(
                back.route(&[PdbOwnedValue::F64(probe)], 0),
                tree.route(&[PdbOwnedValue::F64(probe)], 0),
                "probe {probe}"
            );
        }
    }

    #[test]
    fn display_lists_every_split_and_leaf() {
        let tree = KdTree::from_sample(dims(&["id"]), i64s(0..100), 3, None);
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
        let tree = KdTree::from_sample(dims(&["x", "y"]), grid(40), 4, None);
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
        let tree = KdTree::from_sample(dims(&["day", "key"]), sample, 2, None);
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
        let tree = KdTree::from_sample(dims(&["ctid"]), sample, 2, None);
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
}
