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
use crate::index::fast_fields_helper::FFType;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::{Bm25Settings, SearchIndexReader};
use crate::postgres::index::IndexKind;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::{
    LinkedList, MVCCEntry, SegmentMetaEntry, SegmentMetaEntryContent,
};
use crate::postgres::storage::metadata::MetaPage;
use crate::postgres::utils::{item_pointer_to_u64, u64_to_item_pointer};
use crate::query::pdb_query::pdb as pdb_query;
use crate::query::SearchQueryInput;
use crate::schema::IndexRecordOption;
use anyhow::Result;
use pgrx::datum::DatumWithOid;
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
        query: pdb_query::Query::Term {
            value: ctid_u64.into(),
            is_datetime: false,
        },
    };
    let search_index = SearchIndexReader::open(
        &index,
        query,
        Bm25Settings::default(),
        MvccSatisfies::Snapshot,
    )?;
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

/// Deprecated: Use `pdb.verify_index` instead, which includes checksum validation
/// along with additional integrity checks.
#[pg_extern]
fn validate_checksum(index: PgRelation) -> Result<SetOfIterator<'static, String>> {
    pgrx::warning!(
        "validate_checksum is deprecated. Use pdb.verify_index('{}') instead, \
         which includes checksum validation along with additional integrity checks.",
        index.name()
    );

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

/// Result of heap reference check: (total_checked, total_docs, missing_ctids)
/// - total_checked: number of documents successfully checked
/// - total_docs: number of documents attempted (after sampling)
/// - missing_ctids: list of (block, offset) pairs for ctids not found in heap
///
/// Note: if total_checked < total_docs, then (total_docs - total_checked) documents
/// had missing ctid fields, indicating index corruption.
type HeapCheckResult = Result<(usize, usize, Vec<(u32, u16)>)>;

/// Helper function to verify that all indexed ctids exist in the heap.
/// Returns (total_checked, total_docs, missing_ctids, docs_without_ctid)
fn verify_heap_references(
    index_rel: &PgSearchRelation,
    search_reader: &SearchIndexReader,
    sample_rate: Option<f64>,
    report_progress: bool,
    verbose: bool,
    segment_filter: &Option<HashSet<usize>>,
) -> HeapCheckResult {
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
    // None means check all (no sampling)
    let sample_threshold = sample_rate.map(|r| (r * u32::MAX as f64) as u32);

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
                "verify_index: Heap check - starting segment {}/{} (index={}, id={}, {} docs)",
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

            // Apply sampling: use a hash of the doc_id for deterministic sampling
            // None means check all (no sampling)
            if let Some(threshold) = sample_threshold {
                // Simple hash: multiply by a prime and take modulo
                let hash = doc_id.wrapping_mul(2654435761);
                if hash > threshold {
                    continue;
                }
            }

            // Count documents we're attempting to check (after sampling)
            total_docs += 1;

            // Get the ctid for this document
            // Every indexed document MUST have a ctid - if missing, the index is corrupted
            let ctid_u64 = match ctid_column.as_u64(doc_id) {
                Some(val) => val,
                None => continue, // Will be detected as total_docs > total_checked
            };

            total_checked += 1;

            // Report progress periodically
            if report_progress && total_checked - last_progress_report >= progress_interval {
                let pct = (total_docs as f64 / total_alive_docs as f64 * 100.0).min(100.0);
                pgrx::warning!(
                    "verify_index: Progress {:.1}% ({} docs checked, {} missing so far)",
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
                    "verify_index: Heap check - completed segment {}/{} (index={}, id={}). All segments done.",
                    segments_completed,
                    segments_to_check.len(),
                    seg_idx,
                    segment_id
                );
            } else {
                pgrx::warning!(
                    "verify_index: Heap check - completed segment {}/{} (index={}, id={}). To resume: segment_ids := ARRAY[{}]",
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
        let without_ctid = total_docs - total_checked;
        pgrx::warning!(
            "verify_index: Heap check complete. Checked {} of {} docs, {} missing{}",
            total_checked,
            total_docs,
            missing_ctids.len(),
            if without_ctid > 0 {
                format!(", {} without ctid (corruption)", without_ctid)
            } else {
                String::new()
            }
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

// =============================================================================
// pdb schema functions for index verification
// =============================================================================

#[pgrx::pg_schema]
pub mod pdb {
    use super::*;

    /// Verify the integrity of a BM25 index, similar to PostgreSQL's amcheck extension.
    ///
    /// This function performs various verification checks on the index:
    /// - Schema validation: Ensures the index schema can be loaded
    /// - Index readability: Verifies the index can be opened for reading
    /// - Segment checksum validation: Uses Tantivy's built-in checksums to detect corruption
    /// - Segment metadata consistency: Checks that segment metadata is internally consistent
    /// - Heap reference validation (optional): Verifies all indexed ctids exist in the heap table
    ///
    /// # Arguments
    /// * `index` - The BM25 index to verify (name or OID)
    /// * `heapallindexed` - If true, verify that all indexed ctids exist in the heap table.
    ///   This is expensive but thorough. Default: false
    /// * `sample_rate` - For large indexes, check only this fraction of documents (0.0-1.0).
    ///   Default: NULL (100%). Use lower values for quick spot checks.
    /// * `report_progress` - If true, emit progress messages via WARNING. Default: false
    /// * `verbose` - If true, show detailed segment-by-segment progress and resume hints.
    ///   Useful for resuming after connection drops. Default: false
    /// * `on_error_stop` - If true, stop verification on first error found (like pg_amcheck
    ///   --on-error-stop). Default: false
    /// * `segment_ids` - Optional array of segment indices to verify (0-based). If NULL,
    ///   verify all segments. Use for manual parallelization across database connections.
    ///
    /// # Returns
    /// A table with columns:
    /// - `check_name`: Name of the verification check (e.g., "my_index: schema_valid")
    /// - `passed`: Whether the check passed (true/false)
    /// - `details`: Additional details about the check result
    ///
    /// # Example
    /// ```sql
    /// -- Basic verification
    /// SELECT * FROM pdb.verify_index('my_index');
    ///
    /// -- With heap reference validation (thorough but slower)
    /// SELECT * FROM pdb.verify_index('my_index', heapallindexed := true);
    ///
    /// -- For large indexes: sample 10% of documents with progress reporting
    /// SELECT * FROM pdb.verify_index('my_index',
    ///     heapallindexed := true,
    ///     sample_rate := 0.1,
    ///     report_progress := true);
    ///
    /// -- Verbose mode: show segment list and resume hints
    /// SELECT * FROM pdb.verify_index('my_index',
    ///     heapallindexed := true,
    ///     report_progress := true,
    ///     verbose := true);
    ///
    /// -- Manual parallelization: run from separate database connections
    /// -- First, list all segments:
    /// SELECT * FROM pdb.index_segments('my_index');
    /// -- Then split verification across connections:
    /// -- Connection 1:
    /// SELECT * FROM pdb.verify_index('my_index',
    ///     heapallindexed := true,
    ///     segment_ids := ARRAY[0,1,2]);
    /// -- Connection 2:
    /// SELECT * FROM pdb.verify_index('my_index',
    ///     heapallindexed := true,
    ///     segment_ids := ARRAY[3,4,5]);
    ///
    /// -- Stop on first error (like pg_amcheck --on-error-stop)
    /// SELECT * FROM pdb.verify_index('my_index', on_error_stop := true);
    /// ```
    #[allow(clippy::type_complexity)]
    #[pg_extern]
    pub fn verify_index(
        index: PgRelation,
        heapallindexed: default!(bool, false),
        sample_rate: default!(Option<f64>, "NULL"),
        report_progress: default!(bool, false),
        verbose: default!(bool, false),
        on_error_stop: default!(bool, false),
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
        // Validate sample_rate (None means check all)
        let sample_rate = sample_rate.map(|r| r.clamp(0.0, 1.0));

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
                pgrx::warning!("verify_index: Starting verification of {}", partition_name);
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
                    if on_error_stop {
                        return Ok(TableIterator::new(results));
                    }
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
                    if on_error_stop {
                        return Ok(TableIterator::new(results));
                    }
                    continue;
                }
            };

            // Check 3: Segment checksum validation
            if report_progress {
                pgrx::warning!("verify_index: Validating checksums for {}", partition_name);
            }
            let checksum_result = search_reader.validate_checksum();
            let mut checksum_failed = false;
            match checksum_result {
                Ok(failed_checksums) => {
                    if failed_checksums.is_empty() {
                        results.push((
                            format!("{}: checksums_valid", partition_name),
                            true,
                            Some("All segment checksums validated successfully".to_string()),
                        ));
                    } else {
                        checksum_failed = true;
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
                    checksum_failed = true;
                    results.push((
                        format!("{}: checksums_valid", partition_name),
                        false,
                        Some(format!("Checksum validation error: {}", e)),
                    ));
                }
            }
            if on_error_stop && checksum_failed {
                return Ok(TableIterator::new(results));
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
                    "verify_index: {} has {} segments: [{}]",
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
                        "verify_index: Will process {} of {} segments: [{}]",
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
                        "verify_index: To resume from start, use: segment_ids := ARRAY[{}]",
                        remaining_indices.join(", ")
                    );
                }
            } else if report_progress {
                // Basic progress: just show segment count
                if segment_filter.is_some() {
                    pgrx::warning!(
                        "verify_index: Verifying {} of {} segments",
                        segments_to_process.len(),
                        num_segments
                    );
                } else {
                    pgrx::warning!("verify_index: Verifying {} segments", num_segments);
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
                        "verify_index: Processing segment {}/{} (index={}, id={}, docs={})",
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
                            "verify_index: Completed segment {} (index={}, id={}). All segments done.",
                            segments_checked,
                            idx,
                            segment_id
                        );
                    } else {
                        pgrx::warning!(
                            "verify_index: Completed segment {} (index={}, id={}). To resume: segment_ids := ARRAY[{}]",
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
                if on_error_stop {
                    return Ok(TableIterator::new(results));
                }
            }

            // Check 5: Heap reference validation (optional, expensive)
            if heapallindexed {
                if report_progress {
                    let segment_info = if segment_filter.is_some() {
                        format!(" for {} selected segments", segments_checked)
                    } else {
                        String::new()
                    };
                    let sample_pct = sample_rate.map(|r| r * 100.0).unwrap_or(100.0);
                    pgrx::warning!(
                        "verify_index: Starting heap reference check for {} (sample_rate: {:.0}%){}",
                        partition_name,
                        sample_pct,
                        segment_info
                    );
                }
                let heap_check_result = super::verify_heap_references(
                    &partition,
                    &search_reader,
                    sample_rate,
                    report_progress,
                    verbose,
                    &segment_filter,
                );
                match heap_check_result {
                    Ok((total_checked, total_docs, missing_ctids)) => {
                        let sample_info = if sample_rate.is_some_and(|r| r < 1.0) {
                            format!(" (sampled {} of {} docs)", total_checked, total_docs)
                        } else {
                            String::new()
                        };

                        // Check for documents without ctid (indicates index corruption)
                        // This is detected when total_checked < total_docs
                        let docs_without_ctid = total_docs - total_checked;
                        if docs_without_ctid > 0 {
                            results.push((
                                format!("{}: ctid_field_valid", partition_name),
                                false,
                                Some(format!(
                                    "{} documents missing ctid in index (corruption detected)",
                                    docs_without_ctid
                                )),
                            ));
                            if on_error_stop {
                                return Ok(TableIterator::new(results));
                            }
                        } else {
                            results.push((
                                format!("{}: ctid_field_valid", partition_name),
                                true,
                                Some(format!(
                                    "All {} documents have valid ctid{}",
                                    total_checked, sample_info
                                )),
                            ));
                        }

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
                            if on_error_stop {
                                return Ok(TableIterator::new(results));
                            }
                        }
                    }
                    Err(e) => {
                        results.push((
                            format!("{}: heap_references_valid", partition_name),
                            false,
                            Some(format!("Heap reference check failed: {}", e)),
                        ));
                        if on_error_stop {
                            return Ok(TableIterator::new(results));
                        }
                    }
                }
            }

            if report_progress {
                pgrx::warning!("verify_index: Completed verification of {}", partition_name);
            }
        }

        Ok(TableIterator::new(results))
    }

    /// List all segments in a BM25 index.
    ///
    /// Returns information about each Tantivy segment in the index. This is useful for:
    /// - Understanding index structure and segment distribution
    /// - Planning parallel verification with `pdb.verify_index(..., segment_ids := ...)`
    /// - Automating multi-client index checks
    /// - Monitoring segment merging and index health
    ///
    /// # Arguments
    /// * `index` - The BM25 index to inspect (name or OID)
    ///
    /// # Returns
    /// A table with columns:
    /// - `partition_name`: Name of the index partition
    /// - `segment_idx`: Segment index (0-based, use with `segment_ids` parameter)
    /// - `segment_id`: Tantivy segment UUID (short form)
    /// - `num_docs`: Number of live documents in the segment
    /// - `num_deleted`: Number of deleted (but not yet purged) documents
    /// - `max_doc`: Maximum document ID in the segment
    ///
    /// # Example
    /// ```sql
    /// -- List all segments
    /// SELECT * FROM pdb.index_segments('my_index');
    ///
    /// -- Get segment count
    /// SELECT COUNT(*) FROM pdb.index_segments('my_index');
    ///
    /// -- Find segments with deleted documents
    /// SELECT * FROM pdb.index_segments('my_index') WHERE num_deleted > 0;
    ///
    /// -- Automate parallel verification: split segments across N workers
    /// -- Worker 1 (even segments):
    /// SELECT * FROM pdb.verify_index('my_index',
    ///     heapallindexed := true,
    ///     segment_ids := (SELECT array_agg(segment_idx)
    ///                     FROM pdb.index_segments('my_index')
    ///                     WHERE segment_idx % 2 = 0));
    /// -- Worker 2 (odd segments):
    /// SELECT * FROM pdb.verify_index('my_index',
    ///     heapallindexed := true,
    ///     segment_ids := (SELECT array_agg(segment_idx)
    ///                     FROM pdb.index_segments('my_index')
    ///                     WHERE segment_idx % 2 = 1));
    /// ```
    #[allow(clippy::type_complexity)]
    #[pg_extern]
    pub fn index_segments(
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
            let search_reader = match SearchIndexReader::empty(&partition, MvccSatisfies::Snapshot)
            {
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

    /// List all BM25 indexes in the current database.
    ///
    /// Similar to pg_amcheck's index discovery, this function finds all BM25 indexes
    /// so they can be verified or inspected. Useful for automation and monitoring.
    ///
    /// # Returns
    /// A table with columns:
    /// - `schemaname`: Schema containing the index
    /// - `tablename`: Table the index is on
    /// - `indexname`: Name of the index
    /// - `indexrelid`: OID of the index (can be passed to verify_index)
    /// - `num_segments`: Number of Tantivy segments in the index
    /// - `total_docs`: Total documents across all segments
    ///
    /// # Example
    /// ```sql
    /// -- List all BM25 indexes
    /// SELECT * FROM pdb.indexes();
    ///
    /// -- Find large indexes (by document count)
    /// SELECT * FROM pdb.indexes() ORDER BY total_docs DESC;
    ///
    /// -- Find indexes with many segments (may benefit from optimization)
    /// SELECT * FROM pdb.indexes() WHERE num_segments > 10;
    ///
    /// -- Verify all BM25 indexes in a specific schema
    /// SELECT v.* FROM pdb.indexes() i
    /// CROSS JOIN LATERAL pdb.verify_index(i.indexrelid) v
    /// WHERE i.schemaname = 'public';
    /// ```
    #[allow(clippy::type_complexity)]
    #[pg_extern]
    pub fn indexes() -> Result<
        TableIterator<
            'static,
            (
                name!(schemaname, String),
                name!(tablename, String),
                name!(indexname, String),
                name!(indexrelid, pg_sys::Oid),
                name!(num_segments, i32),
                name!(total_docs, i64),
            ),
        >,
    > {
        let mut results = Vec::new();

        // Query pg_index joined with pg_class to find all BM25 indexes
        let query = r#"
        SELECT
            n.nspname::text AS schemaname,
            t.relname::text AS tablename,
            i.relname::text AS indexname,
            i.oid AS indexrelid
        FROM pg_index idx
        JOIN pg_class i ON idx.indexrelid = i.oid
        JOIN pg_class t ON idx.indrelid = t.oid
        JOIN pg_namespace n ON i.relnamespace = n.oid
        JOIN pg_am am ON i.relam = am.oid
        WHERE am.amname = 'bm25'
        ORDER BY n.nspname, t.relname, i.relname
    "#;

        Spi::connect(|client| {
            let args: [DatumWithOid; 0] = [];
            let result = client.select(query, None, &args)?;

            for row in result {
                let schemaname: String = row.get_by_name("schemaname")?.unwrap_or_default();
                let tablename: String = row.get_by_name("tablename")?.unwrap_or_default();
                let indexname: String = row.get_by_name("indexname")?.unwrap_or_default();
                let indexrelid: pg_sys::Oid =
                    row.get_by_name("indexrelid")?.unwrap_or(pg_sys::InvalidOid);

                // Open the index to get segment information
                let index_rel =
                    PgSearchRelation::with_lock(indexrelid, pg_sys::AccessShareLock as _);
                let index_kind = match IndexKind::for_index(index_rel.clone()) {
                    Ok(k) => k,
                    Err(_) => continue,
                };

                let mut num_segments = 0i32;
                let mut total_docs = 0i64;

                for partition in index_kind.partitions() {
                    if let Ok(search_reader) =
                        SearchIndexReader::empty(&partition, MvccSatisfies::Snapshot)
                    {
                        for segment_reader in search_reader.segment_readers().iter() {
                            num_segments += 1;
                            total_docs += segment_reader.num_docs() as i64;
                        }
                    }
                }

                results.push((
                    schemaname,
                    tablename,
                    indexname,
                    indexrelid,
                    num_segments,
                    total_docs,
                ));
            }

            Ok::<_, pgrx::spi::Error>(())
        })?;

        Ok(TableIterator::new(results))
    }

    /// Verify all BM25 indexes in the current database.
    ///
    /// Similar to pg_amcheck's `--all` option, this function discovers and verifies
    /// all BM25 indexes in the database. Useful for scheduled health checks and
    /// post-upgrade validation.
    ///
    /// # Arguments
    /// * `schema_pattern` - Optional schema pattern to filter indexes (SQL LIKE pattern).
    ///   Example: 'public' or 'app_%'
    /// * `index_pattern` - Optional index name pattern to filter indexes (SQL LIKE pattern).
    ///   Example: 'search_%' or '%_idx'
    /// * `heapallindexed` - If true, verify all indexed ctids exist in the heap.
    ///   Default: false
    /// * `sample_rate` - Fraction of documents to check (0.0-1.0). Default: NULL (100%)
    /// * `report_progress` - Emit progress messages. Default: false
    /// * `on_error_stop` - Stop on first error found. Default: false
    ///
    /// # Returns
    /// A table with columns:
    /// - `schemaname`: Schema containing the index
    /// - `indexname`: Name of the index
    /// - `check_name`: Name of the verification check
    /// - `passed`: Whether the check passed
    /// - `details`: Additional details about the check result
    ///
    /// # Example
    /// ```sql
    /// -- Verify all BM25 indexes
    /// SELECT * FROM pdb.verify_all_indexes();
    ///
    /// -- Verify only indexes in 'public' schema with full heap check
    /// SELECT * FROM pdb.verify_all_indexes(
    ///     schema_pattern := 'public',
    ///     heapallindexed := true);
    ///
    /// -- Quick spot check with 10% sampling
    /// SELECT * FROM pdb.verify_all_indexes(
    ///     sample_rate := 0.1,
    ///     report_progress := true);
    ///
    /// -- Verify indexes matching a name pattern
    /// SELECT * FROM pdb.verify_all_indexes(index_pattern := 'search_%');
    ///
    /// -- Stop on first corrupted index
    /// SELECT * FROM pdb.verify_all_indexes(
    ///     heapallindexed := true,
    ///     on_error_stop := true);
    ///
    /// -- Get summary of verification results
    /// SELECT indexname,
    ///        bool_and(passed) as all_passed,
    ///        count(*) filter (where not passed) as failed_checks
    /// FROM pdb.verify_all_indexes(heapallindexed := true)
    /// GROUP BY indexname;
    /// ```
    #[allow(clippy::type_complexity)]
    #[pg_extern]
    pub fn verify_all_indexes(
        schema_pattern: default!(Option<String>, "NULL"),
        index_pattern: default!(Option<String>, "NULL"),
        heapallindexed: default!(bool, false),
        sample_rate: default!(Option<f64>, "NULL"),
        report_progress: default!(bool, false),
        on_error_stop: default!(bool, false),
    ) -> Result<
        TableIterator<
            'static,
            (
                name!(schemaname, String),
                name!(indexname, String),
                name!(check_name, String),
                name!(passed, bool),
                name!(details, Option<String>),
            ),
        >,
    > {
        // Validate sample_rate (None means check all)
        let sample_rate = sample_rate.map(|r| r.clamp(0.0, 1.0));
        let mut results = Vec::new();

        // Build query with optional pattern filters
        let mut query = String::from(
            r#"
        SELECT
            n.nspname::text AS schemaname,
            i.relname::text AS indexname,
            i.oid AS indexrelid
        FROM pg_index idx
        JOIN pg_class i ON idx.indexrelid = i.oid
        JOIN pg_class t ON idx.indrelid = t.oid
        JOIN pg_namespace n ON i.relnamespace = n.oid
        JOIN pg_am am ON i.relam = am.oid
        WHERE am.amname = 'bm25'
    "#,
        );

        if schema_pattern.is_some() {
            query.push_str(" AND n.nspname LIKE $1");
        }
        if index_pattern.is_some() {
            query.push_str(if schema_pattern.is_some() {
                " AND i.relname LIKE $2"
            } else {
                " AND i.relname LIKE $1"
            });
        }
        query.push_str(" ORDER BY n.nspname, i.relname");

        // Collect indexes to verify
        let indexes: Vec<(String, String, pg_sys::Oid)> = Spi::connect(|client| {
            let mut indexes = Vec::new();

            let result = match (&schema_pattern, &index_pattern) {
                (Some(sp), Some(ip)) => {
                    let args = unsafe {
                        [
                            DatumWithOid::new(sp.clone().into_datum(), pg_sys::TEXTOID),
                            DatumWithOid::new(ip.clone().into_datum(), pg_sys::TEXTOID),
                        ]
                    };
                    client.select(&query, None, &args)?
                }
                (Some(sp), None) => {
                    let args =
                        unsafe { [DatumWithOid::new(sp.clone().into_datum(), pg_sys::TEXTOID)] };
                    client.select(&query, None, &args)?
                }
                (None, Some(ip)) => {
                    let args =
                        unsafe { [DatumWithOid::new(ip.clone().into_datum(), pg_sys::TEXTOID)] };
                    client.select(&query, None, &args)?
                }
                (None, None) => {
                    let args: [DatumWithOid; 0] = [];
                    client.select(&query, None, &args)?
                }
            };

            for row in result {
                let schemaname: String = row.get_by_name("schemaname")?.unwrap_or_default();
                let indexname: String = row.get_by_name("indexname")?.unwrap_or_default();
                let indexrelid: pg_sys::Oid =
                    row.get_by_name("indexrelid")?.unwrap_or(pg_sys::InvalidOid);
                indexes.push((schemaname, indexname, indexrelid));
            }

            Ok::<_, pgrx::spi::Error>(indexes)
        })?;

        let total_indexes = indexes.len();
        if report_progress {
            pgrx::warning!(
                "verify_all_indexes: Found {} BM25 indexes to verify",
                total_indexes
            );
        }

        // Verify each index
        for (idx_num, (schemaname, indexname, indexrelid)) in indexes.into_iter().enumerate() {
            if report_progress {
                pgrx::warning!(
                    "verify_all_indexes: Verifying {}/{}: {}.{}",
                    idx_num + 1,
                    total_indexes,
                    schemaname,
                    indexname
                );
            }

            // Call verify_index for this index
            let index_rel =
                unsafe { PgRelation::with_lock(indexrelid, pg_sys::AccessShareLock as _) };
            let verification = verify_index(
                index_rel,
                heapallindexed,
                sample_rate,
                false, // Don't double-report progress
                false, // Not verbose for bulk checks
                on_error_stop,
                None, // Check all segments
            );

            match verification {
                Ok(iter) => {
                    let mut had_error = false;
                    for (check_name, passed, details) in iter {
                        if !passed {
                            had_error = true;
                        }
                        results.push((
                            schemaname.clone(),
                            indexname.clone(),
                            check_name,
                            passed,
                            details,
                        ));
                    }
                    if on_error_stop && had_error {
                        return Ok(TableIterator::new(results));
                    }
                }
                Err(e) => {
                    results.push((
                        schemaname.clone(),
                        indexname.clone(),
                        "verification_error".to_string(),
                        false,
                        Some(format!("Failed to verify index: {}", e)),
                    ));
                    if on_error_stop {
                        return Ok(TableIterator::new(results));
                    }
                }
            }
        }

        if report_progress {
            let passed_count = results.iter().filter(|(_, _, _, p, _)| *p).count();
            let failed_count = results.len() - passed_count;
            pgrx::warning!(
                "verify_all_indexes: Complete. {} checks passed, {} failed",
                passed_count,
                failed_count
            );
        }

        Ok(TableIterator::new(results))
    }
}
