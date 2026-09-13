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

use crate::index::fast_fields_helper::{FFHelper, WhichFastField};
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::catalog::OidExt;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::utils::{FieldSource, pg_search_extension_installed};
use crate::schema::SearchIndexSchema;
use pgrx::{PgList, PgMemoryContexts, PgOid, pg_guard, pg_sys};
use std::ptr::null_mut;

pub(crate) fn register_hook() {
    static mut PREV_HOOK: pg_sys::set_rel_pathlist_hook_type = None;

    #[pg_guard]
    unsafe extern "C-unwind" fn callback(
        root: *mut pg_sys::PlannerInfo,
        rel: *mut pg_sys::RelOptInfo,
        rti: pg_sys::Index,
        rte: *mut pg_sys::RangeTblEntry,
    ) {
        unsafe {
            if let Some(previous_hook) = PREV_HOOK {
                previous_hook(root, rel, rti, rte);
            }
            if !pg_sys::enable_indexonlyscan || !pg_search_extension_installed() {
                return;
            }

            let mut candidates = Vec::new();
            let mut partial_candidates = Vec::new();
            let paths = PgList::<pg_sys::Path>::from_pg((*rel).pathlist);
            let partial_paths = PgList::<pg_sys::Path>::from_pg((*rel).partial_pathlist);
            for path in paths.iter_ptr().chain(partial_paths.iter_ptr()) {
                if (*path).type_ != pg_sys::NodeTag::T_IndexPath
                    || (*path).pathtype != pg_sys::NodeTag::T_IndexScan
                {
                    continue;
                }
                let path = &*path.cast::<pg_sys::IndexPath>();
                let index = &*path.indexinfo;
                if index.hypothetical || !index.relam.is_paradedb_am() {
                    continue;
                }
                let Some(target) = index_only_target(root, path) else {
                    continue;
                };

                let required_outer = path
                    .path
                    .param_info
                    .as_ref()
                    .map_or(null_mut(), |params| params.ppi_req_outer);
                // Match PostgreSQL's get_loop_count, including unique-ified semijoin inputs.
                let loop_count = {
                    let relation = |id: i32| {
                        (id < (*root).simple_rel_array_size)
                            .then(|| *(*root).simple_rel_array.add(id as usize))
                            .filter(|rel| !rel.is_null() && !pg_sys::is_dummy_rel(*rel))
                    };
                    let mut loop_count = f64::INFINITY;
                    let mut outer_relid = pg_sys::bms_next_member(required_outer, -1);
                    while outer_relid >= 0 {
                        if let Some(outer_rel) = relation(outer_relid) {
                            let mut rows = (*outer_rel).rows;
                            for join in
                                PgList::<pg_sys::SpecialJoinInfo>::from_pg((*root).join_info_list)
                                    .iter_ptr()
                            {
                                if (*join).jointype != pg_sys::JoinType::JOIN_SEMI
                                    || !pg_sys::bms_is_member(
                                        (*rel).relid as i32,
                                        (*join).syn_lefthand,
                                    )
                                    || !pg_sys::bms_is_member(outer_relid, (*join).syn_righthand)
                                {
                                    continue;
                                }
                                let mut rhs_rows = 1.0;
                                let mut rhs_relid =
                                    pg_sys::bms_next_member((*join).syn_righthand, -1);
                                while rhs_relid >= 0 {
                                    if let Some(rhs_rel) = relation(rhs_relid) {
                                        rhs_rows *= (*rhs_rel).rows;
                                    }
                                    rhs_relid =
                                        pg_sys::bms_next_member((*join).syn_righthand, rhs_relid);
                                }
                                rows = rows.min(pg_sys::estimate_num_groups(
                                    root,
                                    (*join).semi_rhs_exprs,
                                    rhs_rows,
                                    null_mut(),
                                    null_mut(),
                                ));
                            }
                            loop_count = loop_count.min(rows);
                        }
                        outer_relid = pg_sys::bms_next_member(required_outer, outer_relid);
                    }
                    if loop_count.is_finite() && loop_count > 0.0 {
                        loop_count
                    } else {
                        1.0
                    }
                };
                let create_path = |partial| {
                    let candidate = pg_sys::create_index_path(
                        root,
                        path.indexinfo,
                        path.indexclauses,
                        path.indexorderbys,
                        path.indexorderbycols,
                        path.path.pathkeys,
                        path.indexscandir,
                        true,
                        required_outer,
                        loop_count,
                        partial,
                    );
                    (*candidate).path.pathtarget = target;
                    candidate
                };
                if !path.path.parallel_aware {
                    candidates.push(create_path(false));
                }
                // Index-only scans can qualify for workers even when heap scans do not.
                if index.amcanparallel && (*rel).consider_parallel && required_outer.is_null() {
                    let candidate = create_path(true);
                    if (*candidate).path.parallel_workers > 0 {
                        partial_candidates.push(candidate);
                    } else {
                        pg_sys::pfree(candidate.cast());
                    }
                }
            }

            // add_path can remove existing paths, so finish reading the pathlist first.
            for candidate in candidates {
                pg_sys::add_path(rel, candidate.cast());
            }
            for candidate in partial_candidates {
                pg_sys::add_partial_path(rel, candidate.cast());
            }
        }
    }

    unsafe {
        PREV_HOOK = pg_sys::set_rel_pathlist_hook;
        pg_sys::set_rel_pathlist_hook = Some(callback);
    }
}

// Exclude filters and output columns needed only by exact index conditions.
unsafe fn index_only_target(
    root: *mut pg_sys::PlannerInfo,
    path: &pg_sys::IndexPath,
) -> Option<*mut pg_sys::PathTarget> {
    unsafe {
        let index = &*path.indexinfo;
        let rel = &*index.rel;
        let target = &*path.path.pathtarget;
        let params = path.path.param_info.as_ref();
        let mut required = null_mut();
        let restrictions = PgList::<pg_sys::RestrictInfo>::from_pg(index.indrestrictinfo);
        let join_restrictions = PgList::<pg_sys::RestrictInfo>::from_pg(
            params.map_or(null_mut(), |params| params.ppi_clauses),
        );
        // Nonmovable join clauses can still need these columns above the scan.
        let remaining_joins =
            PgList::<pg_sys::RestrictInfo>::from_pg(params.map_or(null_mut(), |_| rel.joininfo));
        for restriction in restrictions
            .iter_ptr()
            .chain(join_restrictions.iter_ptr())
            .chain(remaining_joins.iter_ptr())
        {
            if !(*restriction).pseudoconstant
                && !pg_sys::is_redundant_with_indexclauses(restriction, path.indexclauses)
            {
                pg_sys::pull_varattnos((*restriction).clause.cast(), rel.relid, &mut required);
            }
        }

        // Index-only recheck expressions must also reference returnable columns.
        for clause in PgList::<pg_sys::IndexClause>::from_pg(path.indexclauses).iter_ptr() {
            for qual in PgList::<pg_sys::RestrictInfo>::from_pg((*clause).indexquals).iter_ptr() {
                pg_sys::pull_varattnos((*qual).clause.cast(), rel.relid, &mut required);
            }
        }

        // Join-only fallback Vars need not be emitted once their index conditions are enforced.
        let available = params.map_or(null_mut(), |params| {
            pg_sys::bms_union(rel.relids, params.ppi_req_outer)
        });
        let scan_target =
            params.map_or(path.path.pathtarget, |_| pg_sys::create_empty_pathtarget());
        let expressions = PgList::<pg_sys::Expr>::from_pg(target.exprs);
        for (i, expr) in expressions.iter_ptr().enumerate() {
            let sortgroupref = if target.sortgrouprefs.is_null() {
                0
            } else {
                *target.sortgrouprefs.add(i)
            };
            if params.is_some() && (*expr).type_ == pg_sys::NodeTag::T_Var && sortgroupref == 0 {
                let var = &*expr.cast::<pg_sys::Var>();
                if var.varno as u32 == rel.relid
                    && var.varlevelsup == 0
                    && !pg_sys::bms_is_member(
                        var.varattno as i32 - pg_sys::FirstLowInvalidHeapAttributeNumber,
                        required,
                    )
                    && pg_sys::bms_is_subset(
                        *rel.attr_needed.add((var.varattno - rel.min_attr) as usize),
                        available,
                    )
                {
                    continue;
                }
            }
            pg_sys::pull_varattnos(expr.cast(), rel.relid, &mut required);
            if params.is_some() {
                pg_sys::add_column_to_pathtarget(scan_target, expr, sortgroupref);
            }
        }
        pg_sys::bms_free(available);

        let mut returnable = null_mut();
        for column in 0..index.ncolumns as usize {
            let attno = *index.indexkeys.add(column);
            if attno > 0 && *index.canreturn.add(column) {
                returnable = pg_sys::bms_add_member(
                    returnable,
                    attno - pg_sys::FirstLowInvalidHeapAttributeNumber,
                );
            }
        }

        let covered = pg_sys::bms_is_subset(required, returnable);
        if covered && params.is_some() {
            pg_sys::set_pathtarget_cost_width(root, scan_target);
        }
        pg_sys::bms_free(required);
        pg_sys::bms_free(returnable);
        covered.then_some(scan_target)
    }
}

#[pg_guard]
pub(super) extern "C-unwind" fn amcanreturn(indexrel: pg_sys::Relation, attno: i32) -> bool {
    if attno <= 0 {
        return false;
    }

    unsafe {
        assert!(!indexrel.is_null());
        assert!(!(*indexrel).rd_att.is_null());
        let indexrel = PgSearchRelation::from_pg(indexrel);

        // A partitioned index has no physical storage to inspect. PostgreSQL asks each child
        // index separately whether it supports index-only scans.
        if pg_sys::get_rel_relkind(indexrel.oid()) as u8 == pg_sys::RELKIND_PARTITIONED_INDEX {
            return false;
        }

        AmCanReturnCache::get_or_init(indexrel.as_ptr()).can_return((attno - 1) as usize)
    }
}

struct IndexOnlyField {
    tuple_index: usize,
    fast_field: WhichFastField,
    pg_type: PgOid,
}

impl IndexOnlyField {
    fn from_schema(
        indexrel: &PgSearchRelation,
        schema: &SearchIndexSchema,
        tuple_index: usize,
    ) -> Option<Self> {
        let tuple_desc = indexrel.tuple_desc();
        let attribute = tuple_desc.get(tuple_index)?;
        if ![
            pg_sys::INT4OID,
            pg_sys::INT8OID,
            pg_sys::FLOAT4OID,
            pg_sys::FLOAT8OID,
            pg_sys::BOOLOID,
            pg_sys::UUIDOID,
        ]
        .contains(&attribute.atttypid)
        {
            return None;
        }

        let search_field = schema.search_field(attribute.name())?;
        let categorized = schema.categorized_fields();
        let data = categorized.iter().find_map(|(field, data)| {
            (field == &search_field && data.attno == tuple_index).then_some(data)
        })?;
        if !search_field.is_fast()
            || data.is_array
            || data.is_json
            || !matches!(data.source, FieldSource::Heap { .. })
        {
            return None;
        }

        Some(Self {
            tuple_index,
            fast_field: WhichFastField::Named(
                search_field.field_name().to_string(),
                search_field.field_type(),
            ),
            pg_type: PgOid::from(attribute.atttypid),
        })
    }
}

#[repr(C)]
struct AmCanReturnCache {
    returnable: [bool; pg_sys::INDEX_MAX_KEYS as usize],
}

impl AmCanReturnCache {
    unsafe fn get_or_init(indexrel: pg_sys::Relation) -> &'static Self {
        if (*indexrel).rd_amcache.is_null() {
            let relation = PgSearchRelation::from_pg(indexrel);
            let mut returnable = [false; pg_sys::INDEX_MAX_KEYS as usize];

            if let Ok(schema) = relation.schema() {
                let natts = relation.tuple_desc().len();
                for (tuple_index, can_return) in returnable.iter_mut().enumerate().take(natts) {
                    *can_return =
                        IndexOnlyField::from_schema(&relation, &schema, tuple_index).is_some();
                }
            }

            // PostgreSQL may call amcanreturn once per index attribute. Keep the capability
            // array in the relation's AM cache so those probes share one metadata read.
            let cache = pg_sys::MemoryContextAllocZero(
                (*indexrel).rd_indexcxt,
                std::mem::size_of::<Self>(),
            )
            .cast::<Self>();
            (*cache).returnable = returnable;
            (*indexrel).rd_amcache = cache.cast();
        }

        &*(*indexrel).rd_amcache.cast::<Self>()
    }

    fn can_return(&self, tuple_index: usize) -> bool {
        self.returnable.get(tuple_index).copied().unwrap_or(false)
    }
}

pub(super) struct IndexOnlyScanState {
    fast_fields: FFHelper,
    fields: Vec<IndexOnlyField>,
    values: Vec<pg_sys::Datum>,
    nulls: Vec<bool>,
    // The parent can delete this before dropping the scan state during error cleanup.
    tuple_context: PgMemoryContexts,
}

impl IndexOnlyScanState {
    pub(super) fn new(
        reader: &SearchIndexReader,
        indexrel: &PgSearchRelation,
        natts: usize,
    ) -> Self {
        let fields = (0..natts)
            .filter_map(|tuple_index| {
                IndexOnlyField::from_schema(indexrel, reader.schema(), tuple_index)
            })
            .collect::<Vec<_>>();
        let fast_fields = fields
            .iter()
            .map(|field| field.fast_field.clone())
            .collect::<Vec<_>>();

        Self {
            fast_fields: FFHelper::with_fields(reader, &fast_fields),
            fields,
            values: vec![pg_sys::Datum::null(); natts],
            nulls: vec![true; natts],
            tuple_context: unsafe {
                PgMemoryContexts::For(pg_sys::AllocSetContextCreateExtended(
                    pg_sys::CurrentMemoryContext,
                    c"pg_search index-only tuple".as_ptr(),
                    pg_sys::ALLOCSET_DEFAULT_MINSIZE as usize,
                    pg_sys::ALLOCSET_DEFAULT_INITSIZE as usize,
                    pg_sys::ALLOCSET_DEFAULT_MAXSIZE as usize,
                ))
            },
        }
    }

    pub(super) unsafe fn reset(&mut self) {
        self.tuple_context.reset();
    }

    pub(super) unsafe fn delete(self) {
        pg_sys::MemoryContextDelete(self.tuple_context.value());
    }

    pub(super) unsafe fn form_tuple(
        &mut self,
        tuple_desc: pg_sys::TupleDesc,
        doc_address: tantivy::DocAddress,
    ) -> pg_sys::HeapTuple {
        self.tuple_context.switch_to(|_| {
            self.nulls.fill(true);
            for (fast_field_index, field) in self.fields.iter().enumerate() {
                let value = self
                    .fast_fields
                    .value(fast_field_index, doc_address)
                    .expect("index-only field should be a fast field");
                match value
                    .try_into_datum(field.pg_type)
                    .expect("index-only field should convert to a Datum")
                {
                    Some(datum) => {
                        self.values[field.tuple_index] = datum;
                        self.nulls[field.tuple_index] = false;
                    }
                    None => self.values[field.tuple_index] = pg_sys::Datum::null(),
                }
            }

            pg_sys::heap_form_tuple(
                tuple_desc,
                self.values.as_mut_ptr(),
                self.nulls.as_mut_ptr(),
            )
        })
    }
}
