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

use crate::api::HashSet;
use crate::index::fast_fields_helper::{FFHelper, FastFieldType, WhichFastField};
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::{SearchIndexReader, SearchIndexScore, SearchResults};
use crate::nodecast;
use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::pdbscan::privdat::PrivateData;
use crate::postgres::customscan::pdbscan::projections::score::{score_funcoid, uses_scores};
use crate::postgres::customscan::pdbscan::{scan_state::PdbScanState, PdbScan};
use crate::schema::SearchIndexSchema;
use itertools::Itertools;
use pgrx::pg_sys::CustomScanState;
use pgrx::{pg_sys, IntoDatum, PgList, PgOid, PgRelation, PgTupleDesc};
use tantivy::DocAddress;

pub struct FastFieldExecState {
    heaprel: pg_sys::Relation,
    tupdesc: Option<PgTupleDesc<'static>>,

    /// Execution time WhichFastFields.
    which_fast_fields: Vec<WhichFastField>,
    ffhelper: FFHelper,

    slot: *mut pg_sys::TupleTableSlot,
    strbuf: Option<String>,
    vmbuff: pg_sys::Buffer,
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
            which_fast_fields,
            ffhelper: Default::default(),
            slot: std::ptr::null_mut(),
            strbuf: Some(String::with_capacity(256)),
            vmbuff: pg_sys::InvalidBuffer as pg_sys::Buffer,
            search_results: Default::default(),
            blockvis: (pg_sys::InvalidBlockNumber, false),
            did_query: false,
        }
    }

    fn init(&mut self, state: &mut PdbScanState, cstate: *mut CustomScanState) {
        unsafe {
            self.heaprel = state.heaprel();
            self.tupdesc = Some(PgTupleDesc::from_pg_unchecked(
                (*cstate).ss.ps.ps_ResultTupleDesc,
            ));
            self.slot = pg_sys::MakeTupleTableSlot(
                (*cstate).ss.ps.ps_ResultTupleDesc,
                &pg_sys::TTSOpsVirtual,
            );
            // Initialize the fast field helper
            self.ffhelper = FFHelper::with_fields(
                state.search_reader.as_ref().unwrap(),
                &self.which_fast_fields,
            );
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

/// Find all the fields that can be used as "fast fields", categorize them as [`WhichFastField`]s,
/// and return the list. If there are none, or one or more of the fields can't be used as a
/// "fast field", we return [`None`].
pub unsafe fn collect_fast_fields(
    target_list: *mut pg_sys::List,
    referenced_columns: &HashSet<pg_sys::AttrNumber>,
    rti: pg_sys::Index,
    schema: &SearchIndexSchema,
    heaprel: &PgRelation,
) -> Vec<WhichFastField> {
    let fast_fields = pullup_fast_fields(target_list, referenced_columns, schema, heaprel, rti);
    fast_fields
        .filter(|fast_fields| !fast_fields.is_empty())
        .unwrap_or_default()
}

// Helper function to process an attribute number and add a fast field if appropriate
fn collect_fast_field_try_for_attno(
    attno: i32,
    processed_attnos: &mut HashSet<pg_sys::AttrNumber>,
    matches: &mut Vec<WhichFastField>,
    tupdesc: &PgTupleDesc<'_>,
    heaprel: &PgRelation,
    schema: &SearchIndexSchema,
) -> bool {
    // Skip if we've already processed this attribute number
    if processed_attnos.contains(&(attno as pg_sys::AttrNumber)) {
        return true;
    }

    match attno {
        // any of these mean we can't use fast fields
        pg_sys::MinTransactionIdAttributeNumber
        | pg_sys::MaxTransactionIdAttributeNumber
        | pg_sys::MinCommandIdAttributeNumber
        | pg_sys::MaxCommandIdAttributeNumber => return false,

        // these aren't _exactly_ fast fields, but we do have the information
        // readily available during the scan, so we'll pretend
        pg_sys::SelfItemPointerAttributeNumber => {
            // okay, "ctid" is a fast field but it's secret
            processed_attnos.insert(attno as pg_sys::AttrNumber);
            matches.push(WhichFastField::Ctid);
        }

        pg_sys::TableOidAttributeNumber => {
            processed_attnos.insert(attno as pg_sys::AttrNumber);
            matches.push(WhichFastField::TableOid);
        }

        attno => {
            // Keep track that we've processed this attribute number
            processed_attnos.insert(attno as pg_sys::AttrNumber);

            // Handle attno <= 0 - this can happen in materialized views and FULL JOINs
            if attno <= 0 {
                // Just mark it as processed and continue
                return true;
            }

            // Get attribute info - use if let to handle missing attributes gracefully
            if let Some(att) = tupdesc.get((attno - 1) as usize) {
                let att_name = att.name().to_string();
                if schema.is_fast_field(att.name()) {
                    let ff_type = if att.type_oid().value() == pg_sys::TEXTOID
                        || att.type_oid().value() == pg_sys::VARCHAROID
                    {
                        FastFieldType::String
                    } else {
                        FastFieldType::Numeric
                    };
                    matches.push(WhichFastField::Named(att_name, ff_type));
                }
            }
            // If the attribute doesn't exist in this relation, just continue
            // This can happen in JOIN queries or materialized views
        }
    }
    true
}

/// Find all fields that can be used as "fast fields" without failing if some fields are not fast fields
pub unsafe fn pullup_fast_fields(
    node: *mut pg_sys::List,
    referenced_columns: &HashSet<pg_sys::AttrNumber>,
    schema: &SearchIndexSchema,
    heaprel: &PgRelation,
    rti: pg_sys::Index,
) -> Option<Vec<WhichFastField>> {
    let mut matches = Vec::new();
    let mut processed_attnos = HashSet::default();

    let tupdesc = heaprel.tuple_desc();

    // First collect all matches from the target list (standard behavior)
    let targetlist = PgList::<pg_sys::TargetEntry>::from_pg(node);

    // Process target list entries
    for te in targetlist.iter_ptr() {
        if (*te).resorigtbl != pg_sys::Oid::INVALID && (*te).resorigtbl != heaprel.oid() {
            continue;
        }

        if let Some(var) = nodecast!(Var, T_Var, (*te).expr) {
            if (*var).varno as i32 != rti as i32 {
                // this TargetEntry's Var isn't from the same RangeTable as we were asked to inspect,
                // so just skip it
                continue;
            }
            let attno = (*var).varattno as i32;
            if !collect_fast_field_try_for_attno(
                attno,
                &mut processed_attnos,
                &mut matches,
                &tupdesc,
                heaprel,
                schema,
            ) {
                return None;
            }
            continue;
        } else if uses_scores((*te).expr.cast(), score_funcoid(), rti) {
            matches.push(WhichFastField::Score);
            continue;
        } else if pgrx::is_a((*te).expr.cast(), pg_sys::NodeTag::T_Aggref)
            || nodecast!(Const, T_Const, (*te).expr).is_some()
            || nodecast!(WindowFunc, T_WindowFunc, (*te).expr).is_some()
        {
            continue;
        }
        // we only support Vars or our score function in the target list
        return None;
    }

    // Now also consider all referenced columns from other parts of the query
    for &attno in referenced_columns {
        if !collect_fast_field_try_for_attno(
            attno as i32,
            &mut processed_attnos,
            &mut matches,
            &tupdesc,
            heaprel,
            schema,
        ) {
            return None;
        }
    }

    Some(matches)
}

pub fn fast_field_capable_prereqs(privdata: &PrivateData) -> bool {
    if privdata.referenced_columns_count() == 0 {
        return false;
    }

    let which_fast_fields = privdata.planned_which_fast_fields().as_ref().unwrap();

    if is_all_special_or_junk_fields(which_fast_fields) {
        // if all the fast fields we have are Junk fields, then we're not actually
        // projecting fast fields
        return false;
    }

    // Make sure all referenced columns are fast fields
    let referenced_columns_count = privdata.referenced_columns_count();

    // Count columns that we have fast fields for (excluding system/junk fields)
    let fast_field_column_count = which_fast_fields
        .iter()
        .filter(|ff| matches!(ff, WhichFastField::Named(_, _)))
        .count();

    // If we're missing any columns, we can't use fast field execution
    if referenced_columns_count > fast_field_column_count {
        return false;
    }

    true
}

// Check if we can use the mixed fast field execution method
pub fn is_mixed_fast_field_capable(privdata: &PrivateData) -> bool {
    if !fast_field_capable_prereqs(privdata) {
        return false;
    }

    // Normal mixed fast field detection logic
    let which_fast_fields = privdata.planned_which_fast_fields().as_ref().unwrap();

    // Filter out junk and system fields for our analysis - we only care about real column fast fields
    let field_types = which_fast_fields
        .iter()
        .filter_map(|ff| match ff {
            WhichFastField::Named(name, ff_type) => Some((name.clone(), ff_type.clone())),
            _ => None,
        })
        .collect::<Vec<_>>();

    if field_types.is_empty() {
        return false; // No named fast fields
    }

    true
}

// Update is_string_agg_capable to consider test requirements
pub fn is_string_agg_capable_with_prereqs(privdata: &PrivateData) -> Option<String> {
    if !fast_field_capable_prereqs(privdata) {
        return None;
    }

    is_string_agg_capable(privdata)
}
// Update is_string_agg_capable to consider test requirements
pub fn is_string_agg_capable(privdata: &PrivateData) -> Option<String> {
    if privdata.limit().is_some() {
        // doing a string_agg when there's a limit is always a loss, performance-wise
        return None;
    }

    let which_fast_fields = privdata.planned_which_fast_fields().as_ref().unwrap();

    let mut string_field = None;
    // Count the number of string fields
    let mut string_field_count = 0;
    for ff in which_fast_fields.iter() {
        if let WhichFastField::Named(name, field_type) = ff {
            match field_type {
                FastFieldType::String => {
                    string_field_count += 1;
                    string_field = Some(name.clone());
                }
                FastFieldType::Numeric => {
                    return None;
                }
            }
        }
    }

    if string_field_count != 1 {
        // string_agg requires exactly one string field
        return None;
    }

    // At this point, we've verified that we have exactly one string field
    string_field
}

// Check if we can use numeric fast field execution method
pub fn is_numeric_fast_field_capable(privdata: &PrivateData) -> bool {
    let which_fast_fields = privdata.planned_which_fast_fields().as_ref().unwrap();
    // Make sure we don't have any string fast fields
    for ff in which_fast_fields.iter() {
        if matches!(ff, WhichFastField::Named(_, FastFieldType::String)) {
            return false;
        }
    }
    true
}

fn is_all_special_or_junk_fields(which_fast_fields: &HashSet<WhichFastField>) -> bool {
    which_fast_fields.iter().all(|ff| {
        matches!(
            ff,
            WhichFastField::Junk(_)
                | WhichFastField::TableOid
                | WhichFastField::Ctid
                | WhichFastField::Score
        )
    })
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
            let fields: Vec<_> = which_fast_fields
                .iter()
                .map(|ff| ff.name())
                .sorted()
                .collect();
            explainer.add_text("Fast Fields", fields.join(", "));
        }
        ExecMethodType::FastFieldMixed { which_fast_fields } => {
            // Get all fast fields used
            let string_fields: Vec<_> = which_fast_fields
                .iter()
                .filter(|ff| matches!(ff, WhichFastField::Named(_, FastFieldType::String)))
                .map(|ff| ff.name())
                .sorted()
                .collect();

            let numeric_fields: Vec<_> = which_fast_fields
                .iter()
                .filter(|ff| matches!(ff, WhichFastField::Named(_, FastFieldType::Numeric)))
                .map(|ff| ff.name())
                .sorted()
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

/// Process attributes using fast fields, creating a mapping and populating the datum array.
/// This function is shared between the string and numeric fast field implementations.
#[allow(clippy::too_many_arguments)]
pub unsafe fn extract_data_from_fast_fields(
    natts: usize,
    tupdesc: &PgTupleDesc<'_>,
    which_fast_fields: &[WhichFastField],
    fast_fields: &mut FFHelper,
    slot: *mut pg_sys::TupleTableSlot,
    scored: SearchIndexScore,
    doc_address: DocAddress,
    string_buffer: &mut Option<String>,
) {
    let datums = std::slice::from_raw_parts_mut((*slot).tts_values, natts);
    let isnull = std::slice::from_raw_parts_mut((*slot).tts_isnull, natts);

    #[rustfmt::skip]
    debug_assert!(natts == which_fast_fields.len());

    for (i, att) in tupdesc.iter().enumerate() {
        let which_fast_field = &which_fast_fields[i];

        match ff_to_datum(
            (which_fast_field, i),
            att.atttypid,
            scored.bm25,
            doc_address,
            fast_fields,
            string_buffer,
            slot,
        ) {
            None => {
                datums[i] = pg_sys::Datum::null();
                isnull[i] = true;
            }
            Some(datum) => {
                datums[i] = datum;
                isnull[i] = false;
            }
        }
    }
}
