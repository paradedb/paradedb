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

pub mod numeric;
pub mod string;

use crate::index::fast_fields_helper::{FFHelper, FastFieldType, WhichFastField};
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::{SearchIndexReader, SearchIndexScore, SearchResults};
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
use std::collections::HashSet;
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
pub unsafe fn count_fast_fields(
    builder: &mut CustomPathBuilder<PrivateData>,
    rti: pg_sys::Index,
    table: &PgRelation,
    schema: &SearchIndexSchema,
    target_list: *mut pg_sys::List,
    referenced_columns: &HashSet<pg_sys::AttrNumber>,
) -> f64 {
    let ff =
        pullup_fast_fields(target_list, referenced_columns, schema, table, rti).unwrap_or_default();

    builder.custom_private().set_maybe_ff(!ff.is_empty());
    ff.iter().sorted().dedup().count() as f64
}

/// Find all the fields that can be used as "fast fields", categorize them as [`WhichFastField`]s,
/// and return the list. If there are none, or one or more of the fields can't be used as a
/// "fast field", we return [`None`].
pub unsafe fn collect_fast_fields(
    maybe_ff: bool,
    target_list: *mut pg_sys::List,
    referenced_columns: &HashSet<pg_sys::AttrNumber>,
    rti: pg_sys::Index,
    schema: &SearchIndexSchema,
    heaprel: &PgRelation,
) -> Option<Vec<WhichFastField>> {
    if maybe_ff {
        pullup_fast_fields(target_list, referenced_columns, schema, heaprel, rti)
    } else {
        None
    }
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
                if schema.is_fast_field(att.name()) {
                    let ff_type = if att.type_oid().value() == pg_sys::TEXTOID
                        || att.type_oid().value() == pg_sys::VARCHAROID
                    {
                        FastFieldType::String
                    } else {
                        FastFieldType::Numeric
                    };

                    matches.push(WhichFastField::Named(att.name().to_string(), ff_type));
                }
            }
            // If the attribute doesn't exist in this relation, just continue
            // This can happen in JOIN queries or materialized views
        }
    }
    true
}

pub unsafe fn pullup_fast_fields(
    node: *mut pg_sys::List,
    referenced_columns: &HashSet<pg_sys::AttrNumber>,
    schema: &SearchIndexSchema,
    heaprel: &PgRelation,
    rti: pg_sys::Index,
) -> Option<Vec<WhichFastField>> {
    let mut matches = Vec::new();
    let mut processed_attnos = HashSet::new();

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
        } else if pgrx::is_a((*te).expr.cast(), pg_sys::NodeTag::T_Aggref) {
            matches.push(WhichFastField::Junk("agg".into()));
            continue;
        } else if nodecast!(Const, T_Const, (*te).expr).is_some() {
            matches.push(WhichFastField::Junk("const".into()));
            continue;
        } else if nodecast!(WindowFunc, T_WindowFunc, (*te).expr).is_some() {
            matches.push(WhichFastField::Junk("window".into()));
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

    if matches
        .iter()
        .sorted()
        .dedup()
        .filter(|ff| matches!(ff, WhichFastField::Named(_, FastFieldType::String)))
        .count()
        > 1
    {
        // we cannot support more than 1 different String fast field
        return None;
    }

    Some(matches)
}

pub fn is_string_agg_capable(privdata: &PrivateData) -> Option<String> {
    if privdata.limit().is_some() {
        // doing a string_agg when there's a limit is always a loss, performance-wise
        return None;
    }
    if is_all_junk(privdata.which_fast_fields()) {
        // if all the fast fields we have are Junk fields, then we're not actually
        // projecting fast fields
        return None;
    }

    let mut string_field = None;
    for ff in privdata.which_fast_fields().iter().flatten() {
        match ff {
            WhichFastField::Named(_, FastFieldType::String) if string_field.is_none() => {
                string_field = Some(ff.name())
            }
            WhichFastField::Named(_, FastFieldType::String) => {
                // too many string fields for us to be capable of doing a string_agg
                return None;
            }
            _ => {
                // noop
            }
        }
    }
    string_field
}

pub fn is_numeric_fast_field_capable(privdata: &PrivateData) -> bool {
    if privdata.referenced_columns_count() == 0 {
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
    if let ExecMethodType::FastFieldString {
        which_fast_fields, ..
    }
    | ExecMethodType::FastFieldNumeric {
        which_fast_fields, ..
    } = &state.custom_state().exec_method_type
    {
        explainer.add_text(
            "Fast Fields",
            which_fast_fields
                .iter()
                .map(|field| field.name())
                .collect::<Vec<_>>()
                .join(", "),
        );
    }

    if let ExecMethodType::FastFieldString { field, .. } = &state.custom_state().exec_method_type {
        explainer.add_text("String Agg Field", field);
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
    // Build attribute to fast field mapping
    let mut attr_to_ff_map = std::collections::HashMap::new();

    // Step 1: First try to match named attributes by name
    for i in 0..natts {
        if let Some(att) = tupdesc.get(i) {
            let att_name = att.name().to_lowercase();
            // Skip empty named attributes - handle them later
            if !att_name.is_empty() {
                // Try to find fast field with matching name
                if let Some(idx) = which_fast_fields.iter().position(|ff| {
                    if let WhichFastField::Named(name, _) = ff {
                        name.to_lowercase() == att_name
                    } else {
                        false
                    }
                }) {
                    attr_to_ff_map.insert(i, idx);
                    continue;
                }
            }
        }
    }

    // Step 2: Position-based matching for any remaining attributes
    let mut next_ff_idx = 0;

    // Simple position-based mapping, assuming attributes and fast fields are in the same order
    for i in 0..natts {
        if !attr_to_ff_map.contains_key(&i) {
            // Find next unused fast field index
            while next_ff_idx < which_fast_fields.len()
                && attr_to_ff_map.values().any(|&v| v == next_ff_idx)
            {
                next_ff_idx += 1;
            }

            if next_ff_idx < which_fast_fields.len() {
                attr_to_ff_map.insert(i, next_ff_idx);
                next_ff_idx += 1;
            }
        }
    }

    // Get pointers to datum and isnull arrays
    let datums = std::slice::from_raw_parts_mut((*slot).tts_values, natts);
    let isnull = std::slice::from_raw_parts_mut((*slot).tts_isnull, natts);

    // Process attributes using our mapping
    for i in 0..natts {
        // Ensure every attribute has a mapping
        let &ff_idx = attr_to_ff_map
            .get(&i)
            .unwrap_or_else(|| panic!("Attribute at position {} has no fast field mapping", i));
        assert!(
            ff_idx < which_fast_fields.len(),
            "Attribute at position {} maps to invalid fast field index {}",
            i,
            ff_idx
        );
        let which_fast_field = &which_fast_fields[ff_idx];
        let att = tupdesc.get(i).unwrap();

        match ff_to_datum(
            (which_fast_field, ff_idx),
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
