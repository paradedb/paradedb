// Copyright (c) 2023-2025 ParadeDB, Inc.
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

pub mod mixed;
pub mod numeric;
pub mod string;

use crate::index::fast_fields_helper::{FFHelper, FastFieldType, WhichFastField};
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::{SearchIndexReader, SearchResults};
use crate::nodecast;
use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::pdbscan::privdat::PrivateData;
use crate::postgres::customscan::pdbscan::projections::score::{score_funcoid, uses_scores};
use crate::postgres::customscan::pdbscan::{scan_state::PdbScanState, ExecMethodType, PdbScan};
use crate::schema::SearchIndexSchema;
use itertools::Itertools;
use pgrx::{pg_sys, IntoDatum, PgList, PgOid, PgRelation, PgTupleDesc};
use tantivy::DocAddress;

pub struct FastFieldExecState {
    heaprel: pg_sys::Relation,
    tupdesc: Option<PgTupleDesc<'static>>,

    ffhelper: FFHelper,

    slot: *mut pg_sys::TupleTableSlot,
    strbuf: Option<String>,
    vmbuff: pg_sys::Buffer,
    which_fast_fields: Vec<WhichFastField>,
    search_results: SearchResults,

    // tracks our previous block visibility so we can elide checking again
    blockvis: (pg_sys::BlockNumber, bool),

    did_query: bool,
}

impl Drop for FastFieldExecState {
    fn drop(&mut self) {
        unsafe {
            if crate::postgres::utils::IsTransactionState()
                && self.vmbuff != pg_sys::InvalidBuffer as pg_sys::Buffer
            {
                pg_sys::ReleaseBuffer(self.vmbuff);
            }
        }
    }
}

impl FastFieldExecState {
    pub fn new(which_fast_fields: Vec<WhichFastField>) -> Self {
        Self {
            heaprel: std::ptr::null_mut(),
            tupdesc: None,

            ffhelper: Default::default(),
            slot: std::ptr::null_mut(),
            strbuf: Some(String::with_capacity(256)),
            vmbuff: pg_sys::InvalidBuffer as pg_sys::Buffer,
            which_fast_fields,
            search_results: Default::default(),
            blockvis: (pg_sys::InvalidBlockNumber, false),
            did_query: false,
        }
    }

    pub fn reset(&mut self, state: &mut PdbScanState) {
        self.search_results = SearchResults::None;
        self.did_query = false;
        self.blockvis = (pg_sys::InvalidBlockNumber, false);
    }
}

#[inline(always)]
unsafe fn ff_to_datum(
    which_fast_field: (&WhichFastField, usize),
    typid: pg_sys::Oid,
    score: f32,
    doc_address: DocAddress,
    ff_helper: &mut FFHelper,
    strbuf: &mut Option<String>,
    slot: *const pg_sys::TupleTableSlot,
) -> Option<pg_sys::Datum> {
    let field_index = which_fast_field.1;
    let which_fast_field = which_fast_field.0;

    if typid == pg_sys::INT2OID || typid == pg_sys::INT4OID || typid == pg_sys::INT8OID {
        match ff_helper.i64(field_index, doc_address) {
            None => None,
            Some(v) => v.into_datum(),
        }
    } else if matches!(which_fast_field, WhichFastField::Ctid) {
        (*slot).tts_tid.into_datum()
    } else if matches!(which_fast_field, WhichFastField::TableOid) {
        (*slot).tts_tableOid.into_datum()
    } else if matches!(which_fast_field, WhichFastField::Score) {
        score.into_datum()
    } else if matches!(
        which_fast_field,
        WhichFastField::Named(_, FastFieldType::String)
    ) {
        if let Some(s) = strbuf {
            s.as_str().into_datum()
        } else {
            None
        }
    } else if typid == pg_sys::TEXTOID || typid == pg_sys::VARCHAROID {
        // NB:  we don't actually support text-based fast fields... yet
        // but if we did, we'd want to do it this way
        if let Some(s) = strbuf {
            ff_helper
                .string(field_index, doc_address, s)
                .and_then(|_| s.as_str().into_datum())
        } else {
            None
        }
    } else {
        match ff_helper.value(field_index, doc_address) {
            None => None,
            Some(value) => value
                .try_into_datum(PgOid::from_untagged(typid))
                .expect("value should be convertible to Datum"),
        }
    }
}

/// Count how many "fast fields" are requested to be used by the query, as described by the `builder` argument.
pub unsafe fn count(
    builder: &mut CustomPathBuilder<PrivateData>,
    rti: pg_sys::Index,
    table: &PgRelation,
    schema: &SearchIndexSchema,
    target_list: *mut pg_sys::List,
) -> f64 {
    pgrx::warning!("Counting fast fields");
    let ff = collect_fast_fields(target_list, schema, table, rti);

    let ff_count = ff.len() as f64;
    pgrx::warning!("Found {} fast fields in count", ff_count);

    builder.custom_private().set_maybe_ff(!ff.is_empty());
    ff.iter().sorted().dedup().count() as f64
}

/// Find all fields that can be used as "fast fields" without failing if some fields are not fast fields
pub unsafe fn collect_fast_fields(
    node: *mut pg_sys::List,
    schema: &SearchIndexSchema,
    heaprel: &PgRelation,
    rti: pg_sys::Index,
) -> Vec<WhichFastField> {
    let mut matches = Vec::new();

    let tupdesc = heaprel.tuple_desc();
    let targetlist = PgList::<pg_sys::TargetEntry>::from_pg(node);

    pgrx::warning!(
        "Collecting fast fields from target list with {} entries",
        targetlist.len()
    );

    for te in targetlist.iter_ptr() {
        if (*te).resorigtbl != pg_sys::Oid::INVALID && (*te).resorigtbl != heaprel.oid() {
            continue;
        }

        if let Some(var) = nodecast!(Var, T_Var, (*te).expr) {
            if (*var).varno as i32 != rti as i32 {
                continue;
            }

            match (*var).varattno as i32 {
                // Special system attributes that are treated as fast fields
                pg_sys::SelfItemPointerAttributeNumber => {
                    matches.push(WhichFastField::Ctid);
                }

                pg_sys::TableOidAttributeNumber => {
                    matches.push(WhichFastField::TableOid);
                }

                // Regular columns
                attno => {
                    let att = tupdesc.get((attno - 1) as usize).unwrap_or_else(|| {
                        panic!(
                            "attno {attno} should exist in tupdesc from relation {} (`{}`)",
                            heaprel.oid().to_u32(),
                            heaprel.name()
                        )
                    });

                    if schema.is_fast_field(att.name()) {
                        let ff_type = if (*var).vartype == pg_sys::TEXTOID
                            || (*var).vartype == pg_sys::VARCHAROID
                        {
                            FastFieldType::String
                        } else {
                            FastFieldType::Numeric
                        };

                        matches.push(WhichFastField::Named(att.name().to_string(), ff_type));
                    }
                    // If not a fast field, simply continue without adding
                }
            }
        } else if uses_scores((*te).expr.cast(), score_funcoid(), rti) {
            matches.push(WhichFastField::Score);
        } else if pgrx::is_a((*te).expr.cast(), pg_sys::NodeTag::T_Aggref) {
            matches.push(WhichFastField::Junk("agg".into()));
        } else if nodecast!(Const, T_Const, (*te).expr).is_some() {
            matches.push(WhichFastField::Junk("const".into()));
        } else if nodecast!(WindowFunc, T_WindowFunc, (*te).expr).is_some() {
            matches.push(WhichFastField::Junk("window".into()));
        }
        // If it's not one of these types, simply continue without adding
    }

    pgrx::warning!("Found {} fast fields in collect_fast_fields", matches.len());
    matches
}

/// Find all the fields that can be as "fast fields", categorize them as [`WhichFastField`]s, and
/// return the list.  If there are none, or one or more of the fields can't be used as a "fast field",
/// we return [`None`].
pub unsafe fn collect(
    maybe_ff: bool,
    target_list: *mut pg_sys::List,
    rti: pg_sys::Index,
    schema: &SearchIndexSchema,
    heaprel: &PgRelation,
) -> Option<Vec<WhichFastField>> {
    pgrx::warning!("Collecting fast fields, maybe_ff: {}", maybe_ff);
    if maybe_ff {
        let fast_fields = collect_fast_fields(target_list, schema, heaprel, rti);

        if fast_fields.is_empty() {
            pgrx::warning!("No fast fields found");
            None
        } else {
            pgrx::warning!("Found {} fast fields in collect", fast_fields.len());
            Some(fast_fields)
        }
    } else {
        pgrx::warning!("maybe_ff is false, returning None");
        None
    }
}

pub unsafe fn pullup_fast_fields(
    node: *mut pg_sys::List,
    schema: &SearchIndexSchema,
    heaprel: &PgRelation,
    rti: pg_sys::Index,
) -> Option<Vec<WhichFastField>> {
    // Using our new function that continues even if fields aren't fast fields
    let fast_fields = collect_fast_fields(node, schema, heaprel, rti);

    pgrx::warning!("Pulled up {} fast fields", fast_fields.len());

    if fast_fields.is_empty() {
        pgrx::warning!("No fast fields found in pullup");
        None
    } else {
        pgrx::warning!("Successfully found fast fields: {:?}", fast_fields);
        Some(fast_fields)
    }
}

// Check if we can use the mixed fast field execution method
pub fn is_mixed_fast_field_capable(privdata: &PrivateData) -> bool {
    pgrx::warning!("Checking if mixed fast field capable...");
    if let Some(which_fast_fields) = privdata.which_fast_fields() {
        pgrx::warning!("Found fast fields: {:?}", which_fast_fields);

        // Filter out junk and system fields for our analysis
        let real_fields: Vec<_> = which_fast_fields
            .iter()
            .filter(|ff| {
                matches!(ff, WhichFastField::Named(_, _)) && !matches!(ff, WhichFastField::Junk(_))
            })
            .collect();

        if real_fields.len() < 1 {
            pgrx::warning!(
                "Not enough real fast fields for mixed execution: {}",
                real_fields.len()
            );
            return false; // Need at least 1 actual field
        }

        // Count string and numeric fast fields
        let string_field_count = real_fields
            .iter()
            .filter(|ff| matches!(ff, WhichFastField::Named(_, FastFieldType::String)))
            .count();

        let numeric_field_count = real_fields
            .iter()
            .filter(|ff| matches!(ff, WhichFastField::Named(_, FastFieldType::Numeric)))
            .count();

        pgrx::warning!(
            "String fields: {}, Numeric fields: {}",
            string_field_count,
            numeric_field_count
        );

        // We should use mixed fast fields if:
        // 1. We have multiple string fields (string-only case but more than one)
        // 2. We have both string and numeric fields (truly mixed case)
        let result = string_field_count > 1 || (string_field_count > 0 && numeric_field_count > 0);
        pgrx::warning!("Is mixed fast field capable: {}", result);
        return result;
    }
    pgrx::warning!("No fast fields found");
    false
}

// Update is_string_agg_capable to consider mixed fast fields
pub fn is_string_agg_capable(privdata: &PrivateData) -> Option<String> {
    pgrx::warning!("Checking if string agg capable...");

    // Don't use string_agg if we've determined this is a mixed field case
    if is_mixed_fast_field_capable(privdata) {
        pgrx::warning!("Not string agg capable because mixed fast fields detected");
        return None;
    }

    if privdata.limit().is_some() {
        // doing a string_agg when there's a limit is always a loss, performance-wise
        pgrx::warning!("Not string agg capable because limit is set");
        return None;
    }

    if is_all_junk(privdata.which_fast_fields()) {
        // if all the fast fields we have are Junk fields, then we're not actually
        // projecting fast fields
        pgrx::warning!("Not string agg capable because all fast fields are junk");
        return None;
    }

    let mut string_field = None;
    if let Some(fast_fields) = privdata.which_fast_fields() {
        for ff in fast_fields {
            match ff {
                WhichFastField::Named(_, FastFieldType::String) if string_field.is_none() => {
                    string_field = Some(ff.name());
                    pgrx::warning!("Found string field for string agg: {}", ff.name());
                }
                WhichFastField::Named(_, FastFieldType::String) => {
                    // too many string fields for us to be capable of doing a string_agg
                    pgrx::warning!("Too many string fields for string agg");
                    return None;
                }
                _ => {
                    // noop
                }
            }
        }
    }

    if string_field.is_some() {
        pgrx::warning!("String agg capable with field: {:?}", string_field);
    } else {
        pgrx::warning!("Not string agg capable - no string fields found");
    }

    string_field
}

pub fn is_numeric_fast_field_capable(privdata: &PrivateData) -> bool {
    if privdata.targetlist_len() == 0 {
        return true;
    }

    let which_fast_fields = privdata.which_fast_fields();
    if which_fast_fields.is_none() {
        return false;
    }

    if is_all_junk(which_fast_fields) {
        // if all the fast fields we have are Junk fields, then we're not actually
        // projecting fast fields
        return false;
    }

    for ff in which_fast_fields.iter().flatten() {
        if matches!(ff, WhichFastField::Named(_, FastFieldType::String)) {
            return false;
        }
    }
    true
}

fn is_all_junk(which_fast_fields: &Option<Vec<WhichFastField>>) -> bool {
    which_fast_fields
        .iter()
        .flatten()
        .all(|ff| matches!(ff, WhichFastField::Junk(_)))
}

/// Add nodes to `EXPLAIN` output to describe the "fast fields" being used by the query, if any
pub fn explain(state: &CustomScanStateWrapper<PdbScan>, explainer: &mut Explainer) {
    use crate::postgres::customscan::builders::custom_path::ExecMethodType;

    match &state.custom_state().exec_method_type {
        ExecMethodType::FastFieldString {
            field,
            which_fast_fields,
        } => {
            explainer.add_text("Fast Fields", field);
            explainer.add_text("String Agg Field", field);
        }
        ExecMethodType::FastFieldNumeric { which_fast_fields } => {
            let fields: Vec<_> = which_fast_fields.iter().map(|ff| ff.name()).collect();
            explainer.add_text("Fast Fields", fields.join(", "));
        }
        ExecMethodType::FastFieldMixed { which_fast_fields } => {
            // Get all fast fields used
            let string_fields: Vec<_> = which_fast_fields
                .iter()
                .filter(|ff| matches!(ff, WhichFastField::Named(_, FastFieldType::String)))
                .map(|ff| ff.name())
                .collect();

            let numeric_fields: Vec<_> = which_fast_fields
                .iter()
                .filter(|ff| matches!(ff, WhichFastField::Named(_, FastFieldType::Numeric)))
                .map(|ff| ff.name())
                .collect();

            let all_fields = [string_fields.clone(), numeric_fields.clone()].concat();

            explainer.add_text("Fast Fields", all_fields.join(", "));

            if !string_fields.is_empty() {
                explainer.add_text("String Fast Fields", string_fields.join(", "));
            }

            if !numeric_fields.is_empty() {
                explainer.add_text("Numeric Fast Fields", numeric_fields.join(", "));
            }
        }
        _ => {}
    }
}

pub fn estimate_cardinality(indexrel: &PgRelation, field: &str) -> Option<usize> {
    let reader = SearchIndexReader::open(indexrel, MvccSatisfies::Snapshot)
        .expect("estimate_cardinality: should be able to open SearchIndexReader");
    let searcher = reader.searcher();
    let largest_segment_reader = searcher
        .segment_readers()
        .iter()
        .max_by_key(|sr| sr.num_docs())
        .unwrap();

    Some(
        largest_segment_reader
            .fast_fields()
            .str(field)
            .ok()
            .flatten()?
            .num_terms(),
    )
}
