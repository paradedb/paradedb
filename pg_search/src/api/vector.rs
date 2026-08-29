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
    VectorAuditMoments, VectorCalibrationMeasurements, VectorGammaAuditMeasurements,
    VectorGammaAuditQuery, VectorGammaConeAuditMeasurements, VectorQuantizationCalibrationSource,
    VectorQuantizationDepthCalibration,
};

const MAX_CALIBRATION_QUERIES: usize = 256;
const CALIBRATION_SAMPLE_ROWS: usize = 1_000;
const GAMMA_AUDIT_QUERY_COUNT: usize = 100;
const GAMMA_AUDIT_SAMPLE_ROWS: usize = 1_000;
const REAL_QUERY_GAMMA_PROTOCOL: &str = "Q1_REAL_CROSS_PRODUCT_BQ4";
const HELD_OUT_GAMMA_PROTOCOL: &str = "H1_HELD_OUT_CROSS_PRODUCT_BQ4";
const GAMMA_CONE_PROTOCOL: &str = "C1_ALL_CLUSTERS_GAMMA_CONE_K10_K2_K4";

type CalibrationRow = (
    name!(depth, i32),
    name!(bias, f32),
    name!(spread, f32),
    name!(sample_count, i32),
    name!(source, String),
);

type GammaAuditRow = (
    name!(source, String),
    name!(protocol, String),
    name!(depth, i32),
    name!(bias, f32),
    name!(spread, f32),
    name!(sample_count, i32),
    name!(gamma_sample_count, i64),
    name!(gamma_mean, f64),
    name!(gamma_spread, f64),
    name!(gamma_min, f64),
    name!(gamma_p50, f64),
    name!(gamma_p95, f64),
    name!(gamma_p99, f64),
    name!(gamma_max, f64),
    name!(f16_band_error_sample_count, i64),
    name!(f16_band_error_mean, f64),
    name!(f16_band_error_spread, f64),
    name!(f16_band_error_abs_p99, f64),
    name!(f16_band_error_abs_max, f64),
    name!(clamp_band_error_sample_count, i64),
    name!(clamp_band_error_mean, f64),
    name!(clamp_band_error_spread, f64),
    name!(clamp_band_error_abs_p99, f64),
    name!(clamp_band_error_abs_max, f64),
    name!(zero_count, i64),
    name!(clamp_count, i64),
    name!(orthogonality_sample_count, i64),
    name!(orthogonality_mean, f64),
    name!(orthogonality_spread, f64),
    name!(orthogonality_abs_p99, f64),
    name!(orthogonality_abs_max, f64),
);

type GammaConeAuditRow = (
    name!(protocol, String),
    name!(depth, i32),
    name!(kappa, f32),
    name!(query_count, i32),
    name!(top_k, i32),
    name!(mean_scored_rows, f64),
    name!(mean_survivor_rows, f64),
    name!(mean_survivor_docs, f64),
    name!(mean_survivor_fraction, f64),
    name!(mean_candidate_recall, f64),
    name!(min_candidate_recall, f64),
    name!(queries_with_miss, i32),
    name!(final_depth, bool),
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
    reject_partitioned_index(index_oid, PartitionedIndexOperation::Calibration)?;
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

/// Measure gamma-corrected estimator behavior without changing index settings.
///
/// The real-query protocol validates every caller-supplied query and measures
/// exactly the first 100. The held-out protocol independently samples exactly
/// 100 distinct visible stored documents across the index. Only a stored
/// pseudo-query's origin segment receives its `doc_id`, so all replica
/// memberships of that document are excluded there; external queries have no
/// residency and therefore no exclusion. Both protocols use the same
/// index-wide 1,000-posting-row target allocation.
#[pg_extern(
    name = "vector_gamma_audit",
    sql = r#"
CREATE FUNCTION paradedb.vector_gamma_audit(
    index regclass,
    field text,
    queries vector[]
) RETURNS TABLE(
    source text,
    protocol text,
    depth integer,
    bias real,
    spread real,
    sample_count integer,
    gamma_sample_count bigint,
    gamma_mean double precision,
    gamma_spread double precision,
    gamma_min double precision,
    gamma_p50 double precision,
    gamma_p95 double precision,
    gamma_p99 double precision,
    gamma_max double precision,
    f16_band_error_sample_count bigint,
    f16_band_error_mean double precision,
    f16_band_error_spread double precision,
    f16_band_error_abs_p99 double precision,
    f16_band_error_abs_max double precision,
    clamp_band_error_sample_count bigint,
    clamp_band_error_mean double precision,
    clamp_band_error_spread double precision,
    clamp_band_error_abs_p99 double precision,
    clamp_band_error_abs_max double precision,
    zero_count bigint,
    clamp_count bigint,
    orthogonality_sample_count bigint,
    orthogonality_mean double precision,
    orthogonality_spread double precision,
    orthogonality_abs_p99 double precision,
    orthogonality_abs_max double precision
)
STABLE PARALLEL UNSAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'vector_gamma_audit_internal_wrapper';
"#
)]
fn vector_gamma_audit_internal(
    index: Option<PgRelation>,
    field: Option<String>,
    queries: Option<AnyArray>,
) -> Result<
    TableIterator<
        'static,
        (
            name!(source, String),
            name!(protocol, String),
            name!(depth, i32),
            name!(bias, f32),
            name!(spread, f32),
            name!(sample_count, i32),
            name!(gamma_sample_count, i64),
            name!(gamma_mean, f64),
            name!(gamma_spread, f64),
            name!(gamma_min, f64),
            name!(gamma_p50, f64),
            name!(gamma_p95, f64),
            name!(gamma_p99, f64),
            name!(gamma_max, f64),
            name!(f16_band_error_sample_count, i64),
            name!(f16_band_error_mean, f64),
            name!(f16_band_error_spread, f64),
            name!(f16_band_error_abs_p99, f64),
            name!(f16_band_error_abs_max, f64),
            name!(clamp_band_error_sample_count, i64),
            name!(clamp_band_error_mean, f64),
            name!(clamp_band_error_spread, f64),
            name!(clamp_band_error_abs_p99, f64),
            name!(clamp_band_error_abs_max, f64),
            name!(zero_count, i64),
            name!(clamp_count, i64),
            name!(orthogonality_sample_count, i64),
            name!(orthogonality_mean, f64),
            name!(orthogonality_spread, f64),
            name!(orthogonality_abs_p99, f64),
            name!(orthogonality_abs_max, f64),
        ),
    >,
> {
    let index = index.context("index must not be NULL")?;
    let field = field.context("field must not be NULL")?;
    let queries = queries.context("queries must not be NULL")?;
    let index_oid = index.oid();
    reject_partitioned_index(index_oid, PartitionedIndexOperation::GammaAudit)?;
    drop(index);

    // This endpoint is deliberately read-only: AccessShare keeps the physical
    // index stable while Snapshot visibility determines the live rows. In
    // particular, there is no settings-list replacement or metapage swap.
    let index = PgSearchRelation::with_lock(index_oid, pg_sys::AccessShareLock as _);
    ensure!(
        unsafe { pg_sys::get_rel_relkind(index.oid()) as u8 } == pg_sys::RELKIND_INDEX,
        "vector gamma audit requires a physical index"
    );
    ensure!(
        is_bm25_index(&index),
        "vector gamma audit requires a ParadeDB index"
    );
    ensure!(index.is_usable(), "index is not valid, ready, and live");

    let settings = load_index_settings(&index)?
        .with_context(|| format!("index {:?} has no persisted settings", index.name()))?;
    let quantization = settings
        .vector_quantization
        .iter()
        .find(|config| config.field == field)
        .with_context(|| format!("field {field:?} is not configured for quantization"))?;
    let expected_dim = quantization.dim;
    let (external_queries, input_query_count) = decode_queries_capped(
        &queries,
        expected_dim,
        GAMMA_AUDIT_QUERY_COUNT,
        "gamma audit",
    )?;
    ensure!(
        input_query_count >= GAMMA_AUDIT_QUERY_COUNT,
        "vector gamma audit requires at least {GAMMA_AUDIT_QUERY_COUNT} queries; received {input_query_count}"
    );
    debug_assert_eq!(external_queries.len(), GAMMA_AUDIT_QUERY_COUNT);
    let external_queries = external_queries
        .into_iter()
        .map(|values| VectorGammaAuditQuery {
            values,
            excluded_doc_id: None,
        })
        .collect::<Vec<_>>();

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
    let sample_rows = allocate_sample_rows(&membership_rows, GAMMA_AUDIT_SAMPLE_ROWS);
    ensure!(
        sample_rows.iter().sum::<usize>() != 0,
        "field {field:?} has no visible IVF posting-membership rows to audit"
    );

    let distinct_rows = search_reader
        .segment_readers()
        .iter()
        .map(|segment_reader| -> Result<u64> {
            let vector_index = segment_reader.vector_index(vector_field)?;
            u64::try_from(vector_index.live_distinct_vector_count(segment_reader.alive_bitset()))
                .context("live distinct-vector count exceeds u64")
        })
        .collect::<Result<Vec<_>>>()?;
    let held_out_allocations = allocate_sample_rows(&distinct_rows, GAMMA_AUDIT_QUERY_COUNT);
    ensure!(
        held_out_allocations.iter().sum::<usize>() == GAMMA_AUDIT_QUERY_COUNT,
        "vector gamma audit requires at least {GAMMA_AUDIT_QUERY_COUNT} visible stored vectors; found {}",
        distinct_rows.iter().sum::<u64>()
    );

    let mut held_out_queries = Vec::with_capacity(GAMMA_AUDIT_QUERY_COUNT);
    for (origin_segment, (segment_reader, &segment_query_count)) in search_reader
        .segment_readers()
        .iter()
        .zip(&held_out_allocations)
        .enumerate()
    {
        if segment_query_count == 0 {
            continue;
        }
        let vector_index = segment_reader.vector_index(vector_field)?;
        let segment_queries = vector_index
            .sample_gamma_pseudo_queries(segment_query_count, segment_reader.alive_bitset())?
            .with_context(|| {
                format!(
                    "visible segment {} has vectors but no quantized storage for field {field:?}",
                    segment_reader.segment_id().short_uuid_string()
                )
            })?;
        ensure!(
            segment_queries.len() == segment_query_count,
            "segment {} returned {} held-out gamma queries; expected {segment_query_count}",
            segment_reader.segment_id().short_uuid_string(),
            segment_queries.len()
        );
        ensure!(
            segment_queries
                .iter()
                .all(|query| query.excluded_doc_id.is_some()),
            "held-out gamma queries from their origin segment must carry a document id"
        );
        held_out_queries.extend(
            segment_queries
                .into_iter()
                .map(|query| (origin_segment, query)),
        );
    }
    debug_assert_eq!(held_out_queries.len(), GAMMA_AUDIT_QUERY_COUNT);

    let mut real_measurements: Option<VectorGammaAuditMeasurements> = None;
    let mut held_out_measurements: Option<VectorGammaAuditMeasurements> = None;
    for (target_segment, (segment_reader, &segment_sample_rows)) in search_reader
        .segment_readers()
        .iter()
        .zip(&sample_rows)
        .enumerate()
    {
        if segment_sample_rows == 0 {
            continue;
        }
        let vector_index = segment_reader.vector_index(vector_field)?;
        let segment_real = vector_index
            .audit_gamma_queries(
                VectorQuantizationCalibrationSource::RealQuery,
                &external_queries,
                segment_sample_rows,
                segment_reader.alive_bitset(),
            )?
            .with_context(|| {
                format!(
                    "visible segment {} has sampled rows but no quantized storage for field {field:?}",
                    segment_reader.segment_id().short_uuid_string()
                )
            })?;
        merge_gamma_measurements(&mut real_measurements, segment_real)?;

        let segment_held_out_queries = held_out_queries
            .iter()
            .map(|(origin_segment, query)| VectorGammaAuditQuery {
                values: query.values.clone(),
                excluded_doc_id: if *origin_segment == target_segment {
                    query.excluded_doc_id
                } else {
                    None
                },
            })
            .collect::<Vec<_>>();
        let segment_held_out = vector_index
            .audit_gamma_queries(
                VectorQuantizationCalibrationSource::HeldOut,
                &segment_held_out_queries,
                segment_sample_rows,
                segment_reader.alive_bitset(),
            )?
            .with_context(|| {
                format!(
                    "visible segment {} has sampled rows but no quantized storage for field {field:?}",
                    segment_reader.segment_id().short_uuid_string()
                )
            })?;
        merge_gamma_measurements(&mut held_out_measurements, segment_held_out)?;
    }

    let held_out_measurements = held_out_measurements
        .context("no visible quantized segment supplied held-out gamma measurements")?;
    let real_measurements = real_measurements
        .context("no visible quantized segment supplied real-query gamma measurements")?;
    let mut rows = gamma_audit_rows(&held_out_measurements)?;
    rows.extend(gamma_audit_rows(&real_measurements)?);
    Ok(TableIterator::new(rows))
}

/// Measure only confidence-band survivor behavior: every cluster and every
/// visible posting membership is admitted, so routing and probe budgets cannot
/// hide a band miss. Gamma-corrected estimates use zero centering by contract;
/// the separate 1K cross-product endpoint remains the bias regression gate.
#[pg_extern(
    name = "vector_gamma_cone_audit",
    sql = r#"
CREATE FUNCTION paradedb.vector_gamma_cone_audit(
    index regclass,
    field text,
    queries vector[]
) RETURNS TABLE(
    protocol text,
    depth integer,
    kappa real,
    query_count integer,
    top_k integer,
    mean_scored_rows double precision,
    mean_survivor_rows double precision,
    mean_survivor_docs double precision,
    mean_survivor_fraction double precision,
    mean_candidate_recall double precision,
    min_candidate_recall double precision,
    queries_with_miss integer,
    final_depth boolean
)
STABLE PARALLEL UNSAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'vector_gamma_cone_audit_internal_wrapper';
"#
)]
fn vector_gamma_cone_audit_internal(
    index: Option<PgRelation>,
    field: Option<String>,
    queries: Option<AnyArray>,
) -> Result<
    TableIterator<
        'static,
        (
            name!(protocol, String),
            name!(depth, i32),
            name!(kappa, f32),
            name!(query_count, i32),
            name!(top_k, i32),
            name!(mean_scored_rows, f64),
            name!(mean_survivor_rows, f64),
            name!(mean_survivor_docs, f64),
            name!(mean_survivor_fraction, f64),
            name!(mean_candidate_recall, f64),
            name!(min_candidate_recall, f64),
            name!(queries_with_miss, i32),
            name!(final_depth, bool),
        ),
    >,
> {
    let index = index.context("index must not be NULL")?;
    let field = field.context("field must not be NULL")?;
    let queries = queries.context("queries must not be NULL")?;
    let index_oid = index.oid();
    reject_partitioned_index(index_oid, PartitionedIndexOperation::GammaConeAudit)?;
    drop(index);

    let index = PgSearchRelation::with_lock(index_oid, pg_sys::AccessShareLock as _);
    ensure!(
        unsafe { pg_sys::get_rel_relkind(index.oid()) as u8 } == pg_sys::RELKIND_INDEX,
        "vector gamma cone audit requires a physical index"
    );
    ensure!(
        is_bm25_index(&index),
        "vector gamma cone audit requires a ParadeDB index"
    );
    ensure!(index.is_usable(), "index is not valid, ready, and live");

    let settings = load_index_settings(&index)?
        .with_context(|| format!("index {:?} has no persisted settings", index.name()))?;
    let quantization = settings
        .vector_quantization
        .iter()
        .find(|config| config.field == field)
        .with_context(|| format!("field {field:?} is not configured for quantization"))?;
    let (external_queries, input_query_count) = decode_queries_capped(
        &queries,
        quantization.dim,
        GAMMA_AUDIT_QUERY_COUNT,
        "gamma cone audit",
    )?;
    ensure!(
        input_query_count == GAMMA_AUDIT_QUERY_COUNT,
        "vector gamma cone audit requires exactly {GAMMA_AUDIT_QUERY_COUNT} queries; received {input_query_count}"
    );
    let external_queries = external_queries
        .into_iter()
        .map(|values| VectorGammaAuditQuery {
            values,
            excluded_doc_id: None,
        })
        .collect::<Vec<_>>();

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
        vector_options.dim() == quantization.dim,
        "quantization dimension {} for field {field:?} does not match schema dimension {}",
        quantization.dim,
        vector_options.dim()
    );

    // A cone boundary is segment-wide. Reject multiple physical vector
    // segments rather than silently applying per-segment boundaries and
    // calling their average an index-wide result.
    let mut vector_segments = Vec::new();
    for segment_reader in search_reader.segment_readers() {
        let vector_index = segment_reader.vector_index(vector_field)?;
        if vector_index.num_vectors() != 0 {
            vector_segments.push((segment_reader, vector_index));
        }
    }
    ensure!(
        vector_segments.len() == 1,
        "vector gamma cone audit requires exactly one non-empty vector segment; found {}",
        vector_segments.len()
    );
    let (segment_reader, vector_index) = vector_segments.into_iter().next().unwrap();
    let measurements = vector_index
        .audit_gamma_cone(&external_queries, segment_reader.alive_bitset())?
        .with_context(|| {
            format!(
                "segment {} has no quantized IVF storage for field {field:?}",
                segment_reader.segment_id().short_uuid_string()
            )
        })?;
    Ok(TableIterator::new(gamma_cone_audit_rows(&measurements)?))
}

#[derive(Clone, Copy)]
enum PartitionedIndexOperation {
    Calibration,
    GammaAudit,
    GammaConeAudit,
}

fn reject_partitioned_index(
    index_oid: pg_sys::Oid,
    operation: PartitionedIndexOperation,
) -> Result<()> {
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
    let (message, hint) = match operation {
        PartitionedIndexOperation::Calibration => (
            format!(
                "cannot calibrate a partitioned index parent; child indexes: {children}; calibrate each child index individually with paradedb.vector_calibrate"
            ),
            "call paradedb.vector_calibrate for each child index individually".to_string(),
        ),
        PartitionedIndexOperation::GammaAudit => (
            format!(
                "cannot audit gamma on a partitioned index parent; child indexes: {children}; audit each child index individually with paradedb.vector_gamma_audit"
            ),
            "call paradedb.vector_gamma_audit for each child index individually".to_string(),
        ),
        PartitionedIndexOperation::GammaConeAudit => (
            format!(
                "cannot audit a gamma cone on a partitioned index parent; child indexes: {children}; audit each child index individually with paradedb.vector_gamma_cone_audit"
            ),
            "call paradedb.vector_gamma_cone_audit for each child index individually".to_string(),
        ),
    };
    pgrx::pg_sys::panic::ErrorReport::new(
        pgrx::PgSqlErrorCode::ERRCODE_WRONG_OBJECT_TYPE,
        message,
        pgrx::function_name!(),
    )
    .set_hint(hint)
    .report(pgrx::PgLogLevel::ERROR);
    Ok(())
}

fn decode_queries(queries: &AnyArray, expected_dim: usize) -> Result<Vec<Vec<f32>>> {
    decode_queries_capped(
        queries,
        expected_dim,
        MAX_CALIBRATION_QUERIES,
        "quantization calibration",
    )
    .map(|(queries, _)| queries)
}

/// Validate every supplied array element while retaining only the bounded
/// prefix used by the caller's measurement protocol.
fn decode_queries_capped(
    queries: &AnyArray,
    expected_dim: usize,
    max_queries: usize,
    query_kind: &str,
) -> Result<(Vec<Vec<f32>>, usize)> {
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
        let datum =
            datum.with_context(|| format!("{query_kind} query {} is NULL", query_idx + 1))?;
        // SAFETY: the array element OID was checked against pgvector's OID
        // above, and `Array::iter` established that this element is non-NULL.
        // `PgVector` detoasts and copies the datum before returning.
        let query = unsafe { PgVector::from_polymorphic_datum(datum, false, element_oid) }
            .with_context(|| format!("could not decode {query_kind} query {}", query_idx + 1))?;
        ensure!(
            query.0.len() == expected_dim,
            "{query_kind} query {} has dimension {}; expected {expected_dim}",
            query_idx + 1,
            query.0.len()
        );
        input_count += 1;
        if decoded.len() < max_queries {
            decoded.push(query.0);
        }
    }
    ensure!(input_count != 0, "queries must not be empty");
    Ok((decoded, input_count))
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
    Ok((
        i32::try_from(depth + 1).context("quantization depth exceeds SQL integer range")?,
        calibration.bias,
        calibration.spread,
        i32::try_from(calibration.sample_count)
            .context("quantization calibration sample count exceeds SQL integer range")?,
        calibration_source_label(calibration.source).to_string(),
    ))
}

fn calibration_source_label(source: VectorQuantizationCalibrationSource) -> &'static str {
    match source {
        VectorQuantizationCalibrationSource::HeldOut => "held_out",
        VectorQuantizationCalibrationSource::RealQuery => "real_query",
    }
}

fn gamma_protocol(source: VectorQuantizationCalibrationSource) -> &'static str {
    match source {
        VectorQuantizationCalibrationSource::HeldOut => HELD_OUT_GAMMA_PROTOCOL,
        VectorQuantizationCalibrationSource::RealQuery => REAL_QUERY_GAMMA_PROTOCOL,
    }
}

fn merge_gamma_measurements(
    aggregate: &mut Option<VectorGammaAuditMeasurements>,
    segment: VectorGammaAuditMeasurements,
) -> Result<()> {
    if let Some(aggregate) = aggregate {
        aggregate.merge(&segment)?;
    } else {
        *aggregate = Some(segment);
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct DistributionSummary {
    sample_count: i64,
    mean: f64,
    spread: f64,
    min: f64,
    p50: f64,
    p95: f64,
    p99: f64,
    max: f64,
}

#[derive(Clone, Copy)]
struct ErrorSummary {
    sample_count: i64,
    mean: f64,
    spread: f64,
    abs_p99: f64,
    abs_max: f64,
}

fn moment_count(moments: &VectorAuditMoments, label: &str) -> Result<i64> {
    i64::try_from(moments.sample_count)
        .with_context(|| format!("{label} sample count exceeds SQL bigint range"))
}

fn distribution_summary(moments: &VectorAuditMoments, label: &str) -> Result<DistributionSummary> {
    Ok(DistributionSummary {
        sample_count: moment_count(moments, label)?,
        mean: moments
            .mean()
            .with_context(|| format!("{label} has no finite measurements"))?,
        spread: moments
            .spread()
            .with_context(|| format!("{label} has no finite measurements"))?,
        min: (moments.sample_count != 0)
            .then_some(moments.min)
            .with_context(|| format!("{label} has no finite measurements"))?,
        p50: moments
            .p50()
            .with_context(|| format!("{label} has no finite measurements"))?,
        p95: moments
            .p95()
            .with_context(|| format!("{label} has no finite measurements"))?,
        p99: moments
            .p99()
            .with_context(|| format!("{label} has no finite measurements"))?,
        max: (moments.sample_count != 0)
            .then_some(moments.max)
            .with_context(|| format!("{label} has no finite measurements"))?,
    })
}

fn error_summary(moments: &VectorAuditMoments, label: &str) -> Result<ErrorSummary> {
    Ok(ErrorSummary {
        sample_count: moment_count(moments, label)?,
        mean: moments
            .mean()
            .with_context(|| format!("{label} has no finite measurements"))?,
        spread: moments
            .spread()
            .with_context(|| format!("{label} has no finite measurements"))?,
        abs_p99: moments
            .p99_abs()
            .with_context(|| format!("{label} has no finite measurements"))?,
        abs_max: moments
            .max_abs()
            .with_context(|| format!("{label} has no finite measurements"))?,
    })
}

fn gamma_audit_rows(measurements: &VectorGammaAuditMeasurements) -> Result<Vec<GammaAuditRow>> {
    let calibration = measurements.calibration.finish(measurements.source)?;
    ensure!(
        calibration.len() == measurements.depths.len(),
        "gamma audit calibration and diagnostic depth counts differ"
    );
    let source = calibration_source_label(measurements.source);
    let protocol = gamma_protocol(measurements.source);
    measurements
        .depths
        .iter()
        .zip(&calibration)
        .enumerate()
        .map(|(depth, (diagnostics, calibration))| {
            let gamma = distribution_summary(&diagnostics.gamma_raw, "gamma")?;
            let f16 = error_summary(&diagnostics.f16_band_error, "f16 band error")?;
            let clamp = error_summary(&diagnostics.clamp_band_error, "clamp band error")?;
            let orthogonality =
                error_summary(&diagnostics.orthogonality_defect, "orthogonality defect")?;
            Ok((
                source.to_string(),
                protocol.to_string(),
                i32::try_from(depth + 1).context("quantization depth exceeds SQL integer range")?,
                calibration.bias,
                calibration.spread,
                i32::try_from(calibration.sample_count)
                    .context("gamma audit sample count exceeds SQL integer range")?,
                gamma.sample_count,
                gamma.mean,
                gamma.spread,
                gamma.min,
                gamma.p50,
                gamma.p95,
                gamma.p99,
                gamma.max,
                f16.sample_count,
                f16.mean,
                f16.spread,
                f16.abs_p99,
                f16.abs_max,
                clamp.sample_count,
                clamp.mean,
                clamp.spread,
                clamp.abs_p99,
                clamp.abs_max,
                i64::try_from(diagnostics.zero_count)
                    .context("gamma zero count exceeds SQL bigint range")?,
                i64::try_from(diagnostics.clamp_count)
                    .context("gamma clamp count exceeds SQL bigint range")?,
                orthogonality.sample_count,
                orthogonality.mean,
                orthogonality.spread,
                orthogonality.abs_p99,
                orthogonality.abs_max,
            ))
        })
        .collect()
}

fn gamma_cone_audit_rows(
    measurements: &VectorGammaConeAuditMeasurements,
) -> Result<Vec<GammaConeAuditRow>> {
    ensure!(
        measurements.query_count != 0 && measurements.top_k != 0,
        "gamma cone audit returned an empty query or top-k protocol"
    );
    let depth_count = measurements.depths.len();
    measurements
        .depths
        .iter()
        .enumerate()
        .map(|(depth, measurement)| {
            let mean = |moments: &VectorAuditMoments, label: &str| {
                moments
                    .mean()
                    .with_context(|| format!("gamma cone {label} has no measurements"))
            };
            ensure!(
                measurement.candidate_recall.sample_count == u64::from(measurements.query_count),
                "gamma cone depth {} measured {} queries; expected {}",
                depth + 1,
                measurement.candidate_recall.sample_count,
                measurements.query_count
            );
            Ok((
                GAMMA_CONE_PROTOCOL.to_string(),
                i32::try_from(depth + 1).context("gamma cone depth exceeds SQL integer range")?,
                measurement.kappa,
                i32::try_from(measurements.query_count)
                    .context("gamma cone query count exceeds SQL integer range")?,
                i32::try_from(measurements.top_k)
                    .context("gamma cone top-k exceeds SQL integer range")?,
                mean(&measurement.scored_rows, "scored rows")?,
                mean(&measurement.survivor_rows, "survivor rows")?,
                mean(&measurement.survivor_docs, "survivor docs")?,
                mean(&measurement.survivor_fraction, "survivor fraction")?,
                mean(&measurement.candidate_recall, "candidate recall")?,
                measurement.candidate_recall.min,
                i32::try_from(measurement.queries_with_miss)
                    .context("gamma cone miss count exceeds SQL integer range")?,
                depth + 1 == depth_count,
            ))
        })
        .collect()
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
