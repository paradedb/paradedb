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

//! Per-segment document-count and cost estimation (`QueryEstimate`) without building Tantivy
//! scorers.
//!
//! Callers pass an [`EstimateContext`] pairing a `SegmentReader` with a [`ColumnStatsProvider`]
//! (backed by PostgreSQL `pg_statistic` in production via [`PgColumnStats`], or an in-memory
//! fixture in unit tests).
//!
//! Inverted-index queries (`TermQuery`, `SynonymQuery`, `PhraseQuery`, `BooleanQuery`) derive
//! their estimates from segment `doc_freq` counts, combining `Occur::Must` clauses with
//! correlation backoff (`s_1 * s_2^0.5 * s_3^0.25 ...`) rather than naive independence.
//!
//! Range queries (`RangeQueryWithEstimate`) use a three-part strategy:
//! 1. Same-field `Occur::Must` range bounds inside a `BooleanQuery` are intersected into a single
//!    interval via [`QueryEstimate::as_range_clause`] before estimation (`CoalescedMustRange`).
//!    PostgreSQL lowers `col >= a AND col < b` into two separate range clauses; estimating them
//!    separately under correlation backoff would treat a narrow interval as the intersection of
//!    two wide half-open ranges.
//! 2. For analyzed scalar heap columns, PostgreSQL's `pg_statistic` (`null_frac`, MCVs, and
//!    equi-depth histogram) supplied by [`ColumnStatsProvider`] estimates the range's selectivity,
//!    conditioned on the segment's `.stats` `[empirical.min, empirical.max]`
//!    (`estimate_fraction_from_column_stats`) with outer histogram bin endpoints clamped to the
//!    segment's `[min, max]` (mirroring PostgreSQL's `ineq_histogram_selectivity`). Segment
//!    `.stats` alone cannot estimate nullable or skewed columns (`view_count`, `score`), while
//!    table-wide `pg_statistic` alone ignores `partition_by` or key-ordered segment value bounds.
//! 3. When `pg_statistic` is unavailable, not applicable (e.g. JSON subpaths), or unusable for the
//!    segment, estimation falls back to uniform interpolation over non-nullable segment `.stats`
//!    (`estimate_uniform_from_empirical`), or returns `None` so callers fall back to Tantivy
//!    scorer `size_hint`.

use std::cell::RefCell;
use std::cmp::Ordering;
use std::ops::Bound;
use std::sync::Arc;

use crate::api::HashMap;
use crate::index::stats::{
    EmpiricalStats, SegmentStats, comparable, ends_before, lower_bound_ge, upper_bound_le,
};
use crate::postgres::pdb_owned_value::PdbOwnedValue;
#[cfg(not(test))]
use crate::schema::HeapStatTarget;
use crate::schema::{SearchField, SearchFieldType, SearchIndexSchema};
use tantivy::SegmentReader;
use tantivy::query::{
    AllQuery, BooleanQuery, BoostQuery, ConstScoreQuery, EmptyQuery, EnableScoring, Occur,
    PhraseQuery, Query, RangeQuery, SynonymQuery, TermQuery, Weight,
};
use tantivy::schema::Field;

/// Wraps a Tantivy `RangeQuery` with its logical field metadata and canonicalized bounds for
/// [`QueryEstimate`].
#[derive(Clone, Debug)]
pub(super) struct RangeQueryWithEstimate {
    query: RangeQuery,
    field: SearchField,
    field_path: Option<String>,
    lower_bound: Bound<PdbOwnedValue>,
    upper_bound: Bound<PdbOwnedValue>,
}

impl RangeQueryWithEstimate {
    pub(super) fn new(
        query: RangeQuery,
        field: SearchField,
        field_path: Option<String>,
        bounds: (Bound<PdbOwnedValue>, Bound<PdbOwnedValue>),
    ) -> Self {
        Self {
            query,
            field,
            field_path,
            lower_bound: bounds.0,
            upper_bound: bounds.1,
        }
    }
}

/// Table-level distribution summary for one scalar column (e.g. from PostgreSQL `pg_statistic`).
#[derive(Clone, Debug)]
pub struct ColumnStats {
    null_frac: f64,
    mcvs: Vec<(PdbOwnedValue, f64)>,
    histogram: Vec<PdbOwnedValue>,
}

/// Supplies table-level column distribution statistics to [`QueryEstimate`] implementations.
pub trait ColumnStatsProvider {
    fn column_stats(
        &self,
        field: &SearchField,
        field_path: Option<&str>,
    ) -> Option<Arc<ColumnStats>>;
}

/// [`ColumnStatsProvider`] that reports no table-level statistics.
#[cfg(test)]
struct NoColumnStats;

#[cfg(test)]
impl ColumnStatsProvider for NoColumnStats {
    fn column_stats(
        &self,
        _field: &SearchField,
        _field_path: Option<&str>,
    ) -> Option<Arc<ColumnStats>> {
        None
    }
}

/// [`ColumnStatsProvider`] backed by PostgreSQL's `pg_statistic` catalog for a
/// [`SearchIndexSchema`], caching lookups by Tantivy [`Field`] across clauses and segments.
///
/// TODO(<https://github.com/paradedb/paradedb/issues/6976>): Replace `pg_statistic` syscache
/// lookups once ParadeDB-owned index-wide field summaries land.
pub struct PgColumnStats<'a> {
    schema: &'a SearchIndexSchema,
    cache: RefCell<HashMap<Field, Option<Arc<ColumnStats>>>>,
}

impl<'a> PgColumnStats<'a> {
    pub fn new(schema: &'a SearchIndexSchema) -> Self {
        Self {
            schema,
            cache: RefCell::new(HashMap::default()),
        }
    }
}

impl ColumnStatsProvider for PgColumnStats<'_> {
    fn column_stats(
        &self,
        field: &SearchField,
        field_path: Option<&str>,
    ) -> Option<Arc<ColumnStats>> {
        if field_path.is_some() || !is_supported_stat_field(field) {
            return None;
        }
        if let Some(cached) = self.cache.borrow().get(&field.field()) {
            return cached.clone();
        }
        let loaded = self
            .schema
            .heap_stat_target(field.field_name())
            .and_then(load_pg_column_stats);
        self.cache
            .borrow_mut()
            .insert(field.field(), loaded.clone());
        loaded
    }
}

/// Per-segment estimation context passed to [`QueryEstimate::estimate_docs`].
#[derive(Clone, Copy)]
pub struct EstimateContext<'a> {
    pub reader: &'a SegmentReader,
    pub stats: &'a dyn ColumnStatsProvider,
}

impl<'a> EstimateContext<'a> {
    pub fn new(reader: &'a SegmentReader, stats: &'a dyn ColumnStatsProvider) -> Self {
        Self { reader, stats }
    }

    #[cfg(test)]
    fn without_column_stats(reader: &'a SegmentReader) -> Self {
        Self {
            reader,
            stats: &NoColumnStats,
        }
    }
}

/// Borrowed view of a single-field range constraint exposed through [`QueryEstimate::as_range_clause`]
/// so `BooleanQuery` can coalesce multiple `Occur::Must` bounds on the same field without
/// downcasting concrete query wrappers.
#[derive(Clone, Copy, Debug)]
pub struct RangeClause<'a> {
    field: &'a SearchField,
    field_path: Option<&'a str>,
    bounds: RangeBoundsRef<'a>,
}

#[derive(Clone, Copy, Debug)]
struct RangeBoundsRef<'a> {
    lower: &'a Bound<PdbOwnedValue>,
    upper: &'a Bound<PdbOwnedValue>,
}

#[derive(Clone, Copy, Debug)]
enum CdfCutoff {
    Below,
    AtOrBelow,
}

impl RangeBoundsRef<'_> {
    fn contains(self, value: &PdbOwnedValue) -> Option<bool> {
        let pt = Bound::Included(value.clone());
        if !bound_pair_comparable(&pt, self.lower) || !bound_pair_comparable(&pt, self.upper) {
            return None;
        }
        let pt_ref = Bound::Included(value);
        Some(
            lower_bound_ge(pt_ref, self.lower.as_ref())
                && upper_bound_le(pt_ref, self.upper.as_ref()),
        )
    }
}

fn scalar_diff(a: &PdbOwnedValue, b: &PdbOwnedValue) -> Option<f64> {
    let diff = match (a, b) {
        (PdbOwnedValue::I64(a), PdbOwnedValue::I64(b)) => (i128::from(*a) - i128::from(*b)) as f64,
        (PdbOwnedValue::U64(a), PdbOwnedValue::U64(b)) => (i128::from(*a) - i128::from(*b)) as f64,
        (PdbOwnedValue::I64(a), PdbOwnedValue::U64(b)) => (i128::from(*a) - i128::from(*b)) as f64,
        (PdbOwnedValue::U64(a), PdbOwnedValue::I64(b)) => (i128::from(*a) - i128::from(*b)) as f64,
        (PdbOwnedValue::Date(a), PdbOwnedValue::Date(b)) => {
            (i128::from(a.into_inner()) - i128::from(b.into_inner())) as f64
        }
        (PdbOwnedValue::F64(a), PdbOwnedValue::F64(b)) => a - b,
        _ => return None,
    };
    diff.is_finite().then_some(diff)
}

fn discrete_step(sample: &PdbOwnedValue) -> f64 {
    if matches!(sample, PdbOwnedValue::F64(_)) {
        0.0
    } else {
        1.0
    }
}

impl ColumnStats {
    /// Estimates `P(X in bounds)` across all rows (including NULLs), summing matching MCV
    /// frequencies and interpolating the remaining `1 - null_frac - sum(mcv_freqs)` mass across
    /// the equi-depth histogram (mirroring PostgreSQL's `clauselist_selectivity` /
    /// `ineq_histogram_selectivity`).
    fn estimate_range_selectivity(&self, bounds: RangeBoundsRef<'_>) -> Option<f64> {
        self.estimate_range_selectivity_with_actual(bounds, None)
    }

    fn estimate_range_selectivity_with_actual(
        &self,
        bounds: RangeBoundsRef<'_>,
        actual_range: Option<&EmpiricalStats>,
    ) -> Option<f64> {
        if ends_before(bounds.upper.as_ref(), bounds.lower.as_ref()) {
            return Some(0.0);
        }
        if matches!(
            (bounds.lower, bounds.upper),
            (Bound::Unbounded, Bound::Unbounded)
        ) {
            return Some((1.0 - self.null_frac).clamp(0.0, 1.0));
        }
        for bound in [bounds.lower, bounds.upper] {
            if let Bound::Included(PdbOwnedValue::F64(v)) | Bound::Excluded(PdbOwnedValue::F64(v)) =
                bound
                && !v.is_finite()
            {
                return None;
            }
        }
        let mut mcv_total = 0.0;
        let mut mcv_match = 0.0;
        for (val, freq) in &self.mcvs {
            mcv_total += *freq;
            if bounds.contains(val)? {
                mcv_match += *freq;
            }
        }
        if self.null_frac >= 1.0 - 1e-9 {
            return Some(0.0);
        }
        let other_frac = (1.0 - self.null_frac - mcv_total).max(0.0);
        // When MCVs account for the entire non-null population, PostgreSQL omits the histogram
        // slot; `stanullfrac` + `MCV` `float4` rounding can leave a tiny positive residual.
        if (!self.mcvs.is_empty() && other_frac < 1e-4 && self.histogram.len() < 2)
            || other_frac <= 1e-9
        {
            return Some(mcv_match.clamp(0.0, 1.0));
        }
        if self.histogram.len() < 2 {
            return None;
        }
        let cdf_lo = match bounds.lower {
            Bound::Unbounded => 0.0,
            Bound::Included(v) => self.hist_cdf(CdfCutoff::Below, v, actual_range)?,
            Bound::Excluded(v) => self.hist_cdf(CdfCutoff::AtOrBelow, v, actual_range)?,
        };
        let cdf_hi = match bounds.upper {
            Bound::Unbounded => 1.0,
            Bound::Included(v) => self.hist_cdf(CdfCutoff::AtOrBelow, v, actual_range)?,
            Bound::Excluded(v) => self.hist_cdf(CdfCutoff::Below, v, actual_range)?,
        };
        let hist_frac = (cdf_hi - cdf_lo).clamp(0.0, 1.0);
        Some((mcv_match + other_frac * hist_frac).clamp(0.0, 1.0))
    }

    /// Fraction of the equi-depth histogram strictly below (`Below`) or at-or-below (`AtOrBelow`)
    /// `value`. Bucket `i` spans `[b_i, b_{i+1})`, except the final bucket `[b_{n-1}, b_n]` which
    /// includes the maximum endpoint (`+ step` on discrete integer/date domains so adjacent
    /// intervals partition `[b_0, b_n]` additively).
    fn hist_cdf(
        &self,
        cutoff: CdfCutoff,
        value: &PdbOwnedValue,
        actual_range: Option<&EmpiricalStats>,
    ) -> Option<f64> {
        let nbuckets = self.histogram.len().checked_sub(1)?;
        if nbuckets == 0 || !comparable(value, &self.histogram[0]) {
            return None;
        }
        let step = discrete_step(&self.histogram[0]);
        let mut sum = 0.0;
        for idx in 0..nbuckets {
            let mut lo = &self.histogram[idx];
            let mut hi = &self.histogram[idx + 1];
            if let Some(emp) = actual_range {
                if idx == 0 && comparable(&emp.min, lo) && emp.min.total_cmp(lo) == Ordering::Less {
                    lo = &emp.min;
                }
                if idx + 1 == nbuckets
                    && comparable(&emp.max, hi)
                    && emp.max.total_cmp(hi) == Ordering::Greater
                {
                    hi = &emp.max;
                }
            }
            let bucket_extra = if idx + 1 == nbuckets { step } else { 0.0 };
            let (before_lo, past_hi, cutoff_step) = match cutoff {
                CdfCutoff::Below => (
                    value.total_cmp(lo) != Ordering::Greater,
                    bucket_extra == 0.0 && value.total_cmp(hi) != Ordering::Less,
                    0.0,
                ),
                CdfCutoff::AtOrBelow => (
                    value.total_cmp(lo) == Ordering::Less,
                    value.total_cmp(hi) != Ordering::Less,
                    step,
                ),
            };
            sum += if before_lo {
                0.0
            } else if past_hi {
                1.0
            } else {
                let span = scalar_diff(hi, lo)? + bucket_extra;
                if span <= 0.0 {
                    1.0
                } else {
                    ((scalar_diff(value, lo)? + cutoff_step) / span).clamp(0.0, 1.0)
                }
            };
        }
        Some((sum / nbuckets as f64).clamp(0.0, 1.0))
    }
}

fn is_supported_stat_field(field: &SearchField) -> bool {
    use pgrx::pg_sys;
    let oid = match field.field_type() {
        SearchFieldType::I64(oid)
        | SearchFieldType::U64(oid)
        | SearchFieldType::F64(oid)
        | SearchFieldType::Date(oid) => oid,
        _ => return false,
    };
    matches!(
        oid,
        pg_sys::INT2OID
            | pg_sys::INT4OID
            | pg_sys::INT8OID
            | pg_sys::OIDOID
            | pg_sys::FLOAT4OID
            | pg_sys::FLOAT8OID
            | pg_sys::DATEOID
            | pg_sys::TIMESTAMPOID
            | pg_sys::TIMESTAMPTZOID
    )
}

#[cfg(test)]
fn load_pg_column_stats(_target: crate::schema::HeapStatTarget) -> Option<Arc<ColumnStats>> {
    None
}

#[cfg(not(test))]
fn load_pg_column_stats(target: HeapStatTarget) -> Option<Arc<ColumnStats>> {
    use crate::postgres::datetime::PostgresDateTime;
    use pgrx::{FromDatum, pg_sys};

    unsafe fn datum_to_stat_value(
        datum: pg_sys::Datum,
        typoid: pg_sys::Oid,
    ) -> Option<PdbOwnedValue> {
        unsafe {
            match typoid {
                pg_sys::INT2OID => {
                    i16::from_datum(datum, false).map(|v| PdbOwnedValue::I64(i64::from(v)))
                }
                pg_sys::INT4OID => {
                    i32::from_datum(datum, false).map(|v| PdbOwnedValue::I64(i64::from(v)))
                }
                pg_sys::INT8OID => i64::from_datum(datum, false).map(PdbOwnedValue::I64),
                pg_sys::OIDOID => {
                    u32::from_datum(datum, false).map(|v| PdbOwnedValue::U64(u64::from(v)))
                }
                pg_sys::FLOAT4OID => f32::from_datum(datum, false)
                    .map(f64::from)
                    .filter(|v| v.is_finite())
                    .map(PdbOwnedValue::F64),
                pg_sys::FLOAT8OID => f64::from_datum(datum, false)
                    .filter(|v| v.is_finite())
                    .map(PdbOwnedValue::F64),
                pg_sys::DATEOID => pgrx::datum::Date::from_datum(datum, false)
                    .and_then(|d| PostgresDateTime::try_from(d).ok())
                    .map(PdbOwnedValue::Date),
                pg_sys::TIMESTAMPOID => pgrx::datum::Timestamp::from_datum(datum, false)
                    .map(|ts| PdbOwnedValue::Date(PostgresDateTime::from(ts))),
                pg_sys::TIMESTAMPTZOID => {
                    pgrx::datum::TimestampWithTimeZone::from_datum(datum, false)
                        .map(|ts| PdbOwnedValue::Date(PostgresDateTime::from(ts)))
                }
                _ => None,
            }
        }
    }

    unsafe fn read_stat_slot<T>(
        tuple: pg_sys::HeapTuple,
        kind: u32,
        flags: u32,
        mut map_entry: impl FnMut(&pg_sys::AttStatsSlot, usize, PdbOwnedValue) -> T,
    ) -> Option<Vec<T>> {
        unsafe {
            let mut slot = pg_sys::AttStatsSlot::default();
            if !pg_sys::get_attstatsslot(
                &mut slot,
                tuple,
                kind as _,
                pg_sys::InvalidOid,
                flags as _,
            ) {
                return Some(Vec::new());
            }
            let count = if (flags & pg_sys::ATTSTATSSLOT_NUMBERS) != 0 {
                slot.nvalues.min(slot.nnumbers).max(0) as usize
            } else {
                slot.nvalues.max(0) as usize
            };
            let mut entries = Vec::with_capacity(count);
            let mut ok = true;
            for idx in 0..count {
                let datum = *slot.values.add(idx);
                if let Some(val) = datum_to_stat_value(datum, slot.valuetype) {
                    entries.push(map_entry(&slot, idx, val));
                } else {
                    ok = false;
                    break;
                }
            }
            pg_sys::free_attstatsslot(&mut slot);
            ok.then_some(entries)
        }
    }

    crate::postgres::customscan::mpp::run_on_main_thread(|_| unsafe {
        let tuple = pg_sys::SearchSysCache3(
            pg_sys::SysCacheIdentifier::STATRELATTINH as _,
            target.relid.into(),
            pg_sys::Datum::from(target.attnum),
            false.into(),
        );
        if tuple.is_null() {
            return None;
        }
        let result = (|| {
            if (*tuple).t_data.is_null() {
                return None;
            }
            let stat = &*pgrx::GETSTRUCT!(tuple)
                .get()
                .cast::<pg_sys::FormData_pg_statistic>();
            let null_frac = f64::from(stat.stanullfrac).clamp(0.0, 1.0);
            let mcvs = read_stat_slot(
                tuple,
                pg_sys::STATISTIC_KIND_MCV,
                pg_sys::ATTSTATSSLOT_VALUES | pg_sys::ATTSTATSSLOT_NUMBERS,
                |slot, idx, val| (val, f64::from(*slot.numbers.add(idx)).max(0.0)),
            )?;
            let histogram = read_stat_slot(
                tuple,
                pg_sys::STATISTIC_KIND_HISTOGRAM,
                pg_sys::ATTSTATSSLOT_VALUES,
                |_, _, val| val,
            )?;

            Some(Arc::new(ColumnStats {
                null_frac,
                mcvs,
                histogram,
            }))
        })();
        pg_sys::ReleaseSysCache(tuple);
        result
    })
}

fn bound_pair_comparable(a: &Bound<PdbOwnedValue>, b: &Bound<PdbOwnedValue>) -> bool {
    match (a, b) {
        (Bound::Included(x) | Bound::Excluded(x), Bound::Included(y) | Bound::Excluded(y)) => {
            comparable(x, y)
        }
        _ => true,
    }
}

fn ranges_comparable(a: RangeBoundsRef<'_>, b: RangeBoundsRef<'_>) -> bool {
    bound_pair_comparable(a.lower, b.lower)
        && bound_pair_comparable(a.lower, b.upper)
        && bound_pair_comparable(a.upper, b.lower)
        && bound_pair_comparable(a.upper, b.upper)
}

fn tighter_lower(a: &Bound<PdbOwnedValue>, b: &Bound<PdbOwnedValue>) -> Bound<PdbOwnedValue> {
    if lower_bound_ge(b.as_ref(), a.as_ref()) {
        b.clone()
    } else {
        a.clone()
    }
}

fn tighter_upper(a: &Bound<PdbOwnedValue>, b: &Bound<PdbOwnedValue>) -> Bound<PdbOwnedValue> {
    if upper_bound_le(b.as_ref(), a.as_ref()) {
        b.clone()
    } else {
        a.clone()
    }
}

#[cfg(test)]
impl ColumnStats {
    /// Returns `(stat_min, stat_max)` across all MCVs and histogram bounds when non-empty and
    /// mutually comparable.
    fn value_domain(&self) -> Option<(&PdbOwnedValue, &PdbOwnedValue)> {
        let mut iter = self
            .mcvs
            .iter()
            .map(|(v, _)| v)
            .chain(self.histogram.iter());
        let first = iter.next()?;
        let mut min_val = first;
        let mut max_val = first;
        for val in iter {
            if !comparable(val, first) {
                return None;
            }
            if val.total_cmp(min_val) == Ordering::Less {
                min_val = val;
            }
            if val.total_cmp(max_val) == Ordering::Greater {
                max_val = val;
            }
        }
        Some((min_val, max_val))
    }
}

/// Conditions table-wide `ColumnStats` on this segment's observed `[empirical.min, empirical.max]`
/// and nullability, clamping the outermost histogram bin endpoints to the segment's `[min, max]`
/// (mirroring PostgreSQL's `ineq_histogram_selectivity`) when the segment extends past the
/// analyzed histogram.
fn estimate_fraction_from_column_stats(
    col_stats: &ColumnStats,
    empirical: Option<&EmpiricalStats>,
    bounds: RangeBoundsRef<'_>,
) -> Option<f64> {
    let Some(empirical_stats) = empirical else {
        return col_stats.estimate_range_selectivity(bounds);
    };
    for bound in [bounds.lower, bounds.upper] {
        if let Bound::Included(PdbOwnedValue::F64(v)) | Bound::Excluded(PdbOwnedValue::F64(v)) =
            bound
            && !v.is_finite()
        {
            return None;
        }
    }
    if col_stats.null_frac >= 1.0 - 1e-9 {
        return None;
    }
    let stat_sample = col_stats
        .histogram
        .first()
        .or_else(|| col_stats.mcvs.first().map(|(v, _)| v))?;
    let segment_lower = Bound::Included(empirical_stats.min.clone());
    let segment_upper = Bound::Included(empirical_stats.max.clone());
    let segment_bounds = RangeBoundsRef {
        lower: &segment_lower,
        upper: &segment_upper,
    };
    if !comparable(&empirical_stats.min, stat_sample)
        || !comparable(&empirical_stats.max, stat_sample)
        || !ranges_comparable(segment_bounds, bounds)
    {
        return col_stats.estimate_range_selectivity(bounds);
    }
    if col_stats.estimate_range_selectivity(segment_bounds)? <= 0.0 {
        // Stale or out-of-sample statistics assign zero mass to this segment's `[min, max]`;
        // let the caller fall back to empirical segment bounds.
        return None;
    }
    let non_null_in_segment =
        col_stats.estimate_range_selectivity_with_actual(segment_bounds, Some(empirical_stats))?;
    if non_null_in_segment <= 0.0 {
        return None;
    }
    let intersected_lower = tighter_lower(&segment_lower, bounds.lower);
    let intersected_upper = tighter_upper(&segment_upper, bounds.upper);
    let numerator = col_stats.estimate_range_selectivity_with_actual(
        RangeBoundsRef {
            lower: &intersected_lower,
            upper: &intersected_upper,
        },
        Some(empirical_stats),
    )?;
    let non_null_ratio = numerator / non_null_in_segment;
    let fraction = if empirical_stats.nullable {
        // Spread `null_frac` across nullable segments (`P(non-null | segment) = 1 - null_frac`).
        // TODO: Consult `SegmentStats::logical(field)` when `partition_by` splits on this field,
        // where partitioned builds route all table `NULL`s into the bottom partition
        // (`logical.lower == Unbounded && logical.upper != Unbounded`).
        non_null_ratio * (1.0 - col_stats.null_frac).clamp(0.0, 1.0)
    } else {
        non_null_ratio
    };
    Some(fraction.clamp(0.0, 1.0))
}

/// Fallback uniform interpolation over segment `.stats` `[min, max]` when `pg_statistic` is
/// unavailable or stale. Only valid on non-nullable single-valued segments.
fn estimate_uniform_from_empirical(
    stats: &EmpiricalStats,
    bounds: RangeBoundsRef<'_>,
) -> Option<f64> {
    let span = scalar_diff(&stats.max, &stats.min)?;
    let step = discrete_step(&stats.min);
    let lower_offset = match bounds.lower {
        Bound::Included(value) => scalar_diff(value, &stats.min)?,
        Bound::Excluded(value) => scalar_diff(value, &stats.min)? + step,
        Bound::Unbounded => 0.0,
    };
    let upper_offset = match bounds.upper {
        Bound::Included(value) => scalar_diff(value, &stats.min)? + step,
        Bound::Excluded(value) => scalar_diff(value, &stats.min)?,
        Bound::Unbounded => span + step,
    };
    if !stats.intersects(bounds.lower, bounds.upper) || lower_offset > upper_offset {
        Some(0.0)
    } else if stats.nullable {
        // Nullability also covers multivalued fields; min/max alone cannot estimate their counts.
        None
    } else if span == 0.0 {
        Some(1.0)
    } else {
        Some(
            ((upper_offset.min(span + step) - lower_offset.max(0.0)) / (span + step))
                .clamp(0.0, 1.0),
        )
    }
}

fn estimate_range_docs(
    field: &SearchField,
    field_path: Option<&str>,
    bounds: RangeBoundsRef<'_>,
    ctx: &EstimateContext<'_>,
) -> tantivy::Result<Option<Estimate>> {
    let reader = ctx.reader;
    if ends_before(bounds.upper.as_ref(), bounds.lower.as_ref()) {
        return Ok(Some(Estimate {
            matches: 0.0,
            cost: 0,
        }));
    }
    let empirical = SegmentStats::of_reader(reader)?
        .map(|stats| stats.empirical_for(field))
        .transpose()?
        .flatten();
    if empirical
        .as_ref()
        .is_some_and(|emp| !emp.intersects(bounds.lower, bounds.upper))
    {
        return Ok(Some(Estimate {
            matches: 0.0,
            cost: 0,
        }));
    }
    let max_doc = f64::from(reader.max_doc());
    let fraction = ctx
        .stats
        .column_stats(field, field_path)
        .and_then(|cs| estimate_fraction_from_column_stats(&cs, empirical.as_ref(), bounds))
        .or_else(|| {
            empirical
                .as_ref()
                .and_then(|emp| estimate_uniform_from_empirical(emp, bounds))
        });
    Ok(fraction.map(|f| {
        let matches = (f * max_doc).clamp(0.0, max_doc);
        Estimate {
            matches,
            cost: if matches <= 0.0 {
                0
            } else {
                u64::from(reader.max_doc())
            },
        }
    }))
}

impl Query for RangeQueryWithEstimate {
    fn weight(&self, enable_scoring: EnableScoring<'_>) -> tantivy::Result<Box<dyn Weight>> {
        self.query.weight(enable_scoring)
    }
}

pub trait QueryEstimate {
    fn is_fast_count(&self) -> bool {
        false
    }

    fn as_range_clause(&self) -> Option<RangeClause<'_>> {
        None
    }

    fn estimate_docs(&self, ctx: &EstimateContext<'_>) -> tantivy::Result<Option<Estimate>>;
}

pub struct Estimate {
    matches: f64,
    pub cost: u64,
}

impl Estimate {
    pub fn live_docs(&self, reader: &SegmentReader) -> u64 {
        let live_fraction = reader.num_docs() as f64 / reader.max_doc().max(1) as f64;
        ((self.matches * live_fraction).ceil() as u64).min(reader.num_docs() as u64)
    }
}

impl QueryEstimate for dyn Query {
    fn is_fast_count(&self) -> bool {
        self.is::<AllQuery>()
            || self.is::<TermQuery>()
            || self.is::<ConstScoreQuery<AllQuery>>()
            || self
                .downcast_ref::<BooleanQuery>()
                .is_some_and(QueryEstimate::is_fast_count)
            || self
                .downcast_ref::<BoostQuery>()
                .is_some_and(QueryEstimate::is_fast_count)
            || self
                .downcast_ref::<ConstScoreQuery>()
                .is_some_and(QueryEstimate::is_fast_count)
    }

    fn as_range_clause(&self) -> Option<RangeClause<'_>> {
        macro_rules! range_clause_as {
            ($($query:ty),* $(,)?) => {$(
                if let Some(query) = self.downcast_ref::<$query>() {
                    return query.as_range_clause();
                }
            )*};
        }
        range_clause_as! {
            RangeQueryWithEstimate, ConstScoreQuery, BoostQuery, BooleanQuery
        }
        None
    }

    fn estimate_docs(&self, ctx: &EstimateContext<'_>) -> tantivy::Result<Option<Estimate>> {
        macro_rules! estimate_as {
            ($($query:ty),* $(,)?) => {$(
                if let Some(query) = self.downcast_ref::<$query>() {
                    return query.estimate_docs(ctx);
                }
            )*};
        }
        estimate_as! {
            AllQuery, TermQuery, SynonymQuery, PhraseQuery, BooleanQuery, EmptyQuery,
            ConstScoreQuery, BoostQuery, RangeQueryWithEstimate
        }
        // SearchQueryInput::All wraps AllQuery directly, without boxing it.
        if let Some(query) = self.downcast_ref::<ConstScoreQuery<AllQuery>>() {
            return query.query().estimate_docs(ctx);
        }
        Ok(None)
    }
}

impl QueryEstimate for RangeQueryWithEstimate {
    fn as_range_clause(&self) -> Option<RangeClause<'_>> {
        Some(RangeClause {
            field: &self.field,
            field_path: self.field_path.as_deref(),
            bounds: RangeBoundsRef {
                lower: &self.lower_bound,
                upper: &self.upper_bound,
            },
        })
    }

    fn estimate_docs(&self, ctx: &EstimateContext<'_>) -> tantivy::Result<Option<Estimate>> {
        estimate_range_docs(
            &self.field,
            self.field_path.as_deref(),
            RangeBoundsRef {
                lower: &self.lower_bound,
                upper: &self.upper_bound,
            },
            ctx,
        )
    }
}

impl QueryEstimate for AllQuery {
    fn is_fast_count(&self) -> bool {
        true
    }

    fn estimate_docs(&self, ctx: &EstimateContext<'_>) -> tantivy::Result<Option<Estimate>> {
        Ok(Some(Estimate {
            matches: ctx.reader.max_doc() as f64,
            cost: ctx.reader.max_doc() as u64,
        }))
    }
}

impl QueryEstimate for ConstScoreQuery {
    fn is_fast_count(&self) -> bool {
        self.query().is_fast_count()
    }

    fn as_range_clause(&self) -> Option<RangeClause<'_>> {
        self.query().as_range_clause()
    }

    fn estimate_docs(&self, ctx: &EstimateContext<'_>) -> tantivy::Result<Option<Estimate>> {
        self.query().estimate_docs(ctx)
    }
}

impl QueryEstimate for BoostQuery {
    fn is_fast_count(&self) -> bool {
        self.query().is_fast_count()
    }

    fn as_range_clause(&self) -> Option<RangeClause<'_>> {
        self.query().as_range_clause()
    }

    fn estimate_docs(&self, ctx: &EstimateContext<'_>) -> tantivy::Result<Option<Estimate>> {
        self.query().estimate_docs(ctx)
    }
}

impl QueryEstimate for TermQuery {
    fn is_fast_count(&self) -> bool {
        true
    }

    fn estimate_docs(&self, ctx: &EstimateContext<'_>) -> tantivy::Result<Option<Estimate>> {
        // The index already stores how many documents contain this term.
        let reader = ctx.reader;
        let term = self.term();
        if !reader.schema().get_field_entry(term.field()).is_indexed() {
            return Ok(None);
        }
        let count = reader.inverted_index(term.field())?.doc_freq(term)?;
        Ok(Some(Estimate {
            matches: count as f64,
            cost: count as u64,
        }))
    }
}

impl QueryEstimate for SynonymQuery {
    fn estimate_docs(&self, ctx: &EstimateContext<'_>) -> tantivy::Result<Option<Estimate>> {
        // A group matches a document that has any of its terms, like an OR of the terms.
        let reader = ctx.reader;
        let field = self.terms()[0].field();
        if !reader.schema().get_field_entry(field).is_indexed() {
            return Ok(None);
        }
        let inverted_index = reader.inverted_index(field)?;
        let total = reader.max_doc().max(1) as f64;
        let mut no_match = 1.0;
        let mut cost = 0u64;
        for term in self.terms() {
            let count = inverted_index.doc_freq(term)?;
            no_match *= 1.0 - count as f64 / total;
            cost = cost.saturating_add(count as u64);
        }
        Ok(Some(Estimate {
            matches: (total * (1.0 - no_match)).clamp(0.0, total),
            cost,
        }))
    }
}

impl QueryEstimate for PhraseQuery {
    fn estimate_docs(&self, ctx: &EstimateContext<'_>) -> tantivy::Result<Option<Estimate>> {
        // Multiply the fraction of documents containing each term, then scale back to a count.
        // Divide by 10 times the phrase length to roughly account for word order.
        let reader = ctx.reader;
        let total = reader.max_doc().max(1) as f64;
        let terms = self.phrase_terms();
        let mut fraction = 1.0;
        let mut shortest = u64::MAX;
        for term in &terms {
            let count = reader.inverted_index(term.field())?.doc_freq(term)?;
            fraction *= count as f64 / total;
            shortest = shortest.min(count as u64);
        }
        let checks = (10 * terms.len()) as f64;
        Ok(Some(Estimate {
            matches: fraction * total / checks,
            cost: ((fraction * total * checks).ceil() as u64).max(shortest),
        }))
    }
}

struct CoalescedMustRange<'a> {
    field: &'a SearchField,
    field_path: Option<&'a str>,
    lower_bound: Bound<PdbOwnedValue>,
    upper_bound: Bound<PdbOwnedValue>,
}

impl QueryEstimate for BooleanQuery {
    fn is_fast_count(&self) -> bool {
        self.clauses()
            .iter()
            .all(|(_, query)| query.is_fast_count())
    }

    fn as_range_clause(&self) -> Option<RangeClause<'_>> {
        if let [(Occur::Must | Occur::Should, inner)] = self.clauses() {
            inner.as_range_clause()
        } else {
            None
        }
    }

    fn estimate_docs(&self, ctx: &EstimateContext<'_>) -> tantivy::Result<Option<Estimate>> {
        // Exact match queries become terms joined by AND or OR, so they use this estimate too.
        // AND starts with the smallest estimate; each extra condition reduces it less.
        // OR estimates how many documents match enough conditions.
        // NOT removes the estimated share of documents matching the excluded conditions.
        if let [(Occur::Must | Occur::Should, child)] = self.clauses() {
            return child.estimate_docs(ctx);
        }
        let total = ctx.reader.max_doc().max(1) as f64;
        let mut required = Vec::new();
        let mut coalesced_ranges: Vec<CoalescedMustRange<'_>> = Vec::new();
        let mut excluded = 1.0;
        let mut optional = Vec::new();
        let mut required_cost: Option<u64> = None;
        let mut optional_cost = 0u64;
        for (occur, child) in self.clauses() {
            if matches!(occur, Occur::Must)
                && let Some(range) = child.as_range_clause()
            {
                if let Some(existing) = coalesced_ranges.iter_mut().find(|entry| {
                    entry.field.field() == range.field.field()
                        && entry.field_path == range.field_path
                        && ranges_comparable(
                            RangeBoundsRef {
                                lower: &entry.lower_bound,
                                upper: &entry.upper_bound,
                            },
                            range.bounds,
                        )
                }) {
                    existing.lower_bound = tighter_lower(&existing.lower_bound, range.bounds.lower);
                    existing.upper_bound = tighter_upper(&existing.upper_bound, range.bounds.upper);
                } else {
                    coalesced_ranges.push(CoalescedMustRange {
                        field: range.field,
                        field_path: range.field_path,
                        lower_bound: range.bounds.lower.clone(),
                        upper_bound: range.bounds.upper.clone(),
                    });
                }
                continue;
            }
            let Some(child) = child.estimate_docs(ctx)? else {
                return Ok(None);
            };
            match occur {
                Occur::Must => {
                    required.push(child.matches);
                    required_cost =
                        Some(required_cost.map_or(child.cost, |cost| cost.min(child.cost)));
                }
                Occur::Should => {
                    optional.push(child.matches / total);
                    optional_cost = optional_cost.saturating_add(child.cost);
                }
                Occur::MustNot => excluded *= 1.0 - child.matches / total,
            }
        }
        for entry in &coalesced_ranges {
            let Some(child) = estimate_range_docs(
                entry.field,
                entry.field_path,
                RangeBoundsRef {
                    lower: &entry.lower_bound,
                    upper: &entry.upper_bound,
                },
                ctx,
            )?
            else {
                return Ok(None);
            };
            required.push(child.matches);
            required_cost = Some(required_cost.map_or(child.cost, |cost| cost.min(child.cost)));
        }
        required.sort_unstable_by(f64::total_cmp);
        let mut required = required.into_iter();
        let mut required_matches = required.next().unwrap_or(total);
        let mut exponent = 0.5;
        for matches in required {
            required_matches *= (matches / total).powf(exponent);
            exponent *= 0.5;
        }
        let minimum = self
            .get_minimum_number_should_match()
            .max(usize::from(required_cost.is_none()));
        let optional_fraction = if minimum == 0 {
            1.0
        } else if minimum > optional.len() {
            0.0
        } else {
            let mut probabilities = vec![0.0; minimum + 1];
            probabilities[0] = 1.0;
            for fraction in optional {
                probabilities[minimum] += probabilities[minimum - 1] * fraction;
                for n in (1..minimum).rev() {
                    probabilities[n] =
                        probabilities[n] * (1.0 - fraction) + probabilities[n - 1] * fraction;
                }
                probabilities[0] *= 1.0 - fraction;
            }
            probabilities[minimum]
        };
        Ok(Some(Estimate {
            matches: (required_matches * (optional_fraction * excluded)).clamp(0.0, total),
            cost: required_cost.unwrap_or(optional_cost),
        }))
    }
}

// Tokenization can turn an eligible match or phrase into an empty query.
impl QueryEstimate for EmptyQuery {
    fn estimate_docs(&self, _ctx: &EstimateContext<'_>) -> tantivy::Result<Option<Estimate>> {
        Ok(Some(Estimate {
            matches: 0.0,
            cost: 0,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::SearchFieldConfig;
    use proptest::prelude::*;
    use std::collections::BTreeMap;
    use std::sync::LazyLock;
    use tantivy::Directory;
    use tantivy::query::RegexQuery;
    use tantivy::schema::{FAST, Field, INDEXED, IndexRecordOption, Schema, TEXT};
    use tantivy::{Index, Searcher, TantivyDocument, Term, doc};

    struct TextFixture {
        field: Field,
        searcher: Searcher,
    }

    static TEXT_FIXTURE: LazyLock<TextFixture> = LazyLock::new(|| {
        let mut schema = Schema::builder();
        let field = schema.add_text_field("body", TEXT);
        let index = Index::create_in_ram(schema.build());
        let mut writer = index
            .writer_with_num_threads::<TantivyDocument>(1, 50_000_000)
            .unwrap();
        for _ in 0..16 {
            for value in ["alpha beta", "alpha delta", "gamma beta", "gamma delta"] {
                writer.add_document(doc!(field => value)).unwrap();
            }
        }
        writer.commit().unwrap();
        let searcher = index.reader().unwrap().searcher();
        TextFixture { field, searcher }
    });

    struct RangeFixture {
        id_search: SearchField,
        other_search: SearchField,
        searcher: Searcher,
    }

    static RANGE_FIXTURE: LazyLock<RangeFixture> = LazyLock::new(|| {
        let mut schema_builder = Schema::builder();
        let id_field = schema_builder.add_i64_field("id", INDEXED | FAST);
        let other_field = schema_builder.add_i64_field("other", INDEXED | FAST);
        let schema = schema_builder.build();
        let dir = tantivy::directory::RamDirectory::create();
        let meta = tantivy::IndexMeta {
            index_settings: tantivy::IndexSettings::default(),
            segments: Vec::new(),
            schema: schema.clone(),
            opstamp: 0,
            payload: None,
            persisted_custom_extensions: vec!["stats".to_string()],
        };
        dir.atomic_write(
            std::path::Path::new("meta.json"),
            &serde_json::to_vec(&meta).unwrap(),
        )
        .unwrap();
        let mut index = Index::open(dir).unwrap();
        crate::index::stats::register(&mut index);
        let mut writer = index
            .writer_with_num_threads::<TantivyDocument>(1, 50_000_000)
            .unwrap();
        for i in 1..=100i64 {
            writer
                .add_document(doc!(id_field => i, other_field => i))
                .unwrap();
        }
        writer.commit().unwrap();
        let searcher = index.reader().unwrap().searcher();
        let id_search = SearchField::for_testing(
            id_field,
            &schema,
            SearchFieldType::I64(pgrx::pg_sys::INT8OID),
            SearchFieldConfig::default_numeric(),
        );
        let other_search = SearchField::for_testing(
            other_field,
            &schema,
            SearchFieldType::I64(pgrx::pg_sys::INT8OID),
            SearchFieldConfig::default_numeric(),
        );
        RangeFixture {
            id_search,
            other_search,
            searcher,
        }
    });

    struct MapColumnStats(BTreeMap<Field, Arc<ColumnStats>>);

    impl ColumnStatsProvider for MapColumnStats {
        fn column_stats(
            &self,
            field: &SearchField,
            field_path: Option<&str>,
        ) -> Option<Arc<ColumnStats>> {
            if field_path.is_some() || !is_supported_stat_field(field) {
                return None;
            }
            self.0.get(&field.field()).cloned()
        }
    }

    fn make_bound(kind: u8, val: PdbOwnedValue) -> Bound<PdbOwnedValue> {
        match kind % 3 {
            0 => Bound::Unbounded,
            1 => Bound::Included(val),
            _ => Bound::Excluded(val),
        }
    }

    fn make_range_query(
        sf: &SearchField,
        path: Option<&str>,
        lo: Bound<i64>,
        hi: Bound<i64>,
        wrap: u8,
    ) -> Box<dyn Query> {
        let base: Box<dyn Query> = Box::new(RangeQueryWithEstimate::new(
            RangeQuery::new(
                lo.map(|v| Term::from_field_i64(sf.field(), v)),
                hi.map(|v| Term::from_field_i64(sf.field(), v)),
            ),
            sf.clone(),
            path.map(str::to_owned),
            (lo.map(PdbOwnedValue::I64), hi.map(PdbOwnedValue::I64)),
        ));
        match wrap % 4 {
            0 => base,
            1 => Box::new(ConstScoreQuery::new(base, 0.0)),
            2 => Box::new(BoostQuery::new(base, 2.0)),
            _ => Box::new(BooleanQuery::intersection(vec![base])),
        }
    }

    fn bound_i64_value(bound: &Bound<PdbOwnedValue>) -> Bound<i64> {
        bound.as_ref().map(|v| match v {
            PdbOwnedValue::I64(x) => *x,
            _ => unreachable!(),
        })
    }

    #[test]
    fn statistics_estimates_formulas_and_deletes() {
        let segment = TEXT_FIXTURE.searcher.segment_reader(0);
        let field = TEXT_FIXTURE.field;
        let term = |v| -> Box<dyn Query> {
            Box::new(TermQuery::new(
                Term::from_field_text(field, v),
                IndexRecordOption::Basic,
            ))
        };
        let estimate = |q: &dyn Query, seg: &SegmentReader| {
            q.estimate_docs(&EstimateContext::without_column_stats(seg))
                .unwrap()
                .map(|e| (e.live_docs(seg), e.cost))
        };

        for (query, expected) in [
            (term("alpha"), Some((32, 32))),
            (term("absent"), Some((0, 0))),
            (
                Box::new(PhraseQuery::new(vec![
                    Term::from_field_text(field, "alpha"),
                    Term::from_field_text(field, "beta"),
                ])),
                Some((1, 320)),
            ),
            (
                Box::new(SynonymQuery::new(vec![
                    Term::from_field_text(field, "alpha"),
                    Term::from_field_text(field, "beta"),
                ])),
                Some((48, 64)),
            ),
            (
                Box::new(BooleanQuery::intersection(vec![
                    term("alpha"),
                    term("beta"),
                ])),
                Some((23, 32)),
            ),
            (
                Box::new(BooleanQuery::intersection(vec![
                    term("alpha"),
                    term("beta"),
                    term("gamma"),
                ])),
                Some((20, 32)),
            ),
            (
                Box::new(BooleanQuery::intersection(vec![
                    term("alpha"),
                    Box::new(RegexQuery::from_pattern("b.*", field).unwrap()),
                ])),
                None,
            ),
            (
                Box::new(ConstScoreQuery::new(AllQuery, 0.0)),
                Some((64, 64)),
            ),
            (Box::new(EmptyQuery), Some((0, 0))),
        ] {
            assert_eq!(estimate(query.as_ref(), segment), expected);
        }

        // Deletion scaling: live_docs scales by `num_docs / max_doc`, while cost reflects raw postings.
        let mut schema = Schema::builder();
        let del_field = schema.add_text_field("body", TEXT);
        let index = Index::create_in_ram(schema.build());
        let mut writer = index
            .writer_with_num_threads::<TantivyDocument>(1, 50_000_000)
            .unwrap();
        for n in 0..100 {
            writer
                .add_document(doc!(del_field => if n < 7 { "rare" } else { "common" }))
                .unwrap();
        }
        writer.commit().unwrap();
        writer.delete_term(Term::from_field_text(del_field, "common"));
        writer.commit().unwrap();
        let searcher = index.reader().unwrap().searcher();
        let del_seg = searcher.segment_reader(0);
        assert_eq!(estimate(&AllQuery, del_seg), Some((7, 100)));

        // Outer-bucket clamping on post-ANALYZE append-only growth, heavy-tail max, and single outlier.
        let append_hist: Vec<PdbOwnedValue> = (0..=100)
            .map(|i| PdbOwnedValue::I64(if i == 0 { 1 } else { i * 2_000 }))
            .collect();
        let append_stats = ColumnStats {
            null_frac: 0.0,
            mcvs: Vec::new(),
            histogram: append_hist,
        };
        for (emp_min, emp_max, total_rows, lo, hi, expected) in [
            (
                1,
                220_000,
                220_000.0,
                Bound::Excluded(200_000),
                Bound::Unbounded,
                Some(2_000.0),
            ),
            (
                1,
                220_000,
                220_000.0,
                Bound::Excluded(199_000),
                Bound::Unbounded,
                Some(2_100.0),
            ),
            (
                1,
                1_357_603,
                200_000.0,
                Bound::Unbounded,
                Bound::Included(100_000),
                Some(100_000.0),
            ),
            (
                1,
                100_000_000,
                200_001.0,
                Bound::Unbounded,
                Bound::Included(100_000),
                Some(100_000.0),
            ),
            (
                200_001,
                220_000,
                20_000.0,
                Bound::Excluded(200_000),
                Bound::Unbounded,
                None,
            ),
        ] {
            let emp = EmpiricalStats {
                min: PdbOwnedValue::I64(emp_min),
                max: PdbOwnedValue::I64(emp_max),
                nullable: false,
            };
            let lo_b = lo.map(PdbOwnedValue::I64);
            let hi_b = hi.map(PdbOwnedValue::I64);
            let frac = estimate_fraction_from_column_stats(
                &append_stats,
                Some(&emp),
                RangeBoundsRef {
                    lower: &lo_b,
                    upper: &hi_b,
                },
            );
            match expected {
                Some(expected_rows) => {
                    assert!(((frac.unwrap() * total_rows) - expected_rows).abs() <= 15.0);
                }
                None => assert_eq!(frac, None),
            }
        }

        // Mostly-NULL column (`null_frac = 0.95`) on a segment covering a 1/8 slice of the range:
        // NULLs spread across segments rather than concentrating the whole table's `null_frac` in one slice.
        let nullable_stats = ColumnStats {
            null_frac: 0.95,
            mcvs: Vec::new(),
            histogram: append_stats.histogram.clone(),
        };
        let slice_emp = EmpiricalStats {
            min: PdbOwnedValue::I64(1),
            max: PdbOwnedValue::I64(25_000),
            nullable: true,
        };
        let slice_frac = estimate_fraction_from_column_stats(
            &nullable_stats,
            Some(&slice_emp),
            RangeBoundsRef {
                lower: &Bound::Excluded(PdbOwnedValue::I64(0)),
                upper: &Bound::Unbounded,
            },
        )
        .unwrap();
        assert!((slice_frac - 0.05).abs() <= 1e-9);
    }

    proptest! {
        #[test]
        fn boolean_query_estimate_properties(
            must in prop::collection::vec(0usize..5, 0..4),
            should in prop::collection::vec(0usize..5, 0..4),
            must_not in prop::collection::vec(0usize..5, 0..3),
            min_should in 0usize..=5,
        ) {
            const VOCAB: [&str; 5] = ["alpha", "beta", "gamma", "delta", "absent"];
            let segment = TEXT_FIXTURE.searcher.segment_reader(0);
            let ctx = EstimateContext::without_column_stats(segment);
            let field = TEXT_FIXTURE.field;
            let term = |idx: usize| -> Box<dyn Query> {
                Box::new(TermQuery::new(
                    Term::from_field_text(field, VOCAB[idx]),
                    IndexRecordOption::Basic,
                ))
            };
            let build = |m: &[usize], s: &[usize], mn: &[usize], msm: usize| {
                let mut clauses = Vec::new();
                clauses.extend(m.iter().map(|&i| (Occur::Must, term(i))));
                clauses.extend(s.iter().map(|&i| (Occur::Should, term(i))));
                clauses.extend(mn.iter().map(|&i| (Occur::MustNot, term(i))));
                BooleanQuery::with_minimum_required_clauses(clauses, msm)
            };

            let fwd = build(&must, &should, &must_not, min_should)
                .estimate_docs(&ctx)
                .unwrap()
                .unwrap();
            let rev_must: Vec<_> = must.iter().copied().rev().collect();
            let rev_should: Vec<_> = should.iter().copied().rev().collect();
            let rev_mn: Vec<_> = must_not.iter().copied().rev().collect();
            let rev = build(&rev_must, &rev_should, &rev_mn, min_should)
                .estimate_docs(&ctx)
                .unwrap()
                .unwrap();
            prop_assert!((fwd.matches - rev.matches).abs() <= 1e-9);
            prop_assert_eq!(fwd.cost, rev.cost);

            // When `min_should <= should.len()` and at least one `Must` clause is present (or
            // `should` is empty), adding `Must(AllQuery)` preserves match count, while
            // `Must(EmptyQuery)` annihilates.
            if min_should <= should.len() && (!must.is_empty() || should.is_empty()) {
                let mut with_all = build(&must, &should, &must_not, min_should)
                    .clauses()
                    .to_vec();
                with_all.push((Occur::Must, Box::new(AllQuery)));
                let est_all = BooleanQuery::with_minimum_required_clauses(with_all, min_should)
                    .estimate_docs(&ctx)
                    .unwrap()
                    .unwrap();
                if !(must.is_empty() && should.is_empty()) {
                    prop_assert!((est_all.matches - fwd.matches).abs() <= 1e-9);
                }
            }
            let mut with_empty = build(&must, &should, &must_not, min_should)
                .clauses()
                .to_vec();
            with_empty.push((Occur::Must, Box::new(EmptyQuery)));
            let est_empty = BooleanQuery::with_minimum_required_clauses(with_empty, min_should)
                .estimate_docs(&ctx)
                .unwrap()
                .unwrap();
            prop_assert_eq!((est_empty.live_docs(segment), est_empty.cost), (0, 0));

            // For 2+ `Should` clauses, `minimum_should_match` is non-increasing (`0 == 1`,
            // `> len == 0`); a 1-clause `BooleanQuery` delegates directly to its child.
            if should.len() >= 2 {
                let counts: Vec<u64> = (0..=should.len() + 1)
                    .map(|m| {
                        BooleanQuery::union_with_minimum_required_clauses(
                            should.iter().map(|&i| term(i)).collect(),
                            m,
                        )
                        .estimate_docs(&ctx)
                        .unwrap()
                        .unwrap()
                        .live_docs(segment)
                    })
                    .collect();
                prop_assert_eq!(counts[0], counts[1]);
                prop_assert_eq!(*counts.last().unwrap(), 0);
                for pair in counts.windows(2) {
                    prop_assert!(pair[0] >= pair[1]);
                }
            } else if should.len() == 1 {
                let single = BooleanQuery::with_minimum_required_clauses(
                    vec![(Occur::Should, term(should[0]))],
                    2,
                )
                .estimate_docs(&ctx)
                .unwrap()
                .unwrap();
                let child = term(should[0]).estimate_docs(&ctx).unwrap().unwrap();
                prop_assert_eq!((single.live_docs(segment), single.cost), (child.live_docs(segment), child.cost));
            }
        }

        #[test]
        fn column_stats_and_segment_conditioning_properties(
            is_float in any::<bool>(),
            w_null in 0u32..=20,
            mcv_raw in prop::collection::vec((-40i64..=40, 1u32..=10), 0..=4),
            w_hist in 0u32..=20,
            f32_jitter in any::<bool>(),
            mut hist_raw in prop::collection::vec(-40i64..=40, 2..=6),
            lo_kind in 0u8..3,
            lo_raw in -50i64..=50,
            hi_kind in 0u8..3,
            hi_raw in -50i64..=50,
            mid_raw in -50i64..=50,
            seg_a in -50i64..=50,
            seg_b in -50i64..=50,
            seg_nullable in any::<bool>(),
        ) {
            let val = |x: i64| {
                if is_float {
                    PdbOwnedValue::F64(x as f64 * 0.5)
                } else {
                    PdbOwnedValue::I64(x)
                }
            };
            hist_raw.sort_unstable();
            let mut mcv_map = BTreeMap::new();
            for (k, w) in mcv_raw {
                *mcv_map.entry(k).or_insert(0u32) += w;
            }
            let w_mcv: u32 = mcv_map.values().sum();
            let w_total = w_null + w_mcv + w_hist;

            let use_jitter = f32_jitter && w_hist == 0 && !mcv_map.is_empty() && w_total > 0;
            let stats = if w_total == 0 {
                ColumnStats {
                    null_frac: 1.0,
                    mcvs: Vec::new(),
                    histogram: Vec::new(),
                }
            } else {
                let scale = if use_jitter { 1.0 - 5e-5 } else { 1.0 };
                ColumnStats {
                    null_frac: w_null as f64 / w_total as f64,
                    mcvs: mcv_map
                        .iter()
                        .map(|(&k, &w)| (val(k), (w as f64 / w_total as f64) * scale))
                        .collect(),
                    histogram: if w_hist == 0 {
                        Vec::new()
                    } else {
                        hist_raw.iter().map(|&x| val(x)).collect()
                    },
                }
            };

            let lower = make_bound(lo_kind, val(lo_raw));
            let upper = make_bound(hi_kind, val(hi_raw));
            let bounds = RangeBoundsRef {
                lower: &lower,
                upper: &upper,
            };
            let p = stats.estimate_range_selectivity(bounds).unwrap();
            prop_assert!((0.0..=1.0 - stats.null_frac + 1e-6).contains(&p));
            if ends_before(upper.as_ref(), lower.as_ref()) {
                prop_assert_eq!(p, 0.0);
            }
            let all_p = stats
                .estimate_range_selectivity(RangeBoundsRef {
                    lower: &Bound::Unbounded,
                    upper: &Bound::Unbounded,
                })
                .unwrap();
            prop_assert!((all_p - (1.0 - stats.null_frac)).abs() <= 1e-9);

            // Finite additivity across an arbitrary cut point `mid` for both `< mid` / `>= mid`
            // and `<= mid` / `> mid` splits.
            let mid = val(mid_raw);
            let tol = if use_jitter { 1e-4 } else { 1e-9 };
            for (left_hi, right_lo) in [
                (Bound::Excluded(mid.clone()), Bound::Included(mid.clone())),
                (Bound::Included(mid.clone()), Bound::Excluded(mid)),
            ] {
                let r_left_hi = tighter_upper(&upper, &left_hi);
                let r_right_lo = tighter_lower(&lower, &right_lo);
                let p_left = stats
                    .estimate_range_selectivity(RangeBoundsRef {
                        lower: &lower,
                        upper: &r_left_hi,
                    })
                    .unwrap();
                let p_right = stats
                    .estimate_range_selectivity(RangeBoundsRef {
                        lower: &r_right_lo,
                        upper: &upper,
                    })
                    .unwrap();
                prop_assert!(
                    (p_left + p_right - p).abs() <= tol,
                    "additivity failed: p_left={p_left}, p_right={p_right}, p={p}"
                );
            }

            // On a discrete uniform 1-bucket range `[lo_s, hi_s]`, `ColumnStats` and
            // `estimate_uniform_from_empirical` both equal exact integer counting.
            let (s_lo, s_hi) = (seg_a.min(seg_b), seg_a.max(seg_b));
            if !is_float {
                let exact_matches = (s_lo..=s_hi)
                    .filter(|&x| {
                        bounds
                            .contains(&PdbOwnedValue::I64(x))
                            .unwrap()
                    })
                    .count() as f64;
                let exact_frac = exact_matches / (s_hi - s_lo + 1) as f64;
                let uniform_stats = ColumnStats {
                    null_frac: 0.0,
                    mcvs: Vec::new(),
                    histogram: vec![PdbOwnedValue::I64(s_lo), PdbOwnedValue::I64(s_hi)],
                };
                let emp_non_null = EmpiricalStats {
                    min: PdbOwnedValue::I64(s_lo),
                    max: PdbOwnedValue::I64(s_hi),
                    nullable: false,
                };
                prop_assert!(
                    (uniform_stats.estimate_range_selectivity(bounds).unwrap() - exact_frac).abs()
                        <= 1e-9
                );
                prop_assert!(
                    (estimate_uniform_from_empirical(&emp_non_null, bounds).unwrap() - exact_frac)
                        .abs()
                        <= 1e-9
                );
            }

            // Segment conditioning invariants (`estimate_fraction_from_column_stats`).
            let empirical = EmpiricalStats {
                min: val(s_lo),
                max: val(s_hi),
                nullable: seg_nullable,
            };
            let segment_lower = Bound::Included(empirical.min.clone());
            let segment_upper = Bound::Included(empirical.max.clone());
            let conditioned =
                estimate_fraction_from_column_stats(&stats, Some(&empirical), bounds);
            if stats.null_frac >= 1.0 - 1e-9 || stats.value_domain().is_none() {
                prop_assert_eq!(conditioned, None);
            } else if let Some(conditioned_prob) = conditioned {
                prop_assert!((0.0..=1.0).contains(&conditioned_prob));
                let full_segment = estimate_fraction_from_column_stats(
                    &stats,
                    Some(&empirical),
                    RangeBoundsRef {
                        lower: &segment_lower,
                        upper: &segment_upper,
                    },
                )
                .unwrap();
                let expected_full = if seg_nullable {
                    1.0 - stats.null_frac
                } else {
                    1.0
                };
                prop_assert!((full_segment - expected_full).abs() <= 1e-9);
                let (stat_min, stat_max) = stats.value_domain().unwrap();
                if stats.histogram.len() >= 2 && (!is_float || s_lo < s_hi) {
                    if empirical.max.total_cmp(stat_max) == Ordering::Greater {
                        let above = estimate_fraction_from_column_stats(
                            &stats,
                            Some(&empirical),
                            RangeBoundsRef {
                                lower: &Bound::Excluded(stat_max.clone()),
                                upper: &Bound::Unbounded,
                            },
                        )
                        .unwrap();
                        prop_assert!(above > 0.0);
                    }
                    if empirical.min.total_cmp(stat_min) == Ordering::Less {
                        let below = estimate_fraction_from_column_stats(
                            &stats,
                            Some(&empirical),
                            RangeBoundsRef {
                                lower: &Bound::Unbounded,
                                upper: &Bound::Excluded(stat_min.clone()),
                            },
                        )
                        .unwrap();
                        prop_assert!(below > 0.0);
                    }
                }
                // Outlier invariance: extending `empirical.max` above `stat_max` leaves interior
                // bins (`< histogram[nbuckets - 1]`) unchanged and shifts `<= stat_max` by at most
                // `1 / nbuckets`.
                if stats.histogram.len() >= 3 {
                    let nbuckets = stats.histogram.len() - 1;
                    let base_emp = EmpiricalStats {
                        min: stat_min.clone(),
                        max: stat_max.clone(),
                        nullable: seg_nullable,
                    };
                    let outlier_emp = EmpiricalStats {
                        min: stat_min.clone(),
                        max: val(100_000),
                        nullable: seg_nullable,
                    };
                    for (cap, tol) in [
                        (Bound::Excluded(stats.histogram[nbuckets - 1].clone()), 1e-9),
                        (Bound::Included(stat_max.clone()), 1.0 / nbuckets as f64 + 1e-6),
                    ] {
                        let capped_hi = tighter_upper(&upper, &cap);
                        let capped = RangeBoundsRef {
                            lower: &lower,
                            upper: &capped_hi,
                        };
                        let b = estimate_fraction_from_column_stats(&stats, Some(&base_emp), capped)
                            .unwrap();
                        let o = estimate_fraction_from_column_stats(
                            &stats,
                            Some(&outlier_emp),
                            capped,
                        )
                        .unwrap();
                        prop_assert!((o - b).abs() <= tol);
                    }
                }
                let mid_cut = val(mid_raw);
                let left_upper = tighter_upper(&upper, &Bound::Excluded(mid_cut.clone()));
                let right_lower = tighter_lower(&lower, &Bound::Included(mid_cut));
                let conditioned_left = estimate_fraction_from_column_stats(
                    &stats,
                    Some(&empirical),
                    RangeBoundsRef {
                        lower: &lower,
                        upper: &left_upper,
                    },
                )
                .unwrap();
                let conditioned_right = estimate_fraction_from_column_stats(
                    &stats,
                    Some(&empirical),
                    RangeBoundsRef {
                        lower: &right_lower,
                        upper: &upper,
                    },
                )
                .unwrap();
                prop_assert!((conditioned_left + conditioned_right - conditioned_prob).abs() <= 1e-6);
            } else {
                let in_stats_seg = stats
                    .estimate_range_selectivity(RangeBoundsRef {
                        lower: &segment_lower,
                        upper: &segment_upper,
                    })
                    .unwrap();
                prop_assert_eq!(in_stats_seg, 0.0);
            }
        }

        #[test]
        fn boolean_must_range_coalescing_properties(
            use_col_stats in any::<bool>(),
            lo1_kind in 0u8..3,
            lo1_val in -10i64..=110,
            hi1_kind in 0u8..3,
            hi1_val in -10i64..=110,
            lo2_kind in 0u8..3,
            lo2_val in -10i64..=110,
            hi2_kind in 0u8..3,
            hi2_val in -10i64..=110,
            wrap1 in 0u8..4,
            wrap2 in 0u8..4,
            mode in 0u8..3,
        ) {
            let to_i64_bound = |kind: u8, v: i64| match kind % 3 {
                0 => Bound::Unbounded,
                1 => Bound::Included(v),
                _ => Bound::Excluded(v),
            };
            let (lo1, hi1) = (to_i64_bound(lo1_kind, lo1_val), to_i64_bound(hi1_kind, hi1_val));
            let (lo2, hi2) = (to_i64_bound(lo2_kind, lo2_val), to_i64_bound(hi2_kind, hi2_val));
            let segment = RANGE_FIXTURE.searcher.segment_reader(0);
            let mut map = BTreeMap::new();
            if use_col_stats {
                // Stale `[1, 80]` histogram over a `1..=100` segment exercises `ColumnStatsProvider`
                // and outer-bucket clamping end-to-end through `QueryEstimate::estimate_docs`.
                map.insert(
                    RANGE_FIXTURE.id_search.field(),
                    Arc::new(ColumnStats {
                        null_frac: 0.0,
                        mcvs: Vec::new(),
                        histogram: vec![
                            PdbOwnedValue::I64(1),
                            PdbOwnedValue::I64(20),
                            PdbOwnedValue::I64(80),
                        ],
                    }),
                );
            }
            let provider = MapColumnStats(map);
            let ctx = EstimateContext::new(segment, &provider);

            let (sf2, path1, path2) = match mode % 3 {
                0 => (&RANGE_FIXTURE.id_search, None, None),
                1 => (&RANGE_FIXTURE.other_search, None, None),
                _ => (&RANGE_FIXTURE.id_search, Some("a"), Some("b")),
            };
            let q1 = make_range_query(&RANGE_FIXTURE.id_search, path1, lo1, hi1, wrap1);
            let q2 = make_range_query(sf2, path2, lo2, hi2, wrap2);
            let combined = BooleanQuery::intersection(vec![q1, q2])
                .estimate_docs(&ctx)
                .unwrap()
                .unwrap();

            if mode % 3 == 0 {
                let pdb_lo = tighter_lower(&lo1.map(PdbOwnedValue::I64), &lo2.map(PdbOwnedValue::I64));
                let pdb_hi = tighter_upper(&hi1.map(PdbOwnedValue::I64), &hi2.map(PdbOwnedValue::I64));
                let single = make_range_query(
                    &RANGE_FIXTURE.id_search,
                    None,
                    bound_i64_value(&pdb_lo),
                    bound_i64_value(&pdb_hi),
                    0,
                )
                .estimate_docs(&ctx)
                .unwrap()
                .unwrap();
                prop_assert_eq!(
                    (combined.live_docs(segment), combined.cost),
                    (single.live_docs(segment), single.cost)
                );
                let b_ref = RangeBoundsRef {
                    lower: &pdb_lo,
                    upper: &pdb_hi,
                };
                let exact = (1..=100i64)
                    .filter(|&x| b_ref.contains(&PdbOwnedValue::I64(x)).unwrap())
                    .count() as u64;
                if exact == 0 {
                    prop_assert_eq!((combined.live_docs(segment), combined.cost), (0, 0));
                } else {
                    prop_assert!(combined.live_docs(segment) >= 1);
                    if !use_col_stats {
                        prop_assert!(combined.live_docs(segment).abs_diff(exact) <= 1);
                    }
                    prop_assert_eq!(combined.cost, 100);
                }
            } else {
                let e1 = make_range_query(&RANGE_FIXTURE.id_search, path1, lo1, hi1, 0)
                    .estimate_docs(&ctx)
                    .unwrap()
                    .unwrap();
                let e2 = make_range_query(sf2, path2, lo2, hi2, 0)
                    .estimate_docs(&ctx)
                    .unwrap()
                    .unwrap();
                let (m_min, m_max) = (e1.matches.min(e2.matches), e1.matches.max(e2.matches));
                let expected = ((m_min * (m_max / 100.0).sqrt()).ceil() as u64).min(100);
                prop_assert_eq!(combined.live_docs(segment), expected);
            }
        }
    }
}
