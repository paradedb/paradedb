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

//! SQL diagnostics for vector quantization.

use crate::index::directory::utils::load_index_settings;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::catalog::is_pgvector_oid;
use crate::postgres::is_bm25_index;
use crate::postgres::rel::PgSearchRelation;
use crate::vector::PgVector;
use anyhow::{Context, Result, bail, ensure};
#[cfg(feature = "pg_test")]
use pgrx::JsonB;
use pgrx::prelude::*;
use pgrx::{AnyArray, PgRelation, Spi, pg_sys};
use tantivy::schema::FieldType;
#[cfg(feature = "pg_test")]
use tantivy::vector::{
    VectorAuditMoments, VectorErrorAuditMeasurements, VectorErrorConeAuditMeasurements,
};
use tantivy::vector::{VectorEstimatorMeasurements, VectorEstimatorQuery, VectorEstimatorSource};

const MAX_ESTIMATOR_QUERIES: usize = 256;
const ESTIMATOR_SAMPLE_ROWS: usize = 1_000;
const HELD_OUT_ESTIMATOR_QUERIES: usize = 100;

type EstimatorInfoRow = (
    name!(depth, i32),
    name!(bias, f32),
    name!(spread, f32),
    name!(sample_rows, i32),
    name!(query_count, i32),
    name!(query_source, String),
);

#[cfg(feature = "pg_test")]
const EXACT_E_AUDIT_QUERY_COUNT: usize = 100;
#[cfg(feature = "pg_test")]
const EXACT_E_AUDIT_SAMPLE_ROWS: usize = 1_000;
#[cfg(feature = "pg_test")]
const REAL_QUERY_EXACT_E_PROTOCOL: &str = "REAL_QUERY_EXACT_E_BQ4";
#[cfg(feature = "pg_test")]
const HELD_OUT_EXACT_E_PROTOCOL: &str = "HELD_OUT_EXACT_E_BQ4";
#[cfg(feature = "pg_test")]
const EXACT_E_CONE_PROTOCOL: &str = "ALL_CLUSTERS_EXACT_E_CONE_K10_KAPPA2";

#[cfg(feature = "pg_test")]
type ExactEAuditRow = (
    name!(source, String),
    name!(protocol, String),
    name!(depth, i32),
    name!(bias, f32),
    name!(spread, f32),
    name!(sample_count, i32),
    name!(residual_norm_squared_sample_count, i64),
    name!(residual_norm_squared_mean, f64),
    name!(residual_norm_squared_spread, f64),
    name!(residual_norm_squared_p95, f64),
    name!(residual_norm_squared_p99, f64),
    name!(residual_norm_squared_max, f64),
    name!(gamma_sample_count, i64),
    name!(gamma_mean, f64),
    name!(gamma_spread, f64),
    name!(gamma_p95, f64),
    name!(gamma_p99, f64),
    name!(gamma_max, f64),
    name!(corrected_error_ratio_sample_count, i64),
    name!(corrected_error_ratio_mean, f64),
    name!(corrected_error_ratio_spread, f64),
    name!(corrected_error_ratio_p95, f64),
    name!(corrected_error_ratio_p99, f64),
    name!(corrected_error_ratio_max, f64),
    name!(sigma_sample_count, i64),
    name!(sigma_mean, f64),
    name!(sigma_spread, f64),
    name!(sigma_p95, f64),
    name!(sigma_p99, f64),
    name!(sigma_max, f64),
    name!(gamma_diagnostics, JsonB),
);

#[cfg(feature = "pg_test")]
type ExactEConeAuditRow = (
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

/// Measures normalized quantized-estimator errors without running a search.
///
/// The work is approximately 1,000 sampled rows times the query count per depth. It takes seconds
/// on a warm index and is not intended for per-request use.
///
/// # Errors
///
/// Returns an error for invalid input, a partitioned parent, or unavailable quantized storage.
#[pg_extern(
    name = "vector_estimator_info",
    sql = r#"
CREATE FUNCTION paradedb.vector_estimator_info(
    index regclass,
    field text,
    queries vector[] DEFAULT NULL
) RETURNS TABLE(
    depth integer,
    bias real,
    spread real,
    sample_rows integer,
    query_count integer,
    query_source text
)
STABLE PARALLEL UNSAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'vector_estimator_info_internal_wrapper';
"#
)]
fn vector_estimator_info_internal(
    index: Option<PgRelation>,
    field: Option<String>,
    queries: Option<AnyArray>,
) -> Result<TableIterator<'static, EstimatorInfoRow>> {
    let index = index.context("index must not be NULL")?;
    let field = field.context("field must not be NULL")?;
    let index_oid = index.oid();
    drop(index);

    let index = PgSearchRelation::with_lock(index_oid, pg_sys::AccessShareLock as _);
    reject_partitioned_index(index.oid(), PartitionedIndexOperation::EstimatorInfo)?;
    ensure!(
        unsafe { pg_sys::get_rel_relkind(index.oid()) as u8 } == pg_sys::RELKIND_INDEX
            && is_bm25_index(&index),
        "vector_estimator_info requires a ParadeDB index"
    );
    ensure!(index.is_usable(), "index is not valid, ready, and live");

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
    let settings = load_index_settings(&index)?;
    let quantization = settings
        .as_ref()
        .into_iter()
        .flat_map(|settings| &settings.vector_quantization)
        .find(|config| config.field == field)
        .with_context(|| format!("field is not quantized; nothing to diagnose: {field:?}"))?;
    let expected_dim = quantization.dim;
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
    let sample_rows = allocate_sample_rows(&membership_rows, ESTIMATOR_SAMPLE_ROWS);
    ensure!(
        sample_rows.iter().sum::<usize>() != 0,
        "field {field:?} has no visible IVF posting-membership rows to diagnose"
    );

    let (queries, query_source) = match queries {
        Some(queries) => {
            let queries = decode_estimator_queries(&queries, expected_dim)?
                .into_iter()
                .map(|values| VectorEstimatorQuery {
                    values,
                    excluded_doc_id: None,
                })
                .map(|query| (None, query))
                .collect();
            (queries, "provided")
        }
        None => (
            sample_held_out_queries(&search_reader, vector_field)?,
            "held_out",
        ),
    };
    let estimator_source = match query_source {
        "provided" => VectorEstimatorSource::Provided,
        "held_out" => VectorEstimatorSource::HeldOut,
        _ => unreachable!("query source is selected above"),
    };

    let mut measurements: Option<VectorEstimatorMeasurements> = None;
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
        let target_queries = queries
            .iter()
            .map(|(origin_segment, query)| VectorEstimatorQuery {
                values: query.values.clone(),
                excluded_doc_id: (*origin_segment == Some(target_segment))
                    .then_some(query.excluded_doc_id)
                    .flatten(),
            })
            .collect::<Vec<_>>();
        let segment_measurements = vector_index
            .measure_estimator_queries(
                estimator_source,
                &target_queries,
                segment_sample_rows,
                segment_reader.alive_bitset(),
            )?
            .with_context(|| {
                format!(
                    "visible segment {} has sampled rows but no quantized storage for field {field:?}",
                    segment_reader.segment_id().short_uuid_string()
                )
            })?;
        merge_estimator_measurements(&mut measurements, segment_measurements)?;
    }

    let measurements =
        measurements.context("no visible quantized segment supplied estimator measurements")?;
    ensure!(
        measurements.source() == estimator_source,
        "estimator measurement source does not match the requested query source"
    );
    let sample_rows = i32::try_from(measurements.sample_rows())
        .context("estimator sample-row count exceeds SQL integer range")?;
    let query_count = i32::try_from(measurements.query_count())
        .context("estimator query count exceeds SQL integer range")?;
    let rows = measurements
        .aggregate()
        .iter()
        .enumerate()
        .map(|(depth, moments)| {
            Ok((
                i32::try_from(depth + 1).context("quantization depth exceeds SQL integer range")?,
                measurement_to_real(
                    moments
                        .bias()
                        .context("estimator depth has no finite bias measurements")?,
                    "estimator bias",
                )?,
                measurement_to_real(
                    moments
                        .spread()
                        .context("estimator depth has no finite spread measurements")?,
                    "estimator spread",
                )?,
                sample_rows,
                query_count,
                query_source.to_string(),
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(TableIterator::new(rows))
}

/// Measures exact-error estimator diagnostics without changing index settings.
///
/// # Errors
///
/// Returns an error for invalid input or unavailable quantized storage.
#[cfg(feature = "pg_test")]
#[pg_extern(
    name = "vector_error_audit",
    sql = r#"
CREATE FUNCTION paradedb.vector_error_audit(
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
    residual_norm_squared_sample_count bigint,
    residual_norm_squared_mean double precision,
    residual_norm_squared_spread double precision,
    residual_norm_squared_p95 double precision,
    residual_norm_squared_p99 double precision,
    residual_norm_squared_max double precision,
    gamma_sample_count bigint,
    gamma_mean double precision,
    gamma_spread double precision,
    gamma_p95 double precision,
    gamma_p99 double precision,
    gamma_max double precision,
    corrected_error_ratio_sample_count bigint,
    corrected_error_ratio_mean double precision,
    corrected_error_ratio_spread double precision,
    corrected_error_ratio_p95 double precision,
    corrected_error_ratio_p99 double precision,
    corrected_error_ratio_max double precision,
    sigma_sample_count bigint,
    sigma_mean double precision,
    sigma_spread double precision,
    sigma_p95 double precision,
    sigma_p99 double precision,
    sigma_max double precision,
    gamma_diagnostics jsonb
)
STABLE PARALLEL UNSAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'vector_error_audit_internal_wrapper';
"#
)]
fn vector_error_audit_internal(
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
            name!(residual_norm_squared_sample_count, i64),
            name!(residual_norm_squared_mean, f64),
            name!(residual_norm_squared_spread, f64),
            name!(residual_norm_squared_p95, f64),
            name!(residual_norm_squared_p99, f64),
            name!(residual_norm_squared_max, f64),
            name!(gamma_sample_count, i64),
            name!(gamma_mean, f64),
            name!(gamma_spread, f64),
            name!(gamma_p95, f64),
            name!(gamma_p99, f64),
            name!(gamma_max, f64),
            name!(corrected_error_ratio_sample_count, i64),
            name!(corrected_error_ratio_mean, f64),
            name!(corrected_error_ratio_spread, f64),
            name!(corrected_error_ratio_p95, f64),
            name!(corrected_error_ratio_p99, f64),
            name!(corrected_error_ratio_max, f64),
            name!(sigma_sample_count, i64),
            name!(sigma_mean, f64),
            name!(sigma_spread, f64),
            name!(sigma_p95, f64),
            name!(sigma_p99, f64),
            name!(sigma_max, f64),
            name!(gamma_diagnostics, JsonB),
        ),
    >,
> {
    let index = index.context("index must not be NULL")?;
    let field = field.context("field must not be NULL")?;
    let queries = queries.context("queries must not be NULL")?;
    let index_oid = index.oid();
    reject_partitioned_index(index_oid, PartitionedIndexOperation::ErrorAudit)?;
    drop(index);

    let index = PgSearchRelation::with_lock(index_oid, pg_sys::AccessShareLock as _);
    ensure!(
        unsafe { pg_sys::get_rel_relkind(index.oid()) as u8 } == pg_sys::RELKIND_INDEX,
        "vector error audit requires a physical index"
    );
    ensure!(
        is_bm25_index(&index),
        "vector error audit requires a ParadeDB index"
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
        EXACT_E_AUDIT_QUERY_COUNT,
        "error audit",
    )?;
    ensure!(
        input_query_count >= EXACT_E_AUDIT_QUERY_COUNT,
        "vector error audit requires at least {EXACT_E_AUDIT_QUERY_COUNT} queries; received {input_query_count}"
    );
    debug_assert_eq!(external_queries.len(), EXACT_E_AUDIT_QUERY_COUNT);
    let external_queries = external_queries
        .into_iter()
        .map(|values| VectorEstimatorQuery {
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
    let sample_rows = allocate_sample_rows(&membership_rows, EXACT_E_AUDIT_SAMPLE_ROWS);
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
    let held_out_allocations = allocate_sample_rows(&distinct_rows, EXACT_E_AUDIT_QUERY_COUNT);
    ensure!(
        held_out_allocations.iter().sum::<usize>() == EXACT_E_AUDIT_QUERY_COUNT,
        "vector error audit requires at least {EXACT_E_AUDIT_QUERY_COUNT} visible stored vectors; found {}",
        distinct_rows.iter().sum::<u64>()
    );

    let mut held_out_queries = Vec::with_capacity(EXACT_E_AUDIT_QUERY_COUNT);
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
            .sample_estimator_pseudo_queries(segment_query_count, segment_reader.alive_bitset())?
            .with_context(|| {
                format!(
                    "visible segment {} has vectors but no quantized storage for field {field:?}",
                    segment_reader.segment_id().short_uuid_string()
                )
            })?;
        ensure!(
            segment_queries.len() == segment_query_count,
            "segment {} returned {} held-out exact-E queries; expected {segment_query_count}",
            segment_reader.segment_id().short_uuid_string(),
            segment_queries.len()
        );
        ensure!(
            segment_queries
                .iter()
                .all(|query| query.excluded_doc_id.is_some()),
            "held-out exact-E queries from their origin segment must carry a document id"
        );
        held_out_queries.extend(
            segment_queries
                .into_iter()
                .map(|query| (origin_segment, query)),
        );
    }
    debug_assert_eq!(held_out_queries.len(), EXACT_E_AUDIT_QUERY_COUNT);

    let mut real_measurements: Option<VectorErrorAuditMeasurements> = None;
    let mut held_out_measurements: Option<VectorErrorAuditMeasurements> = None;
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
            .audit_error_queries(
                VectorEstimatorSource::Provided,
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
        merge_exact_e_measurements(&mut real_measurements, segment_real)?;

        let segment_held_out_queries = held_out_queries
            .iter()
            .map(|(origin_segment, query)| VectorEstimatorQuery {
                values: query.values.clone(),
                excluded_doc_id: if *origin_segment == target_segment {
                    query.excluded_doc_id
                } else {
                    None
                },
            })
            .collect::<Vec<_>>();
        let segment_held_out = vector_index
            .audit_error_queries(
                VectorEstimatorSource::HeldOut,
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
        merge_exact_e_measurements(&mut held_out_measurements, segment_held_out)?;
    }

    let held_out_measurements = held_out_measurements
        .context("no visible quantized segment supplied held-out exact-E measurements")?;
    let real_measurements = real_measurements
        .context("no visible quantized segment supplied real-query exact-E measurements")?;
    let mut rows = exact_e_audit_rows(&held_out_measurements)?;
    rows.extend(exact_e_audit_rows(&real_measurements)?);
    Ok(TableIterator::new(rows))
}

/// Measures confidence-band survivor behavior across all clusters.
///
/// # Errors
///
/// Returns an error for invalid input or unavailable quantized storage.
#[cfg(feature = "pg_test")]
#[pg_extern(
    name = "vector_error_cone_audit",
    sql = r#"
CREATE FUNCTION paradedb.vector_error_cone_audit(
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
AS 'MODULE_PATHNAME', 'vector_error_cone_audit_internal_wrapper';
"#
)]
fn vector_error_cone_audit_internal(
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
    reject_partitioned_index(index_oid, PartitionedIndexOperation::ErrorConeAudit)?;
    drop(index);

    let index = PgSearchRelation::with_lock(index_oid, pg_sys::AccessShareLock as _);
    ensure!(
        unsafe { pg_sys::get_rel_relkind(index.oid()) as u8 } == pg_sys::RELKIND_INDEX,
        "vector error cone audit requires a physical index"
    );
    ensure!(
        is_bm25_index(&index),
        "vector error cone audit requires a ParadeDB index"
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
        EXACT_E_AUDIT_QUERY_COUNT,
        "error cone audit",
    )?;
    ensure!(
        input_query_count == EXACT_E_AUDIT_QUERY_COUNT,
        "vector error cone audit requires exactly {EXACT_E_AUDIT_QUERY_COUNT} queries; received {input_query_count}"
    );
    let external_queries = external_queries
        .into_iter()
        .map(|values| VectorEstimatorQuery {
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

    let mut vector_segments = Vec::new();
    for segment_reader in search_reader.segment_readers() {
        let vector_index = segment_reader.vector_index(vector_field)?;
        if vector_index.num_vectors() != 0 {
            vector_segments.push((segment_reader, vector_index));
        }
    }
    ensure!(
        vector_segments.len() == 1,
        "vector error cone audit requires exactly one non-empty vector segment; found {}",
        vector_segments.len()
    );
    let (segment_reader, vector_index) = vector_segments.into_iter().next().unwrap();
    let measurements = vector_index
        .audit_error_cone(&external_queries, segment_reader.alive_bitset())?
        .with_context(|| {
            format!(
                "segment {} has no quantized IVF storage for field {field:?}",
                segment_reader.segment_id().short_uuid_string()
            )
        })?;
    Ok(TableIterator::new(exact_e_cone_audit_rows(&measurements)?))
}

#[derive(Clone, Copy)]
enum PartitionedIndexOperation {
    EstimatorInfo,
    #[cfg(feature = "pg_test")]
    ErrorAudit,
    #[cfg(feature = "pg_test")]
    ErrorConeAudit,
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
        PartitionedIndexOperation::EstimatorInfo => (
            format!(
                "cannot diagnose a partitioned index parent; child indexes: {children}; run vector_estimator_info for each child index"
            ),
            "run paradedb.vector_estimator_info for each child index individually".to_string(),
        ),
        #[cfg(feature = "pg_test")]
        PartitionedIndexOperation::ErrorAudit => (
            format!(
                "cannot audit exact-E on a partitioned index parent; child indexes: {children}; audit each child index individually with paradedb.vector_error_audit"
            ),
            "call paradedb.vector_error_audit for each child index individually".to_string(),
        ),
        #[cfg(feature = "pg_test")]
        PartitionedIndexOperation::ErrorConeAudit => (
            format!(
                "cannot audit an exact-E cone on a partitioned index parent; child indexes: {children}; audit each child index individually with paradedb.vector_error_cone_audit"
            ),
            "call paradedb.vector_error_cone_audit for each child index individually".to_string(),
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

fn decode_estimator_queries(queries: &AnyArray, expected_dim: usize) -> Result<Vec<Vec<f32>>> {
    let (queries, input_count) =
        decode_queries_capped(queries, expected_dim, MAX_ESTIMATOR_QUERIES, "estimator")?;
    ensure!(
        input_count <= MAX_ESTIMATOR_QUERIES,
        "vector_estimator_info accepts at most {MAX_ESTIMATOR_QUERIES} queries; received {input_count}"
    );
    Ok(queries)
}

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

    let array = pgrx::AnyArray::into::<pgrx::Array<pg_sys::Datum>>(queries)
        .context("could not decode queries vector[]")?;
    let mut decoded = Vec::new();
    let mut input_count = 0usize;
    for (query_idx, datum) in array.iter().enumerate() {
        let datum =
            datum.with_context(|| format!("{query_kind} query {} is NULL", query_idx + 1))?;
        // SAFETY: `element_oid` is pgvector and `datum` is non-null.
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

fn sample_held_out_queries(
    search_reader: &SearchIndexReader,
    vector_field: tantivy::schema::Field,
) -> Result<Vec<(Option<usize>, VectorEstimatorQuery)>> {
    let distinct_rows = search_reader
        .segment_readers()
        .iter()
        .map(|segment_reader| -> Result<u64> {
            let vector_index = segment_reader.vector_index(vector_field)?;
            u64::try_from(vector_index.live_distinct_vector_count(segment_reader.alive_bitset()))
                .context("live distinct-vector count exceeds u64")
        })
        .collect::<Result<Vec<_>>>()?;
    let allocations = allocate_sample_rows(&distinct_rows, HELD_OUT_ESTIMATOR_QUERIES);
    let total_distinct = distinct_rows.iter().sum::<u64>();
    ensure!(
        allocations.iter().sum::<usize>() == HELD_OUT_ESTIMATOR_QUERIES,
        "vector_estimator_info held-out mode requires at least {HELD_OUT_ESTIMATOR_QUERIES} visible stored vectors; found {total_distinct}"
    );

    let mut queries = Vec::with_capacity(HELD_OUT_ESTIMATOR_QUERIES);
    for (origin_segment, (segment_reader, &query_count)) in search_reader
        .segment_readers()
        .iter()
        .zip(&allocations)
        .enumerate()
    {
        if query_count == 0 {
            continue;
        }
        let vector_index = segment_reader.vector_index(vector_field)?;
        let segment_queries = vector_index
            .sample_estimator_pseudo_queries(query_count, segment_reader.alive_bitset())?
            .with_context(|| {
                format!(
                    "visible segment {} has vectors but no quantized storage",
                    segment_reader.segment_id().short_uuid_string()
                )
            })?;
        ensure!(
            segment_queries.len() == query_count,
            "segment {} returned {} held-out estimator queries; expected {query_count}",
            segment_reader.segment_id().short_uuid_string(),
            segment_queries.len()
        );
        ensure!(
            segment_queries
                .iter()
                .all(|query| query.excluded_doc_id.is_some()),
            "held-out estimator queries must carry their source document id"
        );
        queries.extend(
            segment_queries
                .into_iter()
                .map(|query| (Some(origin_segment), query)),
        );
    }
    ensure!(
        queries.len() == HELD_OUT_ESTIMATOR_QUERIES,
        "held-out estimator query count does not match its deterministic allocation"
    );
    Ok(queries)
}

fn merge_estimator_measurements(
    aggregate: &mut Option<VectorEstimatorMeasurements>,
    segment: VectorEstimatorMeasurements,
) -> Result<()> {
    if let Some(aggregate) = aggregate {
        aggregate.merge(&segment)?;
    } else {
        *aggregate = Some(segment);
    }
    Ok(())
}

fn measurement_to_real(value: f64, label: &str) -> Result<f32> {
    ensure!(
        value.is_finite() && value.abs() <= f64::from(f32::MAX),
        "{label} is outside the SQL real range"
    );
    Ok(value as f32)
}

#[cfg(feature = "pg_test")]
fn calibration_source_label(source: VectorEstimatorSource) -> &'static str {
    match source {
        VectorEstimatorSource::HeldOut => "held_out",
        VectorEstimatorSource::Provided => "real_query",
    }
}

#[cfg(feature = "pg_test")]
fn exact_e_protocol(source: VectorEstimatorSource) -> &'static str {
    match source {
        VectorEstimatorSource::HeldOut => HELD_OUT_EXACT_E_PROTOCOL,
        VectorEstimatorSource::Provided => REAL_QUERY_EXACT_E_PROTOCOL,
    }
}

#[cfg(feature = "pg_test")]
fn merge_exact_e_measurements(
    aggregate: &mut Option<VectorErrorAuditMeasurements>,
    segment: VectorErrorAuditMeasurements,
) -> Result<()> {
    if let Some(aggregate) = aggregate {
        aggregate.merge(&segment)?;
    } else {
        *aggregate = Some(segment);
    }
    Ok(())
}

#[cfg(feature = "pg_test")]
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

#[cfg(feature = "pg_test")]
fn moment_count(moments: &VectorAuditMoments, label: &str) -> Result<i64> {
    i64::try_from(moments.sample_count)
        .with_context(|| format!("{label} sample count exceeds SQL bigint range"))
}

#[cfg(feature = "pg_test")]
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

#[cfg(feature = "pg_test")]
fn gamma_diagnostics_json(
    diagnostics: &tantivy::vector::VectorErrorDepthMeasurements,
) -> Result<JsonB> {
    let stored = distribution_summary(&diagnostics.stored_gamma, "stored gamma")?;
    let raw = distribution_summary(&diagnostics.raw_gamma, "raw gamma")?;
    let round_trip = &diagnostics.gamma_round_trip_band_error;
    Ok(JsonB(serde_json::json!({
        "stored": {
            "sample_count": stored.sample_count,
            "mean": stored.mean,
            "spread": stored.spread,
            "min": stored.min,
            "p50": stored.p50,
            "p95": stored.p95,
            "p99": stored.p99,
            "max": stored.max,
        },
        "raw": {
            "sample_count": raw.sample_count,
            "mean": raw.mean,
            "spread": raw.spread,
            "min": raw.min,
            "p50": raw.p50,
            "p95": raw.p95,
            "p99": raw.p99,
            "max": raw.max,
        },
        "zero_scale_count": diagnostics.zero_scale_count,
        "lower_clamp_count": diagnostics.gamma_lower_clamp_count,
        "upper_clamp_count": diagnostics.gamma_upper_clamp_count,
        "round_trip_band_error": {
            "sample_count": moment_count(round_trip, "gamma round-trip band error")?,
            "p99_abs": round_trip
                .p99_abs()
                .context("gamma round-trip band error has no finite measurements")?,
            "max_abs": round_trip
                .max_abs()
                .context("gamma round-trip band error has no finite measurements")?,
        },
    })))
}

#[cfg(feature = "pg_test")]
fn exact_e_audit_rows(measurements: &VectorErrorAuditMeasurements) -> Result<Vec<ExactEAuditRow>> {
    let protocol = exact_e_protocol(measurements.source);
    ensure!(
        measurements.estimator.aggregate().len() == measurements.depths.len(),
        "exact-E audit estimator and diagnostic depth counts differ"
    );
    let source = calibration_source_label(measurements.source);
    measurements
        .depths
        .iter()
        .zip(measurements.estimator.aggregate())
        .enumerate()
        .map(|(depth, (diagnostics, estimator))| {
            let residual_norm_squared =
                distribution_summary(&diagnostics.residual_norm_squared, "residual squared norm")?;
            let gamma = distribution_summary(&diagnostics.stored_gamma, "stored gamma")?;
            let corrected_error_ratio =
                distribution_summary(&diagnostics.corrected_error_ratio, "corrected error ratio")?;
            let sigma = distribution_summary(&diagnostics.sigma, "production sigma")?;
            Ok((
                source.to_string(),
                protocol.to_string(),
                i32::try_from(depth + 1).context("quantization depth exceeds SQL integer range")?,
                measurement_to_real(
                    estimator
                        .bias()
                        .context("exact-E audit depth has no bias measurements")?,
                    "exact-E audit bias",
                )?,
                measurement_to_real(
                    estimator
                        .spread()
                        .context("exact-E audit depth has no spread measurements")?,
                    "exact-E audit spread",
                )?,
                i32::try_from(estimator.sample_count)
                    .context("exact-E audit sample count exceeds SQL integer range")?,
                residual_norm_squared.sample_count,
                residual_norm_squared.mean,
                residual_norm_squared.spread,
                residual_norm_squared.p95,
                residual_norm_squared.p99,
                residual_norm_squared.max,
                gamma.sample_count,
                gamma.mean,
                gamma.spread,
                gamma.p95,
                gamma.p99,
                gamma.max,
                corrected_error_ratio.sample_count,
                corrected_error_ratio.mean,
                corrected_error_ratio.spread,
                corrected_error_ratio.p95,
                corrected_error_ratio.p99,
                corrected_error_ratio.max,
                sigma.sample_count,
                sigma.mean,
                sigma.spread,
                sigma.p95,
                sigma.p99,
                sigma.max,
                gamma_diagnostics_json(diagnostics)?,
            ))
        })
        .collect()
}

#[cfg(feature = "pg_test")]
fn exact_e_cone_audit_rows(
    measurements: &VectorErrorConeAuditMeasurements,
) -> Result<Vec<ExactEConeAuditRow>> {
    ensure!(
        measurements.query_count != 0 && measurements.top_k != 0,
        "exact-E cone audit returned an empty query or top-k protocol"
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
                    .with_context(|| format!("exact-E cone {label} has no measurements"))
            };
            ensure!(
                measurement.candidate_recall.sample_count == u64::from(measurements.query_count),
                "exact-E cone depth {} measured {} queries; expected {}",
                depth + 1,
                measurement.candidate_recall.sample_count,
                measurements.query_count
            );
            Ok((
                EXACT_E_CONE_PROTOCOL.to_string(),
                i32::try_from(depth + 1).context("exact-E cone depth exceeds SQL integer range")?,
                measurement.kappa,
                i32::try_from(measurements.query_count)
                    .context("exact-E cone query count exceeds SQL integer range")?,
                i32::try_from(measurements.top_k)
                    .context("exact-E cone top-k exceeds SQL integer range")?,
                mean(&measurement.scored_rows, "scored rows")?,
                mean(&measurement.survivor_rows, "survivor rows")?,
                mean(&measurement.survivor_docs, "survivor docs")?,
                mean(&measurement.survivor_fraction, "survivor fraction")?,
                mean(&measurement.candidate_recall, "candidate recall")?,
                measurement.candidate_recall.min,
                i32::try_from(measurement.queries_with_miss)
                    .context("exact-E cone miss count exceeds SQL integer range")?,
                depth + 1 == depth_count,
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::allocate_sample_rows;

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
}
