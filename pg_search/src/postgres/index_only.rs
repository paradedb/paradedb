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
use crate::postgres::build::is_bm25_index;
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
            for path in PgList::<pg_sys::Path>::from_pg((*rel).pathlist).iter_ptr() {
                if (*path).type_ != pg_sys::NodeTag::T_IndexPath
                    || (*path).pathtype != pg_sys::NodeTag::T_IndexScan
                    || !(*path).param_info.is_null()
                    || (*path).parallel_aware
                {
                    continue;
                }
                let path = &*path.cast::<pg_sys::IndexPath>();
                let index = &*path.indexinfo;
                if index.hypothetical
                    || !is_bm25_index(&PgSearchRelation::open(index.indexoid))
                    || !covers_required_attributes(path)
                {
                    continue;
                }

                candidates.push(pg_sys::create_index_path(
                    root,
                    path.indexinfo,
                    path.indexclauses,
                    path.indexorderbys,
                    path.indexorderbycols,
                    path.path.pathkeys,
                    path.indexscandir,
                    true,
                    null_mut(),
                    1.0,
                    false,
                ));
            }

            // add_path can remove existing paths, so finish reading the pathlist first.
            for candidate in candidates {
                pg_sys::add_path(rel, candidate.cast());
            }
        }
    }

    unsafe {
        PREV_HOOK = pg_sys::set_rel_pathlist_hook;
        pg_sys::set_rel_pathlist_hook = Some(callback);
    }
}

// Like PostgreSQL's check_index_only, but exclude filters enforced by exact index conditions.
unsafe fn covers_required_attributes(path: &pg_sys::IndexPath) -> bool {
    unsafe {
        let index = &*path.indexinfo;
        let rel = &*index.rel;
        let mut required = null_mut();
        pg_sys::pull_varattnos((*rel.reltarget).exprs.cast(), rel.relid, &mut required);

        for restriction in PgList::<pg_sys::RestrictInfo>::from_pg(index.indrestrictinfo).iter_ptr()
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
        pg_sys::bms_free(required);
        pg_sys::bms_free(returnable);
        covered
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
    returnable: u32,
}

impl AmCanReturnCache {
    unsafe fn get_or_init(indexrel: pg_sys::Relation) -> &'static Self {
        if (*indexrel).rd_amcache.is_null() {
            let relation = PgSearchRelation::from_pg(indexrel);
            let mut returnable = 0;

            if let Ok(schema) = relation.schema() {
                let natts = relation.tuple_desc().len();
                for tuple_index in 0..natts {
                    if IndexOnlyField::from_schema(&relation, &schema, tuple_index).is_some() {
                        returnable |= 1 << tuple_index;
                    }
                }
            }

            // PostgreSQL may call amcanreturn once per index attribute. Keep the capability
            // mask in the relation's AM cache so those probes share one metadata read.
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
        tuple_index < pg_sys::INDEX_MAX_KEYS as usize && self.returnable & (1 << tuple_index) != 0
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
