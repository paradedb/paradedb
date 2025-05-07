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
use crate::index::reader::index::{SearchIndexReader, SearchIndexScore, SearchResults};
use crate::nodecast;
use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::pdbscan::privdat::PrivateData;
use crate::postgres::customscan::pdbscan::projections::score::{score_funcoid, uses_scores};
use crate::postgres::customscan::pdbscan::{scan_state::PdbScanState, PdbScan};
use crate::schema::SearchIndexSchema;
use itertools::Itertools;
use pgrx::pg_sys::CustomScanState;
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

    let ff_count = ff.len() as f64;
    pgrx::warning!("Found {} fast fields in count", ff_count);

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
    pgrx::warning!("Collecting fast fields, maybe_ff: {}", maybe_ff);
    if maybe_ff {
        let fast_fields = pullup_fast_fields(target_list, referenced_columns, schema, heaprel, rti);

        if let Some(fast_fields) = fast_fields {
            if fast_fields.is_empty() {
                pgrx::warning!("No fast fields found");
                None
            } else {
                pgrx::warning!("Found {} fast fields in collect", fast_fields.len());
                Some(fast_fields)
            }
        } else {
            pgrx::warning!("No fast fields found");
            None
        }
    } else {
        pgrx::warning!("maybe_ff is false, returning None");
        None
    }
}

// Helper function to process an attribute number and add a fast field if appropriate
fn collect_fast_field_try_for_attno(
    attno: i32,
    processed_attnos: &mut HashSet<pg_sys::AttrNumber>,
    matches: &mut Vec<WhichFastField>,
    field_names: &mut HashSet<String>,
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

                    // Add a field to the matches list - field_names is only used for
                    // duplicate detection, not as part of the actual data structure
                    if field_names.insert(att_name.clone()) {
                        pgrx::warning!("⭐️ Adding fast field: attno={}, name={}", attno, att_name);
                        matches.push(WhichFastField::Named(att_name, ff_type));
                    } else {
                        pgrx::warning!(
                            "⭐️ Skipping duplicate fast field: attno={}, name={}",
                            attno,
                            att_name
                        );
                    }
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
    let mut processed_attnos = HashSet::new();
    // Track field names to avoid duplicates - still needed for case where different attnos
    // might map to same field name in different relations
    let mut field_names = HashSet::new();

    let tupdesc = heaprel.tuple_desc();

    // First collect all matches from the target list (standard behavior)
    let targetlist = PgList::<pg_sys::TargetEntry>::from_pg(node);

    pgrx::warning!(
        "Collecting fast fields from target list with {} entries",
        targetlist.len()
    );

    // Process target list entries
    for te in targetlist.iter_ptr() {
        if (*te).resorigtbl != pg_sys::Oid::INVALID && (*te).resorigtbl != heaprel.oid() {
            pgrx::warning!(
                "⭐️ Skipping target entry: resorigtbl={:?} vs heaprel.oid={:?}",
                (*te).resorigtbl,
                heaprel.oid()
            );
            continue;
        }

        if let Some(var) = nodecast!(Var, T_Var, (*te).expr) {
            if (*var).varno as i32 != rti as i32 {
                pgrx::warning!("⭐️ Skipping var: varno={} vs rti={}", (*var).varno, rti);
                continue;
            }
            let attno = (*var).varattno as i32;
            pgrx::warning!("⭐️ Processing var: attno={}", attno);
            if !collect_fast_field_try_for_attno(
                attno,
                &mut processed_attnos,
                &mut matches,
                &mut field_names,
                &tupdesc,
                heaprel,
                schema,
            ) {
                pgrx::warning!("⭐️ Cannot use fast fields for attno={}", attno);
                return None;
            }
            continue;
        } else if uses_scores((*te).expr.cast(), score_funcoid(), rti) {
            pgrx::warning!("⭐️ Adding score fast field");
            matches.push(WhichFastField::Score);
            continue;
        } else if pgrx::is_a((*te).expr.cast(), pg_sys::NodeTag::T_Aggref) {
            pgrx::warning!("⭐️ Adding agg junk fast field");
            matches.push(WhichFastField::Junk("agg".into()));
            continue;
        } else if nodecast!(Const, T_Const, (*te).expr).is_some() {
            pgrx::warning!("⭐️ Adding const junk fast field");
            matches.push(WhichFastField::Junk("const".into()));
            continue;
        } else if nodecast!(WindowFunc, T_WindowFunc, (*te).expr).is_some() {
            pgrx::warning!("⭐️ Adding window junk fast field");
            matches.push(WhichFastField::Junk("window".into()));
            continue;
        }
        // we only support Vars or our score function in the target list
        pgrx::warning!("⭐️ Unsupported node type in target list");
        return None;
    }

    // Now also consider all referenced columns from other parts of the query
    for &attno in referenced_columns {
        pgrx::warning!("⭐️ Processing referenced column: attno={}", attno);
        if !collect_fast_field_try_for_attno(
            attno as i32,
            &mut processed_attnos,
            &mut matches,
            &mut field_names,
            &tupdesc,
            heaprel,
            schema,
        ) {
            pgrx::warning!(
                "⭐️ Cannot use fast fields for referenced column: attno={}",
                attno
            );
            return None;
        }
    }

    // Print collected fast fields for debugging
    pgrx::warning!("⭐️ Collected fast fields: {:?}", matches);
    pgrx::warning!("⭐️ Processed attribute numbers: {:?}", processed_attnos);

    Some(matches)
}

// Check if we can use the mixed fast field execution method
pub fn is_mixed_fast_field_capable(privdata: &PrivateData) -> bool {
    pgrx::warning!("⭐️ Checking if mixed fast field capable...");

    // Normal mixed fast field detection logic
    if let Some(which_fast_fields) = privdata.which_fast_fields() {
        pgrx::warning!("⭐️ Found fast fields: {:?}", which_fast_fields);

        // Filter out junk and system fields for our analysis - we only care about real column fast fields
        let field_types = which_fast_fields
            .iter()
            .filter_map(|ff| match ff {
                WhichFastField::Named(name, ff_type) => Some((name.clone(), ff_type.clone())),
                _ => None,
            })
            .collect::<Vec<_>>();

        pgrx::warning!("⭐️ Filtered field types: {:?}", field_types);

        if field_types.is_empty() {
            pgrx::warning!("⭐️ No named fast fields, returning false");
            return false; // No named fast fields
        }

        // Count string and numeric fast fields
        let string_field_count = field_types
            .iter()
            .filter(|(_, ff_type)| matches!(ff_type, FastFieldType::String))
            .count();

        let numeric_field_count = field_types
            .iter()
            .filter(|(_, ff_type)| matches!(ff_type, FastFieldType::Numeric))
            .count();

        pgrx::warning!(
            "⭐️ String fields: {}, Numeric fields: {}",
            string_field_count,
            numeric_field_count
        );

        // If we have multiple string fields or a mix of string and numeric fields,
        // we should use MixedFastFieldExecState
        let should_use_mixed =
            string_field_count > 1 || (string_field_count > 0 && numeric_field_count > 0);

        pgrx::warning!("⭐️ Should use mixed fast fields: {}", should_use_mixed);
        return should_use_mixed;
    }

    pgrx::warning!("⭐️ No fast fields in privdata");
    false
}

// Update is_string_agg_capable to consider test requirements
pub fn is_string_agg_capable(privdata: &PrivateData) -> Option<String> {
    pgrx::warning!("⭐️ Checking if string agg capable...");
    if privdata.limit().is_some() {
        // doing a string_agg when there's a limit is always a loss, performance-wise
        pgrx::warning!("⭐️ Not string agg capable because limit is set");
        return None;
    }

    let maybe_which_fast_fields = privdata.which_fast_fields();
    if maybe_which_fast_fields.is_none() {
        // if we don't have any field info, we shouldn't try string_agg
        pgrx::warning!("⭐️ Not string agg capable because no fast fields");
        return None;
    } else if maybe_which_fast_fields.as_ref().unwrap().iter().all(|ff| {
        matches!(
            ff,
            WhichFastField::Junk(_)
                | WhichFastField::TableOid
                | WhichFastField::Ctid
                | WhichFastField::Score
        )
    }) {
        // if all the fast fields we have are Junk fields, then we're not actually
        // projecting fast fields
        pgrx::warning!("⭐️ Not string agg capable because all fast fields are junk");
        return None;
    }

    let mut string_field = None;
    for ff in privdata.which_fast_fields().iter().flatten() {
        match ff {
            WhichFastField::Named(name, FastFieldType::String) if string_field.is_none() => {
                string_field = Some(name.clone());
                pgrx::warning!("⭐️ Found string field for string agg: {}", name);
            }
            WhichFastField::Named(_, FastFieldType::String) => {
                // too many string fields for us to be capable of doing a string_agg
                pgrx::warning!("⭐️ Too many string fields for string agg");
                return None;
            }
            _ => {
                // noop
            }
        }
    }

    if string_field.is_some() {
        pgrx::warning!("⭐️ String agg capable with field: {:?}", string_field);
    } else {
        pgrx::warning!("⭐️ Not string agg capable - no string fields found");
    }

    string_field
}

// Check if we can use numeric fast field execution method
pub fn is_numeric_fast_field_capable(privdata: &PrivateData) -> bool {
    pgrx::warning!("⭐️ Checking if numeric fast field capable...");

    if privdata.referenced_columns_count() == 0 {
        pgrx::warning!("⭐️ No referenced columns, can use numeric fast fields");
        return true;
    }

    let which_fast_fields = privdata.which_fast_fields();
    if which_fast_fields.is_none() {
        pgrx::warning!("⭐️ No fast fields, can't use numeric fast fields");
        return false;
    }

    if is_all_junk(which_fast_fields) {
        // if all the fast fields we have are Junk fields, then we're not actually
        // projecting fast fields
        pgrx::warning!("⭐️ All junk fields, can't use numeric fast fields");
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
