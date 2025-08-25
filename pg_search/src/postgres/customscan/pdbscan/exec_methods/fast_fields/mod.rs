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

use std::sync::Arc;

use crate::api::FieldName;
use crate::api::HashSet;
use crate::gucs;
use crate::index::fast_fields_helper::{FFHelper, FastFieldType, WhichFastField};
use crate::nodecast;
use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::pdbscan::privdat::PrivateData;
use crate::postgres::customscan::pdbscan::projections::score::uses_scores;
use crate::postgres::customscan::pdbscan::{scan_state::PdbScanState, PdbScan};
use crate::postgres::customscan::score_funcoid;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::var::{find_one_var, find_one_var_and_fieldname, VarContext};

use arrow_array::builder::StringViewBuilder;
use arrow_array::ArrayRef;
use pgrx::pg_sys::CustomScanState;
use pgrx::{pg_sys, IntoDatum, PgList, PgOid, PgTupleDesc};
use tantivy::columnar::StrColumn;
use tantivy::termdict::TermOrdinal;
use tantivy::DocAddress;

const NULL_TERM_ORDINAL: TermOrdinal = u64::MAX;

pub struct FastFieldExecState {
    heaprel: Option<PgSearchRelation>,
    tupdesc: Option<PgTupleDesc<'static>>,

    /// Execution time WhichFastFields.
    which_fast_fields: Vec<WhichFastField>,
    ffhelper: FFHelper,

    slot: *mut pg_sys::TupleTableSlot,
    vmbuff: pg_sys::Buffer,

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
            heaprel: None,
            tupdesc: None,
            which_fast_fields,
            ffhelper: Default::default(),
            slot: std::ptr::null_mut(),
            vmbuff: pg_sys::InvalidBuffer as pg_sys::Buffer,
            blockvis: (pg_sys::InvalidBlockNumber, false),
            did_query: false,
        }
    }

    fn init(&mut self, state: &mut PdbScanState, cstate: *mut CustomScanState) {
        unsafe {
            self.heaprel = Some(Clone::clone(state.heaprel()));
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
    heaprel: &PgSearchRelation,
    index: &PgSearchRelation,
    is_execution_time: bool,
) -> Vec<WhichFastField> {
    let fast_fields = pullup_fast_fields(
        target_list,
        referenced_columns,
        heaprel,
        index,
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
    matches: &mut Vec<WhichFastField>,
    tupdesc: &PgTupleDesc<'_>,
    heaprel: &PgSearchRelation,
    index: &PgSearchRelation,
    fieldname: Option<&FieldName>,
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
            matches.push(WhichFastField::Ctid);
        }

        pg_sys::TableOidAttributeNumber => {
            matches.push(WhichFastField::TableOid);
        }

        attno => {
            // Handle attno <= 0 - this can happen in materialized views and FULL JOINs
            if attno <= 0 {
                // Just mark it as processed and continue
                return true;
            }

            // Get attribute info - use if let to handle missing attributes gracefully
            if let Some(att) = tupdesc.get((attno - 1) as usize) {
                let schema = index
                    .schema()
                    .expect("pullup_fast_fields: should have a schema");
                if let Some(search_field) = schema.search_field(att.name()) {
                    if search_field.is_fast() {
                        let ff_type = match att.type_oid().value() {
                            pg_sys::TEXTOID | pg_sys::VARCHAROID | pg_sys::UUIDOID => {
                                FastFieldType::String
                            }
                            pg_sys::BOOLOID
                            | pg_sys::DATEOID
                            | pg_sys::FLOAT4OID
                            | pg_sys::FLOAT8OID
                            | pg_sys::INT2OID
                            | pg_sys::INT4OID
                            | pg_sys::INT8OID
                            | pg_sys::NUMERICOID
                            | pg_sys::TIMEOID
                            | pg_sys::TIMESTAMPOID
                            | pg_sys::TIMESTAMPTZOID
                            | pg_sys::TIMETZOID => FastFieldType::Numeric,
                            _ => {
                                // This fast field type is supported for pushdown of queries, but not for
                                // rendering via fast field execution.
                                //
                                // NOTE: JSON/JSONB are included here because fast fields do not
                                // contain the full content of the JSON in a way that we can easily
                                // render: rather, the individual fields are exploded out into
                                // dynamic columns.
                                return false;
                            }
                        };
                        matches.push(WhichFastField::Named(att.name().to_string(), ff_type));
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
    heaprel: &PgSearchRelation,
    index: &PgSearchRelation,
    rti: pg_sys::Index,
    is_execution_time: bool,
) -> Option<Vec<WhichFastField>> {
    let mut matches = Vec::new();

    let tupdesc = heaprel.tuple_desc();

    // First collect all matches from the target list (standard behavior)
    let targetlist = PgList::<pg_sys::TargetEntry>::from_pg(node);

    // Process target list entries
    for te in targetlist.iter_ptr() {
        if (*te).resorigtbl != pg_sys::Oid::INVALID && (*te).resorigtbl != heaprel.oid() {
            continue;
        }

        let maybe_var = if let Some(var) = find_one_var((*te).expr.cast()) {
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
            find_one_var_and_fieldname(VarContext::from_exec(heaprel.oid()), (*te).expr.cast())
        } else {
            None
        };

        if let Some((var, fieldname)) = maybe_var {
            if !collect_fast_field_try_for_attno(
                (*var).varattno as i32,
                &mut matches,
                &tupdesc,
                heaprel,
                index,
                Some(&fieldname),
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
                format!("{base}(resno={resno}, restype={restype:?}, resjunk={isjunk})")
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
            &mut matches,
            &tupdesc,
            heaprel,
            index,
            None,
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
        // projecting fast fields, and we're better off using a Normal scan.
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

/// Check if we can use the Mixed fast field execution method
pub fn is_mixed_fast_field_capable(privdata: &PrivateData) -> bool {
    if !gucs::is_mixed_fast_field_exec_enabled() {
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

pub fn is_all_special_or_junk_fields<'a>(
    which_fast_fields: impl IntoIterator<Item = &'a WhichFastField>,
) -> bool {
    which_fast_fields.into_iter().all(|ff| {
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

    if let ExecMethodType::FastFieldMixed {
        which_fast_fields, ..
    } = &state.custom_state().exec_method_type
    {
        // Get all fast fields used
        let fields: Vec<_> = which_fast_fields
            .iter()
            .filter(|ff| matches!(ff, WhichFastField::Named(_, _)))
            .map(|ff| ff.name())
            .collect();

        explainer.add_text("Fast Fields", fields.join(", "));
    }
}

/// Given an unordered collection of TermOrdinals for the given StrColumn, return a
/// `StringViewArray` with one row per input term ordinal (in the input order).
///
/// A `StringViewArray` contains a series of buffers containing arbitrarily concatenated bytes data,
/// and then a series of (buffer, offset, len) entries representing views into those buffers. This
/// method creates a single buffer containing the concatenated data for the given term ordinals in
/// term sorted order, and then a view per input row in input order. A caller can ignore those
/// details and just consume the array as if it were an array of strings.
///
/// `NULL_TERM_ORDINAL` represents NULL, and will be emitted last in the sorted order.
pub fn ords_to_string_array(
    str_ff: StrColumn,
    term_ords: impl IntoIterator<Item = TermOrdinal>,
) -> ArrayRef {
    // Enumerate the term ordinals to preserve their positions, and then sort them by ordinal.
    let mut term_ords = term_ords.into_iter().enumerate().collect::<Vec<_>>();
    term_ords.sort_unstable_by_key(|(_, term_ord)| *term_ord);

    // Iterate over the sorted term ordinals: as we visit each term ordinal, we will append the
    // term to a StringViewBuilder's data buffer, and record a view to be appended later in sorted
    // order.
    let mut builder = StringViewBuilder::with_capacity(term_ords.len());
    let mut views: Vec<Option<(u32, u32)>> = Vec::with_capacity(term_ords.len());
    views.resize(term_ords.len(), None);

    let mut buffer = Vec::new();
    let mut bytes = Vec::new();
    let mut current_block_addr = str_ff.dictionary().sstable_index.get_block_with_ord(0);
    let mut current_sstable_delta_reader = str_ff
        .dictionary()
        .sstable_delta_reader_block(current_block_addr.clone())
        .expect("Failed to open term dictionary.");
    let mut current_ordinal = 0;
    let mut previous_term: Option<(TermOrdinal, (u32, u32))> = None;
    for (row_idx, ord) in term_ords {
        if ord == NULL_TERM_ORDINAL {
            // NULL_TERM_ORDINAL sorts highest, so all remaining ords will have `None` views, and
            // be appended to the builder as null.
            break;
        }

        // only advance forward if the new ord is different than the one we just processed
        //
        // this allows the input TermOrdinal iterator to contain and reuse duplicates, so long as
        // it's still sorted
        match &previous_term {
            Some((previous_ord, previous_view)) if *previous_ord == ord => {
                // This is the same term ordinal: reuse the previous view.
                views[row_idx] = Some(*previous_view);
                continue;
            }
            // Fall through.
            _ => {}
        }

        // This is a new term ordinal: decode it and append it to the builder.
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

        // Move to ord inside that block
        for _ in current_ordinal..=ord {
            match current_sstable_delta_reader.advance() {
                Ok(true) => {}
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

        // Set the view for this row_idx.
        let offset: u32 = buffer
            .len()
            .try_into()
            .expect("Too many terms requested in `ords_to_string_array`");
        let len: u32 = bytes
            .len()
            .try_into()
            .expect("Single term is too long in `ords_to_string_array`");
        buffer.extend_from_slice(&bytes);
        previous_term = Some((ord, (offset, len)));
        views[row_idx] = Some((offset, len));
    }

    // Append all the rows' views to the builder.
    let block_no = builder.append_block(arrow_buffer::Buffer::from(buffer));
    for view in views {
        // Each view is an offset and len in our single block, or None for a null.
        match view {
            Some((offset, len)) => unsafe {
                builder.append_view_unchecked(block_no, offset, len);
            },
            None => builder.append_null(),
        }
    }

    Arc::new(builder.finish())
}
