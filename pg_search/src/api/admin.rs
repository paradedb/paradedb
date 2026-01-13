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

use crate::api::FieldName;
use crate::api::{HashMap, HashSet};
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::index::IndexKind;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::{
    LinkedList, MVCCEntry, SegmentMetaEntry, SegmentMetaEntryContent,
};
use crate::postgres::storage::metadata::MetaPage;
use crate::postgres::utils::item_pointer_to_u64;
use crate::query::pdb_query::pdb;
use crate::query::SearchQueryInput;
use crate::schema::IndexRecordOption;
use anyhow::Result;
use pgrx::prelude::*;
use pgrx::JsonB;
use pgrx::PgRelation;
use serde_json::Value;
use tantivy::schema::FieldType;

#[allow(clippy::type_complexity)]
#[pg_extern]
pub fn schema(
    index: PgRelation,
) -> TableIterator<
    'static,
    (
        name!(name, String),
        name!(field_type, String),
        name!(stored, bool),
        name!(indexed, bool),
        name!(fast, bool),
        name!(fieldnorms, bool),
        name!(expand_dots, Option<bool>),
        name!(tokenizer, Option<String>),
        name!(record, Option<String>),
        name!(normalizer, Option<String>),
    ),
> {
    // # Safety
    //
    // Lock the index relation until the end of this function so it is not dropped or
    // altered while we are reading it.
    //
    // Because we accept a PgRelation above, we have confidence that Postgres has already
    // validated the existence of the relation. We are safe calling the function below as
    // long we do not pass pg_sys::NoLock without any other locking mechanism of our own.
    let index = PgSearchRelation::with_lock(index.oid(), pg_sys::AccessShareLock as _);

    // We only consider the first partition for the purposes of computing a schema.
    let index = IndexKind::for_index(index)
        .unwrap()
        .partitions()
        .next()
        .expect("expected at least one partition of the index");

    let search_reader = SearchIndexReader::empty(&index, MvccSatisfies::Snapshot)
        .expect("could not open search index reader");
    let schema = search_reader.schema();

    let mut field_rows = Vec::new();
    for (_, field_entry) in schema.fields() {
        let (field_type, tokenizer, record, normalizer, expand_dots) =
            match field_entry.field_type() {
                FieldType::I64(_) => ("I64".to_string(), None, None, None, None),
                FieldType::U64(_) => ("U64".to_string(), None, None, None, None),
                FieldType::F64(_) => ("F64".to_string(), None, None, None, None),
                FieldType::Bool(_) => ("Bool".to_string(), None, None, None, None),
                FieldType::Str(text_options) => {
                    let indexing_options = text_options.get_indexing_options();
                    let tokenizer = indexing_options.map(|opt| opt.tokenizer().to_string());
                    let record = indexing_options
                        .map(|opt| IndexRecordOption::from(opt.index_option()).to_string());
                    let normalizer = text_options
                        .get_fast_field_tokenizer_name()
                        .map(|s| s.to_string());
                    ("Str".to_string(), tokenizer, record, normalizer, None)
                }
                FieldType::JsonObject(json_options) => {
                    let indexing_options = json_options.get_text_indexing_options();
                    let tokenizer = indexing_options.map(|opt| opt.tokenizer().to_string());
                    let record = indexing_options
                        .map(|opt| IndexRecordOption::from(opt.index_option()).to_string());
                    let normalizer = json_options
                        .get_fast_field_tokenizer_name()
                        .map(|s| s.to_string());
                    let expand_dots = Some(json_options.is_expand_dots_enabled());
                    (
                        "JsonObject".to_string(),
                        tokenizer,
                        record,
                        normalizer,
                        expand_dots,
                    )
                }
                FieldType::Date(_) => ("Date".to_string(), None, None, None, None),
                _ => ("Other".to_string(), None, None, None, None),
            };

        let row = (
            field_entry.name().to_string(),
            field_type,
            field_entry.is_stored(),
            field_entry.is_indexed(),
            field_entry.is_fast(),
            field_entry.has_fieldnorms(),
            expand_dots,
            tokenizer,
            record,
            normalizer,
        );

        field_rows.push(row);
    }

    // Sort field rows for consistent ordering
    field_rows.sort_by_key(|(name, _, _, _, _, _, _, _, _, _)| name.clone());
    TableIterator::new(field_rows)
}

#[pg_extern]
pub unsafe fn index_fields(index: PgRelation) -> anyhow::Result<JsonB> {
    let index = PgSearchRelation::with_lock(index.oid(), pg_sys::AccessShareLock as _);
    let schema = index.schema()?;

    let mut name_and_config = HashMap::default();
    for (_, field_entry) in schema.fields() {
        let field_name = field_entry.name();
        let field_config = index
            .options()
            .field_config_or_default(&FieldName::from(field_name));
        name_and_config.insert(field_name, field_config);
    }

    Ok(JsonB(serde_json::to_value(name_and_config)?))
}

#[pg_extern]
pub unsafe fn layer_sizes(index: PgRelation) -> Vec<AnyNumeric> {
    let index = PgSearchRelation::with_lock(index.oid(), pg_sys::AccessShareLock as _);
    index
        .options()
        .foreground_layer_sizes()
        .into_iter()
        .map(|layer_size| layer_size.into())
        .collect()
}

#[pg_extern]
pub unsafe fn background_layer_sizes(index: PgRelation) -> Vec<AnyNumeric> {
    let index = PgSearchRelation::with_lock(index.oid(), pg_sys::AccessShareLock as _);
    index
        .options()
        .background_layer_sizes()
        .into_iter()
        .map(|layer_size| layer_size.into())
        .collect()
}

#[pg_extern]
pub unsafe fn combined_layer_sizes(index: PgRelation) -> Vec<AnyNumeric> {
    let index = PgSearchRelation::with_lock(index.oid(), pg_sys::AccessShareLock as _);
    let mut sizes: Vec<_> = index
        .options()
        .foreground_layer_sizes()
        .into_iter()
        .chain(index.options().background_layer_sizes())
        .map(|layer_size| layer_size.into())
        .collect();

    sizes.sort_unstable();
    sizes.dedup();
    sizes.into_iter().collect()
}

#[pg_extern]
unsafe fn merge_info(
    index: PgRelation,
) -> TableIterator<
    'static,
    (
        name!(index_name, String),
        name!(pid, i32),
        name!(xmin, pg_sys::TransactionId),
        name!(segno, String),
    ),
> {
    let index = PgSearchRelation::with_lock(index.oid(), pg_sys::AccessShareLock as _);
    let index_kind = IndexKind::for_index(index).unwrap();

    let mut result = Vec::new();
    for index in index_kind.partitions() {
        let metadata = MetaPage::open(&index);
        let merge_lock = metadata.acquire_merge_lock();
        let merge_entries = merge_lock.merge_list().list();
        result.extend(merge_entries.into_iter().flat_map(move |merge_entry| {
            let index_name = index.name().to_owned();
            merge_entry
                .segment_ids(&index)
                .into_iter()
                .map(move |segment_id| {
                    (
                        index_name.clone(),
                        merge_entry.pid,
                        merge_entry.xmin,
                        segment_id.short_uuid_string(),
                    )
                })
        }));
    }
    TableIterator::new(result)
}

/// Deprecated: Use `paradedb.merge_info` instead.
#[pg_extern]
fn is_merging(index: PgRelation) -> bool {
    unsafe { merge_info(index).next().is_some() }
}

#[pg_extern]
unsafe fn vacuum_info(
    index: PgRelation,
) -> TableIterator<'static, (name!(index_name, String), name!(segno, String))> {
    let index = PgSearchRelation::with_lock(index.oid(), pg_sys::AccessShareLock as _);
    let index_kind = IndexKind::for_index(index).unwrap();

    let mut result = Vec::new();
    for index in index_kind.partitions() {
        let metadata = MetaPage::open(&index);
        let vacuum_list = metadata.vacuum_list().read_list();
        result.extend(
            vacuum_list
                .iter()
                .map(|segment_id| (index.name().to_owned(), segment_id.short_uuid_string())),
        );
    }
    TableIterator::new(result)
}

#[allow(clippy::type_complexity)]
#[pg_extern]
fn index_info(
    index: PgRelation,
    show_invisible: default!(bool, false),
) -> anyhow::Result<
    TableIterator<
        'static,
        (
            name!(index_name, String),
            name!(visible, bool),
            name!(recyclable, bool),
            name!(xmax, pg_sys::TransactionId),
            name!(segno, String),
            name!(mutable, bool),
            name!(byte_size, Option<AnyNumeric>),
            name!(num_docs, Option<AnyNumeric>),
            name!(num_deleted, Option<AnyNumeric>),
            name!(termdict_bytes, Option<AnyNumeric>),
            name!(postings_bytes, Option<AnyNumeric>),
            name!(positions_bytes, Option<AnyNumeric>),
            name!(fast_fields_bytes, Option<AnyNumeric>),
            name!(fieldnorms_bytes, Option<AnyNumeric>),
            name!(store_bytes, Option<AnyNumeric>),
            name!(deletes_bytes, Option<AnyNumeric>),
        ),
    >,
> {
    // # Safety
    //
    // Lock the index relation until the end of this function so it is not dropped or
    // altered while we are reading it.
    //
    // Because we accept a PgRelation above, we have confidence that Postgres has already
    // validated the existence of the relation. We are safe calling the function below as
    // long we do not pass pg_sys::NoLock without any other locking mechanism of our own.
    let index = PgSearchRelation::with_lock(index.oid(), pg_sys::AccessShareLock as _);
    let index_kind = IndexKind::for_index(index)?;

    let mut results = Vec::new();
    for index in index_kind.partitions() {
        // open the specified index
        let mut segment_components = MetaPage::open(&index).segment_metas();
        let all_entries = unsafe { segment_components.list(None) };

        for entry in all_entries {
            if !show_invisible && unsafe { !entry.visible() } {
                continue;
            }
            match entry.content {
                SegmentMetaEntryContent::Immutable(content) => {
                    results.push((
                        index.name().to_owned(),
                        unsafe { entry.visible() },
                        unsafe { entry.recyclable(segment_components.bman_mut()) },
                        entry.xmax(),
                        entry.segment_id().short_uuid_string(),
                        false,
                        Some(entry.byte_size().into()),
                        Some(entry.num_docs().into()),
                        Some(entry.num_deleted_docs().into()),
                        content.terms.map(|file| file.total_bytes.into()),
                        content.postings.map(|file| file.total_bytes.into()),
                        content.positions.map(|file| file.total_bytes.into()),
                        content.fast_fields.map(|file| file.total_bytes.into()),
                        content.field_norms.map(|file| file.total_bytes.into()),
                        content.store.map(|file| file.total_bytes.into()),
                        content
                            .delete
                            .map(|file| file.file_entry.total_bytes.into()),
                    ));
                }
                SegmentMetaEntryContent::Mutable(_) => {
                    results.push((
                        index.name().to_owned(),
                        unsafe { entry.visible() },
                        unsafe { entry.recyclable(segment_components.bman_mut()) },
                        entry.xmax(),
                        entry.segment_id().short_uuid_string(),
                        true,
                        None,
                        Some(entry.num_docs().into()),
                        Some(entry.num_deleted_docs().into()),
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                        None,
                    ));
                }
            }
        }
    }

    Ok(TableIterator::new(results))
}

/// Returns the list of segments that contain the specified [`pg_sys::ItemPointerData]` heap tuple
/// identifier.
///
/// If the specified `ctid` is the result of a HOT chain update, then it's likely this function will
/// return NULL -- HOT chains cannot be "reverse searched".
///
/// Otherwise, if this function returns NULL that is likely indicative of a pg_search bug.  We wish
/// you luck in determining which is which.
#[pg_extern]
fn find_ctid(index: PgRelation, ctid: pg_sys::ItemPointerData) -> Result<Option<Vec<String>>> {
    let index = PgSearchRelation::with_lock(index.oid(), pg_sys::AccessShareLock as _);
    let ctid_u64 = item_pointer_to_u64(ctid);
    let query = SearchQueryInput::FieldedQuery {
        field: "ctid".into(),
        query: pdb::Query::Term {
            value: ctid_u64.into(),
            is_datetime: false,
        },
    };
    let search_index = SearchIndexReader::open(&index, query, false, MvccSatisfies::Snapshot)?;
    let results = search_index.search();

    let results = results
        .map(|(_, doc_address)| {
            search_index.segment_readers()[doc_address.segment_ord as usize]
                .segment_id()
                .short_uuid_string()
        })
        .collect::<Vec<_>>();

    if results.is_empty() {
        panic!(
            "find_ctid: didn't find segment for: {:?}.  segments={:#?}",
            pgrx::itemptr::item_pointer_get_both(ctid),
            search_index.segment_ids()
        );
    } else {
        Ok(Some(results))
    }
}

#[pg_extern]
fn validate_checksum(index: PgRelation) -> Result<SetOfIterator<'static, String>> {
    // # Safety
    //
    // Lock the index relation until the end of this function so it is not dropped or
    // altered while we are reading it.
    //
    // Because we accept a PgRelation above, we have confidence that Postgres has already
    // validated the existence of the relation. We are safe calling the function below as
    // long we do not pass pg_sys::NoLock without any other locking mechanism of our own.
    let index = PgSearchRelation::with_lock(index.oid(), pg_sys::AccessShareLock as _);

    // open the specified index
    let search_reader = SearchIndexReader::empty(&index, MvccSatisfies::Snapshot)?;

    let failed = search_reader.validate_checksum()?;
    Ok(SetOfIterator::new(
        failed.into_iter().map(|path| path.display().to_string()),
    ))
}

/// Verify the integrity of a BM25 index, similar to PostgreSQL's amcheck extension.
///
/// This function performs various verification checks on the index:
/// - Segment checksum validation (using Tantivy's built-in checksums)
/// - Segment metadata consistency
/// - Schema validation
/// - Optionally, heap reference validation (when `heapallindexed` is true)
///
/// # Arguments
/// * `index` - The BM25 index to verify
/// * `heapallindexed` - If true, verify that all indexed ctids exist in the heap table
/// * `sample_rate` - For large indexes, check only this fraction of documents (0.0-1.0, default 1.0 = 100%)
/// * `report_progress` - If true, emit progress messages via WARNING for long-running checks
/// * `segment_ids` - Optional array of segment indices to verify (0-based). If NULL, verify all segments.
///                   Use this for manual parallelization across multiple database connections.
///
/// # Returns
/// A table with columns:
/// - `check_name`: Name of the verification check
/// - `passed`: Whether the check passed
/// - `details`: Additional details about the check result
///
/// # Example
/// ```sql
/// -- Basic verification
/// SELECT * FROM paradedb.verify_bm25_index('my_index');
///
/// -- With heap reference validation
/// SELECT * FROM paradedb.verify_bm25_index('my_index', heapallindexed := true);
///
/// -- For large indexes: sample 10% of documents with progress reporting
/// SELECT * FROM paradedb.verify_bm25_index('my_index', heapallindexed := true, sample_rate := 0.1, report_progress := true);
///
/// -- Verbose mode: show segment list and resume hints (useful for resuming after connection drop)
/// SELECT * FROM paradedb.verify_bm25_index('my_index', heapallindexed := true, report_progress := true, verbose := true);
///
/// -- Manual parallelization: run these queries from separate database connections
/// -- First, get the segment count:
/// --   SELECT COUNT(*) FROM paradedb.index_info('my_index');
/// -- Then split verification across connections:
/// -- Connection 1: SELECT * FROM paradedb.verify_bm25_index('my_index', heapallindexed := true, segment_ids := ARRAY[0,1,2]);
/// -- Connection 2: SELECT * FROM paradedb.verify_bm25_index('my_index', heapallindexed := true, segment_ids := ARRAY[3,4,5]);
/// ```
#[allow(clippy::type_complexity)]
#[pg_extern]
fn verify_bm25_index(
    index: PgRelation,
    heapallindexed: default!(bool, false),
    sample_rate: default!(f64, 1.0),
    report_progress: default!(bool, false),
    verbose: default!(bool, false),
    segment_ids: default!(Option<Vec<i32>>, "NULL"),
) -> Result<
    TableIterator<
        'static,
        (
            name!(check_name, String),
            name!(passed, bool),
            name!(details, Option<String>),
        ),
    >,
> {
    // Validate sample_rate
    let sample_rate = sample_rate.clamp(0.0, 1.0);

    // Convert segment_ids to a HashSet for O(1) lookup
    let segment_filter: Option<HashSet<usize>> = segment_ids.map(|ids| {
        ids.into_iter()
            .filter(|&id| id >= 0)
            .map(|id| id as usize)
            .collect()
    });

    let index_rel = PgSearchRelation::with_lock(index.oid(), pg_sys::AccessShareLock as _);
    let index_kind = IndexKind::for_index(index_rel.clone())?;

    let mut results = Vec::new();

    // We process each partition of the index
    for partition in index_kind.partitions() {
        let partition_name = partition.name().to_owned();

        if report_progress {
            pgrx::warning!(
                "verify_bm25_index: Starting verification of {}",
                partition_name
            );
        }

        // Check 1: Schema validation
        let schema_result = partition.schema();
        match &schema_result {
            Ok(_schema) => {
                results.push((
                    format!("{}: schema_valid", partition_name),
                    true,
                    Some("Index schema loaded successfully".to_string()),
                ));
            }
            Err(e) => {
                results.push((
                    format!("{}: schema_valid", partition_name),
                    false,
                    Some(format!("Failed to load index schema: {}", e)),
                ));
                // If schema fails, we can't do much more for this partition
                continue;
            }
        }

        // Check 2: Open index reader
        let reader_result = SearchIndexReader::empty(&partition, MvccSatisfies::Snapshot);
        let search_reader = match reader_result {
            Ok(reader) => {
                results.push((
                    format!("{}: index_readable", partition_name),
                    true,
                    Some("Index reader opened successfully".to_string()),
                ));
                reader
            }
            Err(e) => {
                results.push((
                    format!("{}: index_readable", partition_name),
                    false,
                    Some(format!("Failed to open index reader: {}", e)),
                ));
                continue;
            }
        };

        // Check 3: Segment checksum validation
        if report_progress {
            pgrx::warning!(
                "verify_bm25_index: Validating checksums for {}",
                partition_name
            );
        }
        let checksum_result = search_reader.validate_checksum();
        match checksum_result {
            Ok(failed_checksums) => {
                if failed_checksums.is_empty() {
                    results.push((
                        format!("{}: checksums_valid", partition_name),
                        true,
                        Some("All segment checksums validated successfully".to_string()),
                    ));
                } else {
                    let failed_files: Vec<_> = failed_checksums
                        .iter()
                        .map(|p| p.display().to_string())
                        .collect();
                    results.push((
                        format!("{}: checksums_valid", partition_name),
                        false,
                        Some(format!(
                            "Checksum validation failed for {} files: {}",
                            failed_files.len(),
                            failed_files.join(", ")
                        )),
                    ));
                }
            }
            Err(e) => {
                results.push((
                    format!("{}: checksums_valid", partition_name),
                    false,
                    Some(format!("Checksum validation error: {}", e)),
                ));
            }
        }

        // Check 4: Segment metadata consistency
        let segment_readers = search_reader.segment_readers();
        let num_segments = segment_readers.len();
        let mut segment_issues = Vec::new();
        let mut segments_checked = 0usize;

        // Build list of all segments with their IDs for progress reporting
        let all_segments: Vec<(usize, String)> = segment_readers
            .iter()
            .enumerate()
            .map(|(idx, r)| (idx, r.segment_id().short_uuid_string()))
            .collect();

        // Determine which segments will be processed
        let segments_to_process: Vec<(usize, String)> = all_segments
            .iter()
            .filter(|(idx, _)| {
                segment_filter
                    .as_ref()
                    .is_none_or(|filter| filter.contains(idx))
            })
            .cloned()
            .collect();

        // Log segment list at the start for resumability (only in verbose mode)
        if verbose {
            let all_segment_list: Vec<String> = all_segments
                .iter()
                .map(|(idx, id)| format!("{}:{}", idx, id))
                .collect();
            pgrx::warning!(
                "verify_bm25_index: {} has {} segments: [{}]",
                partition_name,
                num_segments,
                all_segment_list.join(", ")
            );

            if segment_filter.is_some() {
                let to_process_list: Vec<String> = segments_to_process
                    .iter()
                    .map(|(idx, id)| format!("{}:{}", idx, id))
                    .collect();
                pgrx::warning!(
                    "verify_bm25_index: Will process {} of {} segments: [{}]",
                    segments_to_process.len(),
                    num_segments,
                    to_process_list.join(", ")
                );

                // Log which segments to use for resuming if connection drops
                let remaining_indices: Vec<String> = segments_to_process
                    .iter()
                    .map(|(idx, _)| idx.to_string())
                    .collect();
                pgrx::warning!(
                    "verify_bm25_index: To resume from start, use: segment_ids := ARRAY[{}]",
                    remaining_indices.join(", ")
                );
            }
        } else if report_progress {
            // Basic progress: just show segment count
            if segment_filter.is_some() {
                pgrx::warning!(
                    "verify_bm25_index: Verifying {} of {} segments",
                    segments_to_process.len(),
                    num_segments
                );
            } else {
                pgrx::warning!("verify_bm25_index: Verifying {} segments", num_segments);
            }
        }

        for (idx, segment_reader) in segment_readers.iter().enumerate() {
            // Skip segments not in the filter (if filter is specified)
            if let Some(ref filter) = segment_filter {
                if !filter.contains(&idx) {
                    continue;
                }
            }

            segments_checked += 1;
            let segment_id = segment_reader.segment_id().short_uuid_string();
            let num_docs = segment_reader.num_docs();
            let max_doc = segment_reader.max_doc();

            // Log progress for each segment
            if verbose {
                pgrx::warning!(
                    "verify_bm25_index: Processing segment {}/{} (index={}, id={}, docs={})",
                    segments_checked,
                    segments_to_process.len(),
                    idx,
                    segment_id,
                    num_docs
                );
            }

            // Basic sanity check: num_docs should not exceed max_doc
            if num_docs > max_doc {
                segment_issues.push(format!(
                    "Segment {} ({}): num_docs ({}) exceeds max_doc ({})",
                    idx, segment_id, num_docs, max_doc
                ));
            }

            // Check if fast fields are accessible (specifically ctid)
            let fast_fields = segment_reader.fast_fields();
            if fast_fields.u64("ctid").is_err() {
                segment_issues.push(format!(
                    "Segment {} ({}): ctid fast field not accessible",
                    idx, segment_id
                ));
            }

            // Log completion and resume hint after each segment (verbose mode only)
            if verbose {
                let remaining: Vec<String> = segments_to_process
                    .iter()
                    .filter(|(i, _)| *i > idx)
                    .map(|(i, _)| i.to_string())
                    .collect();
                if remaining.is_empty() {
                    pgrx::warning!(
                        "verify_bm25_index: Completed segment {} (index={}, id={}). All segments done.",
                        segments_checked,
                        idx,
                        segment_id
                    );
                } else {
                    pgrx::warning!(
                        "verify_bm25_index: Completed segment {} (index={}, id={}). To resume: segment_ids := ARRAY[{}]",
                        segments_checked,
                        idx,
                        segment_id,
                        remaining.join(", ")
                    );
                }
            }
        }

        let segment_info = if segment_filter.is_some() {
            format!(
                "{} of {} segments validated successfully",
                segments_checked, num_segments
            )
        } else {
            format!("{} segments validated successfully", num_segments)
        };

        if segment_issues.is_empty() {
            results.push((
                format!("{}: segment_metadata_valid", partition_name),
                true,
                Some(segment_info),
            ));
        } else {
            results.push((
                format!("{}: segment_metadata_valid", partition_name),
                false,
                Some(segment_issues.join("; ")),
            ));
        }

        // Check 5: Heap reference validation (optional, expensive)
        if heapallindexed {
            if report_progress {
                let segment_info = if segment_filter.is_some() {
                    format!(" for {} selected segments", segments_checked)
                } else {
                    String::new()
                };
                pgrx::warning!(
                    "verify_bm25_index: Starting heap reference check for {} (sample_rate: {:.0}%){}",
                    partition_name,
                    sample_rate * 100.0,
                    segment_info
                );
            }
            let heap_check_result = verify_heap_references(
                &partition,
                &search_reader,
                sample_rate,
                report_progress,
                verbose,
                &segment_filter,
            );
            match heap_check_result {
                Ok((total_checked, total_docs, missing_ctids)) => {
                    let sample_info = if sample_rate < 1.0 {
                        format!(" (sampled {} of {} total docs)", total_checked, total_docs)
                    } else {
                        String::new()
                    };

                    if missing_ctids.is_empty() {
                        results.push((
                            format!("{}: heap_references_valid", partition_name),
                            true,
                            Some(format!(
                                "All {} indexed ctids exist in heap{}",
                                total_checked, sample_info
                            )),
                        ));
                    } else {
                        // Limit the number of reported missing ctids
                        let sample: Vec<_> = missing_ctids
                            .iter()
                            .take(10)
                            .map(|ctid| format!("{:?}", ctid))
                            .collect();
                        let more = if missing_ctids.len() > 10 {
                            format!(" (and {} more)", missing_ctids.len() - 10)
                        } else {
                            String::new()
                        };
                        results.push((
                            format!("{}: heap_references_valid", partition_name),
                            false,
                            Some(format!(
                                "{} of {} indexed ctids missing from heap{}: {}{}",
                                missing_ctids.len(),
                                total_checked,
                                sample_info,
                                sample.join(", "),
                                more
                            )),
                        ));
                    }
                }
                Err(e) => {
                    results.push((
                        format!("{}: heap_references_valid", partition_name),
                        false,
                        Some(format!("Heap reference check failed: {}", e)),
                    ));
                }
            }
        }

        if report_progress {
            pgrx::warning!(
                "verify_bm25_index: Completed verification of {}",
                partition_name
            );
        }
    }

    Ok(TableIterator::new(results))
}

/// List all segments in a BM25 index.
///
/// Returns information about each segment, useful for:
/// - Understanding index structure
/// - Planning parallel verification with `verify_bm25_index(..., segment_ids := ...)`
/// - Automating multi-client index checks
///
/// # Example
/// ```sql
/// -- List all segments
/// SELECT * FROM paradedb.bm25_index_segments('my_index');
///
/// -- Automate parallel verification: split segments across N workers
/// -- Worker 1:
/// SELECT * FROM paradedb.verify_bm25_index('my_index',
///     heapallindexed := true,
///     segment_ids := (SELECT array_agg(segment_idx) FROM paradedb.bm25_index_segments('my_index')
///                     WHERE segment_idx % 2 = 0));
/// -- Worker 2:
/// SELECT * FROM paradedb.verify_bm25_index('my_index',
///     heapallindexed := true,
///     segment_ids := (SELECT array_agg(segment_idx) FROM paradedb.bm25_index_segments('my_index')
///                     WHERE segment_idx % 2 = 1));
/// ```
#[allow(clippy::type_complexity)]
#[pg_extern]
fn bm25_index_segments(
    index: PgRelation,
) -> Result<
    TableIterator<
        'static,
        (
            name!(partition_name, String),
            name!(segment_idx, i32),
            name!(segment_id, String),
            name!(num_docs, i64),
            name!(num_deleted, i64),
            name!(max_doc, i64),
        ),
    >,
> {
    let index_rel = PgSearchRelation::with_lock(index.oid(), pg_sys::AccessShareLock as _);
    let index_kind = IndexKind::for_index(index_rel.clone())?;

    let mut results = Vec::new();

    for partition in index_kind.partitions() {
        let partition_name = partition.name().to_owned();

        // Try to open the index reader
        let search_reader = match SearchIndexReader::empty(&partition, MvccSatisfies::Snapshot) {
            Ok(reader) => reader,
            Err(_) => continue,
        };

        // Collect segment information
        for (idx, segment_reader) in search_reader.segment_readers().iter().enumerate() {
            let segment_id = segment_reader.segment_id().short_uuid_string();
            let num_docs = segment_reader.num_docs() as i64;
            let max_doc = segment_reader.max_doc() as i64;
            let num_deleted = segment_reader.num_deleted_docs() as i64;

            results.push((
                partition_name.clone(),
                idx as i32,
                segment_id,
                num_docs,
                num_deleted,
                max_doc,
            ));
        }
    }

    Ok(TableIterator::new(results))
}

type HeapCheckResult = Result<(usize, usize, Vec<(u32, u16)>)>;

/// Helper function to verify that all indexed ctids exist in the heap.
/// Returns (total_checked, total_docs, missing_ctids)
fn verify_heap_references(
    index_rel: &PgSearchRelation,
    search_reader: &SearchIndexReader,
    sample_rate: f64,
    report_progress: bool,
    verbose: bool,
    segment_filter: &Option<HashSet<usize>>,
) -> HeapCheckResult {
    use crate::index::fast_fields_helper::FFType;
    use crate::postgres::utils::u64_to_item_pointer;

    // Get the heap relation OID from the index
    let heap_oid = index_rel
        .rel_oid()
        .ok_or_else(|| anyhow::anyhow!("Could not determine heap relation for index"))?;

    // Open the heap relation
    let heap_rel = PgSearchRelation::with_lock(heap_oid, pg_sys::AccessShareLock as _);

    // Set up heap fetch state
    let scan = unsafe { pg_sys::table_index_fetch_begin(heap_rel.as_ptr()) };
    let slot = unsafe {
        pg_sys::MakeTupleTableSlot(
            pg_sys::CreateTupleDesc(0, std::ptr::null_mut()),
            &pg_sys::TTSOpsBufferHeapTuple,
        )
    };
    let snapshot = unsafe { pg_sys::GetActiveSnapshot() };

    let mut total_checked = 0usize;
    let mut total_docs = 0usize;
    let mut missing_ctids = Vec::new();

    // For sampling, we use a simple deterministic approach based on doc_id
    // This ensures reproducible results for the same sample_rate
    let sample_threshold = (sample_rate * u32::MAX as f64) as u32;

    // Calculate total docs for progress reporting (only for segments we'll check)
    let total_alive_docs: usize = search_reader
        .segment_readers()
        .iter()
        .enumerate()
        .filter(|(idx, _)| {
            segment_filter
                .as_ref()
                .is_none_or(|filter| filter.contains(idx))
        })
        .map(|(_, r)| r.num_docs() as usize)
        .sum();

    // Progress reporting interval (report every ~5% or every 100k docs, whichever is larger)
    let progress_interval = (total_alive_docs / 20).max(100_000);
    let mut last_progress_report = 0usize;

    // Build list of segments to process for progress logging
    let segments_to_check: Vec<usize> = search_reader
        .segment_readers()
        .iter()
        .enumerate()
        .filter(|(idx, _)| {
            segment_filter
                .as_ref()
                .is_none_or(|filter| filter.contains(idx))
        })
        .map(|(idx, _)| idx)
        .collect();
    let mut segments_completed = 0usize;

    // Iterate through all segments and all documents
    for (seg_idx, segment_reader) in search_reader.segment_readers().iter().enumerate() {
        // Skip segments not in the filter (if filter is specified)
        if let Some(ref filter) = segment_filter {
            if !filter.contains(&seg_idx) {
                continue;
            }
        }

        let segment_id = segment_reader.segment_id().short_uuid_string();
        let fast_fields = segment_reader.fast_fields();
        let ctid_column = FFType::new_ctid(fast_fields);
        let alive_bitset = segment_reader.alive_bitset();

        if verbose {
            pgrx::warning!(
                "verify_bm25_index: Heap check - starting segment {}/{} (index={}, id={}, {} docs)",
                segments_completed + 1,
                segments_to_check.len(),
                seg_idx,
                segment_id,
                segment_reader.num_docs()
            );
        }

        // Iterate through all document IDs in this segment
        for doc_id in 0..segment_reader.max_doc() {
            // Skip deleted documents
            if let Some(bitset) = &alive_bitset {
                if !bitset.is_alive(doc_id) {
                    continue;
                }
            }

            total_docs += 1;

            // Apply sampling: use a hash of the doc_id for deterministic sampling
            if sample_rate < 1.0 {
                // Simple hash: multiply by a prime and take modulo
                let hash = doc_id.wrapping_mul(2654435761);
                if hash > sample_threshold {
                    continue;
                }
            }

            // Get the ctid for this document
            let ctid_u64 = match ctid_column.as_u64(doc_id) {
                Some(val) => val,
                None => continue, // Skip if ctid is not available
            };

            total_checked += 1;

            // Report progress periodically
            if report_progress && total_checked - last_progress_report >= progress_interval {
                let pct = (total_docs as f64 / total_alive_docs as f64 * 100.0).min(100.0);
                pgrx::warning!(
                    "verify_bm25_index: Progress {:.1}% ({} docs checked, {} missing so far)",
                    pct,
                    total_checked,
                    missing_ctids.len()
                );
                last_progress_report = total_checked;
            }

            // Convert u64 to ItemPointerData
            let mut tid = pg_sys::ItemPointerData::default();
            u64_to_item_pointer(ctid_u64, &mut tid);

            // Check if the tuple exists in the heap
            let mut call_again = false;
            let mut all_dead = false;
            let found = unsafe {
                pg_sys::ExecClearTuple(slot);
                pg_sys::table_index_fetch_tuple(
                    scan,
                    &mut tid,
                    snapshot,
                    slot,
                    &mut call_again,
                    &mut all_dead,
                )
            };

            if !found {
                let (block, offset) = pgrx::itemptr::item_pointer_get_both(tid);
                missing_ctids.push((block, offset));
            }

            // Check for interrupts periodically (every 10k docs)
            if total_checked.is_multiple_of(10_000) {
                pgrx::check_for_interrupts!();
            }
        }

        // Log segment completion with resume hint (verbose mode only)
        segments_completed += 1;
        if verbose {
            let remaining: Vec<String> = segments_to_check
                .iter()
                .filter(|&&i| i > seg_idx)
                .map(|i| i.to_string())
                .collect();
            if remaining.is_empty() {
                pgrx::warning!(
                    "verify_bm25_index: Heap check - completed segment {}/{} (index={}, id={}). All segments done.",
                    segments_completed,
                    segments_to_check.len(),
                    seg_idx,
                    segment_id
                );
            } else {
                pgrx::warning!(
                    "verify_bm25_index: Heap check - completed segment {}/{} (index={}, id={}). To resume: segment_ids := ARRAY[{}]",
                    segments_completed,
                    segments_to_check.len(),
                    seg_idx,
                    segment_id,
                    remaining.join(", ")
                );
            }
        }
    }

    // Clean up
    unsafe {
        pg_sys::ExecDropSingleTupleTableSlot(slot);
        pg_sys::table_index_fetch_end(scan);
    }

    if report_progress {
        pgrx::warning!(
            "verify_bm25_index: Heap check complete. Checked {} of {} docs, {} missing.",
            total_checked,
            total_docs,
            missing_ctids.len()
        );
    }

    Ok((total_checked, total_docs, missing_ctids))
}

#[pg_extern(sql = "")]
fn create_bm25_jsonb() {}

#[allow(clippy::type_complexity)]
#[pg_extern]
fn storage_info(
    index: PgRelation,
) -> TableIterator<'static, (name!(block, i64), name!(max_offset, i32))> {
    let index = PgSearchRelation::with_lock(index.oid(), pg_sys::AccessShareLock as _);
    let segment_components = MetaPage::open(&index).segment_metas();
    let bman = segment_components.bman();
    let (mut blockno, mut buffer) = segment_components.get_start_blockno();
    let mut data = vec![];

    while blockno != pg_sys::InvalidBlockNumber {
        buffer = bman.get_buffer_exchange(blockno, buffer);
        let page = buffer.page();
        let max_offset = page.max_offset_number();
        data.push((blockno as i64, max_offset as i32));
        blockno = page.next_blockno();
    }

    TableIterator::new(data)
}

#[allow(clippy::type_complexity)]
#[pg_extern]
fn page_info(
    index: PgRelation,
    blockno: i64,
) -> anyhow::Result<
    TableIterator<
        'static,
        (
            name!(offsetno, i32),
            name!(size, i32),
            name!(visible, bool),
            name!(recyclable, bool),
            name!(contents, JsonB),
        ),
    >,
> {
    let index = PgSearchRelation::with_lock(index.oid(), pg_sys::AccessShareLock as _);
    let mut segment_components = MetaPage::open(&index).segment_metas();
    let bman = segment_components.bman_mut();
    let buffer = bman.get_buffer(blockno as pg_sys::BlockNumber);
    let page = buffer.page();
    let max_offset = page.max_offset_number();

    if max_offset == pg_sys::InvalidOffsetNumber {
        return Ok(TableIterator::new(vec![]));
    }

    let mut data = vec![];

    for offsetno in pg_sys::FirstOffsetNumber..=max_offset {
        unsafe {
            if let Some((entry, size)) = page.deserialize_item::<SegmentMetaEntry>(offsetno) {
                data.push((
                    offsetno as i32,
                    size as i32,
                    entry.visible(),
                    entry.recyclable(bman),
                    JsonB(serde_json::to_value(entry)?),
                ))
            } else {
                data.push((offsetno as i32, 0_i32, false, false, JsonB(Value::Null)))
            }
        }
    }

    Ok(TableIterator::new(data))
}

#[pg_extern]
fn version_info() -> TableIterator<
    'static,
    (
        name!(version, String),
        name!(githash, String),
        name!(build_mode, String),
    ),
> {
    let version = option_env!("CARGO_PKG_VERSION")
        .unwrap_or("unknown")
        .to_string();

    let git_sha = option_env!("VERGEN_GIT_SHA")
        .unwrap_or("unknown")
        .to_string();

    let build_mode = if cfg!(debug_assertions) {
        "debug".to_string()
    } else {
        "release".to_string()
    };

    TableIterator::once((version, git_sha, build_mode))
}

#[pg_extern(name = "force_merge")]
fn force_merge_pretty_bytes(
    _index: PgRelation,
    _oversized_layer_size_pretty: String,
) -> anyhow::Result<TableIterator<'static, (name!(new_segments, i64), name!(merged_segments, i64))>>
{
    anyhow::bail!("force_merge is deprecated, run `VACUUM` instead");
}

#[pg_extern(name = "force_merge")]
fn force_merge_raw_bytes(
    _index: PgRelation,
    _oversized_layer_size_bytes: i64,
) -> anyhow::Result<TableIterator<'static, (name!(new_segments, i64), name!(merged_segments, i64))>>
{
    anyhow::bail!("force_merge is deprecated, run `VACUUM` instead");
}

#[pg_extern]
fn merge_lock_garbage_collect(index: PgRelation) -> SetOfIterator<'static, i32> {
    unsafe {
        let index = {
            let oid = index.oid();
            drop(index);
            // reopen the index with a RowExclusiveLock b/c we are going to be changing its physical structure
            PgSearchRelation::with_lock(oid, pg_sys::RowExclusiveLock as _)
        };
        let metadata = MetaPage::open(&index);
        let merge_lock = metadata.acquire_merge_lock();
        let mut merge_list = merge_lock.merge_list();
        let before = merge_list.list();
        merge_list.garbage_collect(pg_sys::ReadNextFullTransactionId());
        let after = merge_list.list();
        drop(merge_lock);

        let before_pids = before
            .into_iter()
            .map(|entry| entry.pid)
            .collect::<HashSet<_>>();
        let after_pids = after
            .into_iter()
            .map(|entry| entry.pid)
            .collect::<HashSet<_>>();
        let mut garbage_collected_pids = before_pids
            .difference(&after_pids)
            .copied()
            .collect::<Vec<_>>();
        garbage_collected_pids.sort_unstable();
        SetOfIterator::new(garbage_collected_pids)
    }
}

// Deprecated: Use `pdb.index_layer_info` instead.
extension_sql!(
    r#"create view paradedb.index_layer_info as
select relname::text,
       layer_size,
       low,
       high,
       byte_size,
       case when segments = ARRAY [NULL] then 0 else count end       as count,
       case when segments = ARRAY [NULL] then NULL else segments end as segments
from (select relname,
             coalesce(pg_size_pretty(case when low = 0 then null else low end), '') || '..' ||
             coalesce(pg_size_pretty(case when high = 9223372036854775807 then null else high end), '') as layer_size,
             count(*),
             coalesce(sum(byte_size), 0)                                                                as byte_size,
             min(low)                                                                                   as low,
             max(high)                                                                                  as high,
             array_agg(segno)                                                                           as segments
      from (with indexes as (select oid::regclass as relname
                             from pg_class
                             where relam = (select oid from pg_am where amname = 'bm25')),
                 segments as (select relname, index_info.*
                              from indexes
                                       inner join paradedb.index_info(indexes.relname, true) on true),
                 layer_sizes as (select relname, coalesce(lead(unnest) over (), 0) low, unnest as high
                                 from indexes
                                          inner join lateral (select unnest(0 || paradedb.layer_sizes(indexes.relname) || 9223372036854775807)
                                                              order by 1 desc) x on true)
            select layer_sizes.relname, layer_sizes.low, layer_sizes.high, segments.segno, segments.byte_size
            from layer_sizes
                     left join segments on layer_sizes.relname = segments.relname and
                                           (byte_size * 1.33)::bigint between low and high) x
      where low < high
      group by relname, low, high
      order by relname, low desc) x;

GRANT SELECT ON paradedb.index_layer_info TO PUBLIC;
"#,
    name = "index_layer_info",
    requires = [index_info, layer_sizes]
);

// `pdb.index_layer_info` supersedes `paradedb.index_layer_info`
// It shows both the foreground and background layer sizes, whereas
// `paradedb.index_layer_info` only shows the foreground layer sizes.
extension_sql!(
    r#"create view pdb.index_layer_info as
select relname::text,
       layer_size,
       low,
       high,
       byte_size,
       case when segments = ARRAY [NULL] then 0 else count end       as count,
       case when segments = ARRAY [NULL] then NULL else segments end as segments
from (select relname,
             coalesce(pg_size_pretty(case when low = 0 then null else low end), '') || '..' ||
             coalesce(pg_size_pretty(case when high = 9223372036854775807 then null else high end), '') as layer_size,
             count(*),
             coalesce(sum(byte_size), 0)                                                                as byte_size,
             min(low)                                                                                   as low,
             max(high)                                                                                  as high,
             array_agg(segno)                                                                           as segments
      from (with indexes as (select oid::regclass as relname
                             from pg_class
                             where relam = (select oid from pg_am where amname = 'bm25')),
                 segments as (select relname, index_info.*
                              from indexes
                                       inner join paradedb.index_info(indexes.relname, true) on true),
                 layer_sizes as (select relname, coalesce(lead(unnest) over (), 0) low, unnest as high
                                 from indexes
                                          inner join lateral (select unnest(0 || paradedb.combined_layer_sizes(indexes.relname) || 9223372036854775807)
                                                              order by 1 desc) x on true)
            select layer_sizes.relname, layer_sizes.low, layer_sizes.high, segments.segno, segments.byte_size
            from layer_sizes
                     left join segments on layer_sizes.relname = segments.relname and
                                           (byte_size * 1.33)::bigint between low and high) x
      where low < high
      group by relname, low, high
      order by relname, low desc) x;

GRANT SELECT ON pdb.index_layer_info TO PUBLIC;
"#,
    name = "pdb_index_layer_info",
    requires = [index_info, combined_layer_sizes]
);
