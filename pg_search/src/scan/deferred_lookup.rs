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

//! Pieces shared by the two halves of a deferred column lookup.
//!
//! A deferred string/bytes column moves through three states: a packed doc address
//! (State 0), a term ordinal in one segment's dictionary (State 1), and the decoded
//! `Utf8View` / `BinaryView` value. `TantivyFetchExec` takes State 0 to State 1 and
//! `TantivyDecodeExec` takes State 1 to the value. The two run as separate nodes because
//! a fast-field fetch wants doc order while a dictionary decode is random access either
//! way, so a planner can place each where it costs least.

use std::sync::Arc;

use crate::api::{HashMap, HashSet};
use crate::index::fast_fields_helper::{CanonicalColumn, FFHelper, WhichFastField};
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::rel::PgSearchRelation;
use crate::query::SearchQueryInput;
use crate::scan::late_materialization::{DeferredLookupRebuild, FFHelperKey};

use arrow_schema::{DataType, SchemaRef};
use datafusion::common::{DataFusionError, Result};
use datafusion::physical_expr::expressions::Column;
use datafusion::physical_expr::{
    EquivalenceProperties, LexOrdering, PhysicalExpr, PhysicalSortExpr,
};
use datafusion::physical_plan::ExecutionPlan;

/// Tracks a deferred column inside DataFusion's physical execution plan.
///
/// Unlike the logical `DeferredField` which uses the base column's string name, this struct
/// identifies the column strictly by its `usize` index within the physical `RecordBatch`.
/// This is necessary because DataFusion physical schemas (`arrow_schema::Schema`) drop
/// all relation qualifiers and names are no longer used for strict identity.
///
/// The `display_name` is preserved purely for `EXPLAIN` rendering and debugging; it should
/// never be used for matching columns in the physical plan.
#[derive(
    Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct PhysicalDeferredField {
    /// The positional index of the column in the physical Arrow schema
    pub col_idx: usize,
    /// A human-readable name used purely for `EXPLAIN` formatting
    pub display_name: String,
    pub is_bytes: bool,
    pub canonical: CanonicalColumn,
    /// The range table index of the scan this column comes from. See [`DeferredField`].
    ///
    /// [`DeferredField`]: crate::scan::late_materialization::DeferredField
    pub heap_rti: u32,
    /// JoinScan source identity. Distinguishes self-join aliases that share
    /// `canonical.indexrelid` so each side keeps its own `FFHelper` (#6023).
    #[serde(default)]
    pub plan_position: Option<usize>,
    #[serde(default)]
    pub rebuild: Option<DeferredLookupRebuild>,
}

impl PhysicalDeferredField {
    pub fn helper_key(&self) -> FFHelperKey {
        (self.plan_position, self.canonical.indexrelid)
    }

    pub fn output_data_type(&self) -> DataType {
        if self.is_bytes {
            DataType::BinaryView
        } else {
            DataType::Utf8View
        }
    }
}

/// The fast-field helper that reads a deferred column's index.
pub(crate) fn ffhelper_for<'a>(
    ffhelpers: &'a HashMap<FFHelperKey, Arc<FFHelper>>,
    field: &PhysicalDeferredField,
) -> Result<&'a Arc<FFHelper>> {
    ffhelpers.get(&field.helper_key()).ok_or_else(|| {
        DataFusionError::Execution(format!(
            "missing FFHelper for plan_position {:?} relation ID {}",
            field.plan_position, field.canonical.indexrelid
        ))
    })
}

/// Which snapshot a rebuilt fast-field reader reads. Both decode paths reach the same segments
/// in the same order the addresses were packed against, they just resolve the set differently.
#[derive(Clone, Copy)]
pub(crate) struct LookupRebuildContext {
    pub parallel_state: Option<*mut crate::postgres::ParallelScanState>,
}

/// Resolve the segment view a rebuilt helper opens for one deferred column's index.
pub(crate) fn rebuild_mvcc(
    context: LookupRebuildContext,
    rebuild: &DeferredLookupRebuild,
) -> Result<MvccSatisfies> {
    if let Some(source_idx) = rebuild.source_idx {
        let ps = context.parallel_state.ok_or_else(|| {
            DataFusionError::Internal(
                "ffhelper rebuild: parallel scan requires a ParallelScanState".into(),
            )
        })?;
        Ok(MvccSatisfies::ParallelWorker(unsafe {
            (*ps).segment_view_for_source(source_idx)
        }))
    } else {
        Ok(MvccSatisfies::Snapshot)
    }
}

/// Open a fast-field helper for `indexrelid` with each rebuild entry laid out at its original
/// `ff_index` (`Junk` fills the gaps), over the segment view `mvcc` picks.
pub(crate) fn open_rebuilt_ffhelper(
    indexrelid: u32,
    entries: &[(usize, &DeferredLookupRebuild)],
    mvcc: MvccSatisfies,
) -> Result<Arc<FFHelper>> {
    let index_rel = PgSearchRelation::open(pgrx::pg_sys::Oid::from(indexrelid));
    let reader = SearchIndexReader::open_with_context(
        &index_rel,
        SearchQueryInput::All,
        /* need_scores */ false,
        mvcc,
        None,
        None,
        /* needs_tokenizer_manager */ false,
    )
    .map_err(|e| DataFusionError::Internal(format!("ffhelper rebuild: open reader: {e}")))?;

    let width = entries.iter().map(|(i, _)| i + 1).max().unwrap_or(0);
    let mut which: Vec<WhichFastField> = vec![WhichFastField::Junk(String::new()); width];
    for (ff_index, rb) in entries {
        which[*ff_index] = WhichFastField::Named(rb.field_name.clone(), rb.field_type);
    }
    Ok(Arc::new(FFHelper::with_fields(&reader, &which)))
}

/// Rebuild the fast-field readers for deferred columns whose scan lives in a different plan
/// fragment (a lookup above a network shuffle finds no scan in its decoded subtree). The
/// `context` picks how the segment set is resolved; either way the reader's segment ordering
/// matches the ordering the addresses were packed against.
pub(crate) fn rebuild_missing_ffhelpers(
    deferred_fields: &[PhysicalDeferredField],
    ffhelpers: &mut HashMap<FFHelperKey, Arc<FFHelper>>,
    context: LookupRebuildContext,
) -> Result<()> {
    // Ordinal-typed columns keep a scan's helper when one decoded in this fragment (its layout
    // lines up by construction); they rebuild only on a worker whose fragment has that scan
    // behind a network boundary.
    let mut rebuild_keys: HashSet<FFHelperKey> = Default::default();
    for f in deferred_fields {
        if f.rebuild.is_none() {
            continue;
        }
        let key = f.helper_key();
        if !ffhelpers.contains_key(&key) {
            rebuild_keys.insert(key);
        }
    }

    // Group by `(plan_position, indexrelid)` so two columns of the same alias share one
    // reader, while a self-join's two aliases keep separate layouts. The rebuilt helper
    // replaces the map entry, so it has to serve every rebuildable column of that alias.
    let mut per_key: HashMap<FFHelperKey, Vec<&PhysicalDeferredField>> = HashMap::default();
    for f in deferred_fields {
        let key = f.helper_key();
        if f.rebuild.is_some() && rebuild_keys.contains(&key) {
            per_key.entry(key).or_default().push(f);
        }
    }

    for (key, fields) in per_key {
        let mvcc = rebuild_mvcc(context, fields[0].rebuild.as_ref().unwrap())?;
        let entries: Vec<(usize, &DeferredLookupRebuild)> = fields
            .iter()
            .map(|f| (f.canonical.ff_index, f.rebuild.as_ref().unwrap()))
            .collect();
        ffhelpers.insert(key, open_rebuilt_ffhelper(key.1, &entries, mvcc)?);
    }
    Ok(())
}

/// Carries the input's output ordering onto `output_schema` for a node that keeps row order
/// and only changes column types, so DataFusion does not add a sort above it.
pub(crate) fn preserved_ordering(
    input: &Arc<dyn ExecutionPlan>,
    output_schema: SchemaRef,
) -> EquivalenceProperties {
    let mut eq_props = EquivalenceProperties::new(output_schema.clone());
    if let Some(input_ordering) = input.properties().output_ordering() {
        let rewritten: Vec<_> = input_ordering
            .iter()
            .filter_map(|sort_expr| {
                let col = sort_expr.expr.downcast_ref::<Column>()?;
                if col.index() >= output_schema.fields().len() {
                    return None;
                }
                Some(PhysicalSortExpr {
                    expr: Arc::new(Column::new(col.name(), col.index())) as Arc<dyn PhysicalExpr>,
                    options: sort_expr.options,
                })
            })
            .collect();
        if rewritten.len() == input_ordering.len()
            && let Some(lex) = LexOrdering::new(rewritten)
        {
            eq_props.add_ordering(lex);
        }
    }
    eq_props
}
