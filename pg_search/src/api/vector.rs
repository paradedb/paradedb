// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use crate::index::directory::utils::{load_index_settings, replace_settings};
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::catalog::is_pgvector_oid;
use crate::postgres::is_bm25_index;
use crate::postgres::rel::PgSearchRelation;
use crate::vector::PgVector;
use anyhow::{Context, Result, bail, ensure};
use pgrx::prelude::*;
use pgrx::{AnyArray, PgRelation, Spi, pg_sys};
use tantivy::schema::FieldType;
use tantivy::vector::{
    VectorCalibrationMeasurements, VectorQuantizationCalibrationSource,
    VectorQuantizationDepthCalibration,
};

const MAX_CALIBRATION_QUERIES: usize = 256;
const CALIBRATION_SAMPLE_ROWS: usize = 1_000;

type CalibrationRow = (
    name!(depth, i32),
    name!(bias, f32),
    name!(spread, f32),
    name!(sample_count, i32),
    name!(source, String),
);

/// Calibrate one quantized vector field with caller-supplied production
/// queries, then atomically replace the index-level settings record.
///
/// External queries are independent of the indexed corpus, so row sampling
/// deliberately performs no cluster-membership exclusion. Every supplied
/// query is validated; the first 256 are measured to bound calibration work.
///
/// `AnyArray` is deliberate at the Rust boundary: pgrx has no native pgvector
/// array type. The SQL declaration below fixes the public type to `vector[]`,
/// and the element OID is checked before the sole unsafe datum conversion.
#[pg_extern(
    name = "vector_calibrate",
    sql = r#"
CREATE FUNCTION paradedb.vector_calibrate(
    index regclass,
    field text,
    queries vector[]
) RETURNS TABLE(
    depth integer,
    bias real,
    spread real,
    sample_count integer,
    source text
)
VOLATILE PARALLEL UNSAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'vector_calibrate_internal_wrapper';
"#
)]
fn vector_calibrate_internal(
    index: Option<PgRelation>,
    field: Option<String>,
    queries: Option<AnyArray>,
) -> Result<
    TableIterator<
        'static,
        (
            name!(depth, i32),
            name!(bias, f32),
            name!(spread, f32),
            name!(sample_count, i32),
            name!(source, String),
        ),
    >,
> {
    let index = index.context("index must not be NULL")?;
    let field = field.context("field must not be NULL")?;
    let queries = queries.context("queries must not be NULL")?;
    let index_oid = index.oid();
    reject_partitioned_index(index_oid)?;
    drop(index);

    // The settings list and its metapage pointer are one physical-index
    // resource. Hold the strongest relation lock for the complete
    // read-measure-replace operation so concurrent calibrators cannot lose an
    // update and a concurrent merge cannot observe two settings generations.
    let index = PgSearchRelation::with_lock(index_oid, pg_sys::AccessExclusiveLock as _);
    ensure!(
        unsafe { pg_sys::get_rel_relkind(index.oid()) as u8 } == pg_sys::RELKIND_INDEX,
        "vector calibration requires a physical index"
    );
    ensure!(
        is_bm25_index(&index),
        "vector calibration requires a ParadeDB index"
    );
    ensure!(index.is_usable(), "index is not valid, ready, and live");

    let mut settings = load_index_settings(&index)?
        .with_context(|| format!("index {:?} has no persisted settings", index.name()))?;
    let quantization_idx = settings
        .vector_quantization
        .iter()
        .position(|config| config.field == field)
        .with_context(|| format!("field {field:?} is not configured for quantization"))?;
    let expected_dim = settings.vector_quantization[quantization_idx].dim;
    let queries = decode_queries(&queries, expected_dim)?;

    let search_reader = SearchIndexReader::empty(&index, MvccSatisfies::Snapshot)?;
    let vector_field = search_reader
        .schema()
        .tantivy_schema()
        .get_field(&field)
        .with_context(|| format!("field {field:?} is absent from the index schema"))?;
    let field_entry = search_reader
        .schema()
        .tantivy_schema()
        .get_field_entry(vector_field);
    let FieldType::Vector(vector_options) = field_entry.field_type() else {
        bail!("field {field:?} is not a vector field");
    };
    ensure!(
        vector_options.dim() == expected_dim,
        "quantization dimension {expected_dim} for field {field:?} does not match schema dimension {}",
        vector_options.dim()
    );

    let membership_rows = search_reader
        .segment_readers()
        .iter()
        .map(|segment_reader| -> Result<u64> {
            let vector_index = segment_reader.vector_index(vector_field)?;
            u64::try_from(vector_index.live_posting_row_count(segment_reader.alive_bitset()))
                .context("live posting-membership row count exceeds u64")
        })
        .collect::<Result<Vec<_>>>()?;
    let sample_rows = allocate_sample_rows(&membership_rows, CALIBRATION_SAMPLE_ROWS);
    ensure!(
        sample_rows.iter().sum::<usize>() != 0,
        "field {field:?} has no visible IVF posting-membership rows to calibrate"
    );

    let mut measurements: Option<VectorCalibrationMeasurements> = None;
    for ((segment_reader, &segment_memberships), &segment_sample_rows) in search_reader
        .segment_readers()
        .iter()
        .zip(&membership_rows)
        .zip(&sample_rows)
    {
        if segment_sample_rows == 0 {
            continue;
        }
        let vector_index = segment_reader.vector_index(vector_field)?;
        let segment_measurements = vector_index
            .calibrate_external_queries(
                &queries,
                segment_sample_rows,
                segment_reader.alive_bitset(),
            )?
            .with_context(|| {
                format!(
                    "visible segment {} has {segment_memberships} IVF posting-membership rows but no quantized storage for field {field:?}",
                    segment_reader.segment_id().short_uuid_string()
                )
            })?;
        if let Some(aggregate) = &mut measurements {
            aggregate.merge(&segment_measurements)?;
        } else {
            measurements = Some(segment_measurements);
        }
    }

    let measurements =
        measurements.context("no visible quantized segment supplied calibration measurements")?;
    log_calibration_stability(index.name(), &field, &measurements);
    let calibration = measurements.finish(VectorQuantizationCalibrationSource::RealQuery)?;
    let rows = calibration
        .iter()
        .enumerate()
        .map(|(depth, value)| calibration_row(depth, value))
        .collect::<Result<Vec<_>>>()?;
    settings.vector_quantization[quantization_idx]
        .install_real_query_calibration(calibration.clone())?;
    replace_settings(&index, &settings)?;

    Ok(TableIterator::new(rows))
}

fn reject_partitioned_index(index_oid: pg_sys::Oid) -> Result<()> {
    let relkind = unsafe { pg_sys::get_rel_relkind(index_oid) as u8 };
    if relkind != pg_sys::RELKIND_PARTITIONED_INDEX {
        return Ok(());
    }

    let children = Spi::get_one_with_args::<Vec<String>>(
        "SELECT ARRAY_AGG(c.oid::regclass::text ORDER BY c.oid::regclass::text) \
         FROM pg_inherits i \
         JOIN pg_class c ON c.oid = i.inhrelid \
         WHERE i.inhparent = $1",
        &[index_oid.into()],
    )?
    .unwrap_or_default();
    let children = if children.is_empty() {
        "<none>".to_string()
    } else {
        children.join(", ")
    };
    pgrx::pg_sys::panic::ErrorReport::new(
        pgrx::PgSqlErrorCode::ERRCODE_WRONG_OBJECT_TYPE,
        format!(
            "cannot calibrate a partitioned index parent; child indexes: {children}; calibrate each child index individually with paradedb.vector_calibrate"
        ),
        pgrx::function_name!(),
    )
    .set_hint("call paradedb.vector_calibrate for each child index individually")
    .report(pgrx::PgLogLevel::ERROR);
    Ok(())
}

fn decode_queries(queries: &AnyArray, expected_dim: usize) -> Result<Vec<Vec<f32>>> {
    let element_oid = unsafe { pg_sys::get_element_type(queries.oid()) };
    ensure!(
        is_pgvector_oid(element_oid),
        "queries must have type vector[]"
    );

    // Borrowed iteration is required here. pgrx 0.19's consuming
    // `AnyArrayIterator` asserts that its data pointer is in bounds before it
    // checks a NULL bitmap bit, which panics for an all-NULL varlena array.
    // `Array::iter` checks nullness first and therefore lets us surface the
    // promised SQL validation error without reading the element bytes.
    let array = pgrx::AnyArray::into::<pgrx::Array<pg_sys::Datum>>(queries)
        .context("could not decode queries vector[]")?;
    let mut decoded = Vec::new();
    let mut input_count = 0usize;
    for (query_idx, datum) in array.iter().enumerate() {
        let datum = datum
            .with_context(|| format!("quantization calibration query {} is NULL", query_idx + 1))?;
        // SAFETY: the array element OID was checked against pgvector's OID
        // above, and `Array::iter` established that this element is non-NULL.
        // `PgVector` detoasts and copies the datum before returning.
        let query = unsafe { PgVector::from_polymorphic_datum(datum, false, element_oid) }
            .with_context(|| {
                format!(
                    "could not decode quantization calibration query {}",
                    query_idx + 1
                )
            })?;
        ensure!(
            query.0.len() == expected_dim,
            "quantization calibration query {} has dimension {}; expected {expected_dim}",
            query_idx + 1,
            query.0.len()
        );
        input_count += 1;
        if decoded.len() < MAX_CALIBRATION_QUERIES {
            decoded.push(query.0);
        }
    }
    ensure!(input_count != 0, "queries must not be empty");
    Ok(decoded)
}

/// Allocate an index-wide row budget proportionally to each visible segment's
/// posting-membership count. Largest-remainder rounding is deterministic:
/// equal remainders are awarded in segment-reader order.
fn allocate_sample_rows(membership_rows: &[u64], requested: usize) -> Vec<usize> {
    let total = membership_rows
        .iter()
        .copied()
        .map(u128::from)
        .sum::<u128>();
    if total == 0 || requested == 0 {
        return vec![0; membership_rows.len()];
    }
    let target = total.min(requested as u128) as usize;
    let mut allocations = Vec::with_capacity(membership_rows.len());
    let mut remainders = Vec::with_capacity(membership_rows.len());
    let mut allocated = 0usize;
    for (segment, &rows) in membership_rows.iter().enumerate() {
        let numerator = u128::from(rows) * target as u128;
        let floor = (numerator / total) as usize;
        allocations.push(floor);
        remainders.push((numerator % total, segment));
        allocated += floor;
    }

    remainders.sort_unstable_by(
        |(left_remainder, left_segment), (right_remainder, right_segment)| {
            right_remainder
                .cmp(left_remainder)
                .then_with(|| left_segment.cmp(right_segment))
        },
    );
    for &(_, segment) in remainders.iter().take(target - allocated) {
        allocations[segment] += 1;
    }
    allocations
}

fn calibration_row(
    depth: usize,
    calibration: &VectorQuantizationDepthCalibration,
) -> Result<CalibrationRow> {
    let source = match calibration.source {
        VectorQuantizationCalibrationSource::HeldOut => "held_out",
        VectorQuantizationCalibrationSource::RealQuery => "real_query",
    };
    Ok((
        i32::try_from(depth + 1).context("quantization depth exceeds SQL integer range")?,
        calibration.bias,
        calibration.spread,
        i32::try_from(calibration.sample_count)
            .context("quantization calibration sample count exceeds SQL integer range")?,
        source.to_string(),
    ))
}

fn log_calibration_stability(
    index_name: &str,
    field: &str,
    measurements: &VectorCalibrationMeasurements,
) {
    let depth_count = measurements.aggregate().len();
    for depth in 0..depth_count {
        let (biases, spreads): (Vec<_>, Vec<_>) = measurements
            .per_query()
            .iter()
            .filter_map(|query| {
                let moments = query.get(depth)?;
                Some((moments.bias()?, moments.spread()?))
            })
            .unzip();
        let Some(bias) = summarize(&biases) else {
            continue;
        };
        let spread = summarize(&spreads).expect("bias and spread samples have the same shape");
        log::info!(
            target: crate::postgres::build_logging::QUANTIZATION_CALIBRATION_TARGET,
            "quantization_calibration_stability index={index_name:?} field={field:?} depth={} query_count={} bias_mean={} bias_stddev={} bias_min={} bias_max={} spread_mean={} spread_stddev={} spread_min={} spread_max={}",
            depth + 1,
            biases.len(),
            bias.mean,
            bias.stddev,
            bias.min,
            bias.max,
            spread.mean,
            spread.stddev,
            spread.min,
            spread.max,
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Summary {
    mean: f64,
    stddev: f64,
    min: f64,
    max: f64,
}

fn summarize(values: &[f64]) -> Option<Summary> {
    let (&first, rest) = values.split_first()?;
    let mut sum = first;
    let mut min = first;
    let mut max = first;
    for &value in rest {
        sum += value;
        min = min.min(value);
        max = max.max(value);
    }
    let mean = sum / values.len() as f64;
    let variance = values
        .iter()
        .map(|value| {
            let centered = value - mean;
            centered * centered
        })
        .sum::<f64>()
        / values.len() as f64;
    Some(Summary {
        mean,
        stddev: variance.max(0.0).sqrt(),
        min,
        max,
    })
}

#[cfg(test)]
mod tests {
    use super::{Summary, allocate_sample_rows, summarize};

    #[test]
    fn sample_row_allocation_handles_empty_inputs() {
        assert_eq!(allocate_sample_rows(&[], 1_000), Vec::<usize>::new());
        assert_eq!(allocate_sample_rows(&[0, 0], 1_000), vec![0, 0]);
        assert_eq!(allocate_sample_rows(&[10, 20], 0), vec![0, 0]);
    }

    #[test]
    fn sample_row_allocation_uses_every_row_below_the_cap() {
        assert_eq!(allocate_sample_rows(&[2, 0, 3], 1_000), vec![2, 0, 3]);
    }

    #[test]
    fn sample_row_allocation_breaks_equal_remainders_by_segment_order() {
        assert_eq!(
            allocate_sample_rows(&[1_000, 1_000, 1_000], 1_000),
            vec![334, 333, 333]
        );
    }

    #[test]
    fn sample_row_allocation_is_proportional_and_exact() {
        let allocations = allocate_sample_rows(&[1, 2, 7, 0], 6);
        assert_eq!(allocations, vec![1, 1, 4, 0]);
        assert_eq!(allocations.iter().sum::<usize>(), 6);
    }

    #[test]
    fn stability_summary_reports_population_shape() {
        assert_eq!(summarize(&[]), None);
        assert_eq!(
            summarize(&[1.0, 2.0, 3.0]),
            Some(Summary {
                mean: 2.0,
                stddev: (2.0f64 / 3.0).sqrt(),
                min: 1.0,
                max: 3.0,
            })
        );
    }
}
