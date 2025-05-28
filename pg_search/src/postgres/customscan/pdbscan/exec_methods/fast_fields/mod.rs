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

use std::rc::Rc;

use crate::api::HashSet;
use crate::gucs;
use crate::index::fast_fields_helper::{FFHelper, FastFieldType, WhichFastField};
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::{SearchIndexReader, SearchResults};
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
use tantivy::columnar::StrColumn;
use tantivy::termdict::TermOrdinal;
use tantivy::DocAddress;

const NULL_TERM_ORDINAL: TermOrdinal = u64::MAX;

pub struct FastFieldExecState {
    heaprel: pg_sys::Relation,
    tupdesc: Option<PgTupleDesc<'static>>,

    /// Execution time WhichFastFields.
    which_fast_fields: Vec<WhichFastField>,
    ffhelper: FFHelper,

    slot: *mut pg_sys::TupleTableSlot,
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

/// Extracts a non-String fast field value to a Datum.
///
/// String fast fields are fetched separately using a batch dictionary-based lookup.
#[inline(always)]
pub unsafe fn non_string_ff_to_datum(
    which_fast_field: (&WhichFastField, usize),
    typid: pg_sys::Oid,
    score: f32,
    doc_address: DocAddress,
    ff_helper: &mut FFHelper,
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
        panic!("String fast field {which_fast_field:?} should already have been extracted.");
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
    is_execution_time: bool,
) -> Vec<WhichFastField> {
    let fast_fields = pullup_fast_fields(
        target_list,
        referenced_columns,
        schema,
        heaprel,
        rti,
        is_execution_time,
    );
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
    is_execution_time: bool,
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
                // We expect all Vars in the target list to be from the same range table as the
                // index we're searching, so if we see a Var from a different range table, we skip it.
                if is_execution_time {
                    // This is a sanity check to ensure that the target list is consistent with the
                    // index we're searching. As we're not supporting JOINs and Projection, at
                    // execution time (not planning time), we expect all Vars in the target list to
                    // be from the same range table as the index we're searching.
                    debug_assert_eq!(
                        (*var).varno as i32,
                        rti as i32,
                        "Encountered a Var with a different range table index.",
                    );
                }
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
            let create_resname = |base: &str, te: &pg_sys::TargetEntry| {
                let restype = (*te.expr).type_;
                let resno = te.resno;
                let isjunk = te.resjunk;
                format!(
                    "{}(resno={}, restype={:?}, resjunk={})",
                    base, resno, restype, isjunk
                )
            };
            let resname = if (*te).resname.is_null() {
                create_resname("NONAME", &*te)
            } else {
                unsafe {
                    std::ffi::CStr::from_ptr((*te).resname)
                        .to_str()
                        .unwrap_or(create_resname("INVALID_NAME_STRING", &*te).as_str())
                }
                .to_string()
            };

            matches.push(WhichFastField::Junk(resname));
            continue;
        }
        // we only support Vars or our score function in the target list
        // Other nodes (e.g., T_SubPlan, T_FuncExpr, T_OpExpr, T_CaseExpr, T_PlaceHolderVar, etc.)
        // are not supported in FastFields yet
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

fn fast_field_capable_prereqs(privdata: &PrivateData) -> bool {
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

/// Check if we can use the String fast field execution method
///
/// Using StringFF when there's a limit is always a loss, performance-wise, because it
/// collects the full set of query results (as doc ids and term ordinals) before beginning
/// to return rows. Meanwhile, Normal is fully lazy but unsorted, and TopN searches
/// eagerly, but avoids actually emitting anything but the limit.
pub fn is_string_fast_field_capable(privdata: &PrivateData) -> Option<String> {
    if !gucs::is_fast_field_exec_enabled() {
        return None;
    }

    if privdata.limit().is_some() {
        // See the method doc with regard to limits/laziness.
        return None;
    }

    if !fast_field_capable_prereqs(privdata) {
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

/// Check if we can use the Numeric fast field execution method
pub fn is_numeric_fast_field_capable(privdata: &PrivateData) -> bool {
    if !gucs::is_fast_field_exec_enabled() {
        return false;
    }

    if !fast_field_capable_prereqs(privdata) {
        return false;
    }

    let which_fast_fields = privdata.planned_which_fast_fields().as_ref().unwrap();
    // Make sure we don't have any string fast fields
    for ff in which_fast_fields.iter() {
        if matches!(ff, WhichFastField::Named(_, FastFieldType::String)) {
            return false;
        }
    }
    true
}

/// Check if we can use the Mixed fast field execution method
///
/// MixedFF is subject to the same constraints around limits and laziness as StringFF: see
/// `is_string_fast_field_capable`.
pub fn is_mixed_fast_field_capable(privdata: &PrivateData) -> bool {
    if !gucs::is_mixed_fast_field_exec_enabled() {
        return false;
    }

    if privdata.limit().is_some() {
        // See the method doc with regard to limits/laziness.
        return false;
    }

    if !fast_field_capable_prereqs(privdata) {
        return false;
    }

    // We should only use Mixed if there is at least one named fast field, but fewer than the
    // configured column threshold.
    let which_fast_fields = privdata.planned_which_fast_fields().as_ref().unwrap();
    let named_field_count = which_fast_fields
        .iter()
        .filter(|wff| matches!(wff, WhichFastField::Named(_, _)))
        .count();

    0 < named_field_count && named_field_count < gucs::mixed_fast_field_exec_column_threshold()
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

/// Given a collection of values containing TermOrdinals for the given StrColumn, return an iterator
/// which zips each value with the term for the TermOrdinal in ascending sorted order.
///
/// `NULL_TERM_ORDINAL` represents NULL, and will be emitted last in the sorted order.
pub fn ords_to_sorted_terms<T>(
    str_ff: StrColumn,
    mut items: Vec<T>,
    ordinal_fn: impl Fn(&T) -> TermOrdinal,
) -> impl Iterator<Item = (T, Option<Rc<str>>)> {
    items.sort_unstable_by_key(&ordinal_fn);

    let mut bytes = Vec::new();
    let mut current_block_addr = str_ff.dictionary().sstable_index.get_block_with_ord(0);
    let mut current_sstable_delta_reader = str_ff
        .dictionary()
        .sstable_delta_reader_block(current_block_addr.clone())
        .expect("Failed to open term dictionary.");
    let mut current_ordinal = 0;
    let mut previous_term: Option<(TermOrdinal, Option<Rc<str>>)> = None;
    let mut items = items.into_iter();
    std::iter::from_fn(move || {
        let item = items.next()?;
        let ord = ordinal_fn(&item);

        // only advance forward if the new ord is different than the one we just processed
        //
        // this allows the input TermOrdinal iterator to contain and reuse duplicates, so long as
        // it's still sorted
        match &previous_term {
            Some((previous_ord, term)) if *previous_ord == ord => {
                // This is the same term ordinal: reuse the previous term value.
                return Some((item, term.clone()));
            }
            // Fall through.
            _ => {}
        }

        // This is a new term ordinal: decode and allocate it.
        assert!(ord >= current_ordinal);
        // check if block changed for new term_ord
        let new_block_addr = str_ff.dictionary().sstable_index.get_block_with_ord(ord);
        if new_block_addr != current_block_addr {
            current_block_addr = new_block_addr;
            current_ordinal = current_block_addr.first_ordinal;
            current_sstable_delta_reader = str_ff
                .dictionary()
                .sstable_delta_reader_block(current_block_addr.clone())
                .unwrap_or_else(|e| panic!("Failed to fetch next dictionary block: {e}"));
            bytes.clear();
        }

        // move to ord inside that block
        for _ in current_ordinal..=ord {
            match current_sstable_delta_reader.advance() {
                Ok(true) => {}
                Ok(false) if ord == NULL_TERM_ORDINAL => {
                    // NULL_TERM_ORDINAL sorts highest, so all remaining terms are None.
                    previous_term = Some((ord, None));
                    return Some((item, None));
                }
                Ok(false) => {
                    panic!("Term ordinal {ord} did not exist in the dictionary.");
                }
                Err(e) => {
                    panic!("Failed to decode dictionary block: {e}")
                }
            }
            bytes.truncate(current_sstable_delta_reader.common_prefix_len());
            bytes.extend_from_slice(current_sstable_delta_reader.suffix());
        }
        current_ordinal = ord + 1;

        let term: Option<Rc<str>> = Some(
            std::str::from_utf8(&bytes)
                .expect("term should be valid utf8")
                .into(),
        );
        previous_term = Some((ord, term.clone()));
        Some((item, term))
    })
}
