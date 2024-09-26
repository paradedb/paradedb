// Copyright (c) 2023-2024 Retake, Inc.
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

mod searchconfig;
mod searchqueryinput;
mod text;

use crate::index::SearchIndex;
use crate::index::WriterDirectory;
use crate::postgres::utils::locate_bm25_index;
use crate::query::SearchQueryInput;
use crate::schema::SearchConfig;
use pgrx::callconv::{BoxRet, FcInfo};
use pgrx::datum::Datum;
use pgrx::pgrx_sql_entity_graph::metadata::{
    ArgumentError, Returns, ReturnsError, SqlMapping, SqlTranslatable,
};
use pgrx::*;
use std::ffi::CStr;
use std::ptr::NonNull;

#[repr(transparent)]
pub struct ReturnedNodePointer(Option<NonNull<pg_sys::Node>>);

unsafe impl BoxRet for ReturnedNodePointer {
    unsafe fn box_into<'fcx>(self, fcinfo: &mut FcInfo<'fcx>) -> Datum<'fcx> {
        self.0
            .map(|nonnull| {
                let datum = pg_sys::Datum::from(nonnull.as_ptr());
                fcinfo.return_raw_datum(datum)
            })
            .unwrap_or_else(Datum::null)
    }
}

unsafe impl SqlTranslatable for ReturnedNodePointer {
    fn argument_sql() -> Result<SqlMapping, ArgumentError> {
        Ok(SqlMapping::As("internal".into()))
    }

    fn return_sql() -> Result<Returns, ReturnsError> {
        Ok(Returns::One(SqlMapping::As("internal".into())))
    }
}

fn anyelement_jsonb_procoid() -> pg_sys::Oid {
    unsafe {
        direct_function_call::<pg_sys::Oid>(
            pg_sys::regprocedurein,
            &[c"paradedb.search_with_search_config(anyelement, jsonb)".into_datum()],
        )
        .expect("the `paradedb.search_with_search_config(anyelement, jsonb) function should exist")
    }
}

fn anyelement_text_opoid() -> pg_sys::Oid {
    unsafe {
        direct_function_call::<pg_sys::Oid>(
            pg_sys::regoperatorin,
            &[c"@@@(anyelement, text)".into_datum()],
        )
        .expect("the `@@@(anyelement, text)` operator should exist")
    }
}

fn anyelement_query_input_opoid() -> pg_sys::Oid {
    unsafe {
        direct_function_call::<pg_sys::Oid>(
            pg_sys::regoperatorin,
            &[c"@@@(anyelement, paradedb.searchqueryinput)".into_datum()],
        )
        .expect("the `@@@(anyelement, paradedb.searchqueryinput)` operator should exist")
    }
}

fn anyelement_jsonb_opoid() -> pg_sys::Oid {
    unsafe {
        direct_function_call::<pg_sys::Oid>(
            pg_sys::regoperatorin,
            &[c"@@@(anyelement, jsonb)".into_datum()],
        )
        .expect("the `@@@(anyelement, jsonb)` operator should exist")
    }
}

fn estimate_selectivity(
    heaprelid: pg_sys::Oid,
    relfilenode: pg_sys::Oid,
    search_config: &SearchConfig,
) -> Option<f64> {
    let reltuples = unsafe { PgRelation::open(heaprelid).reltuples().unwrap_or(1.0) as f64 };
    if !reltuples.is_normal() || reltuples.is_sign_negative() {
        // we can't estimate against a non-normal or negative estimate of heap tuples
        return None;
    }

    let directory = WriterDirectory::from_oids(
        search_config.database_oid,
        search_config.index_oid,
        relfilenode.as_u32(),
    );
    let search_index = SearchIndex::from_cache(&directory, &search_config.uuid)
        .unwrap_or_else(|err| panic!("error loading index from directory: {err}"));
    let state = search_index
        .search_state(search_config)
        .expect("SearchState creation should not fail");
    let estimate = state.estimate_docs().unwrap_or(1) as f64;

    let mut selectivity = estimate / reltuples;
    if selectivity > 1.0 {
        selectivity = 1.0;
    }

    Some(selectivity)
}

unsafe fn make_search_config_opexpr_node(
    srs: *mut pg_sys::SupportRequestSimplify,
    input_args: &mut PgList<pg_sys::Node>,
    var: *mut pg_sys::Var,
    query: SearchQueryInput,
) -> ReturnedNodePointer {
    // we're about to fabricate a new pg_sys::OpExpr node to return
    // that represents the `@@@(anyelement, jsonb)` operator
    let mut newopexpr = pg_sys::OpExpr {
        xpr: pg_sys::Expr {
            type_: pg_sys::NodeTag::T_OpExpr,
        },
        opno: anyelement_jsonb_opoid(),
        opfuncid: anyelement_jsonb_procoid(),
        opresulttype: pg_sys::BOOLOID,
        opretset: false,
        opcollid: pg_sys::DEFAULT_COLLATION_OID,
        inputcollid: pg_sys::DEFAULT_COLLATION_OID,
        args: std::ptr::null_mut(),
        location: (*(*srs).fcall).location,
    };

    let (relid, varattno) = find_var_relation(var, (*srs).root);
    if relid == pg_sys::Oid::INVALID {
        panic!("could not determine relation for var");
    }

    // we need to use what should be the only `USING bm25` index on the table
    let heaprel = PgRelation::open(relid);
    let indexrel = locate_bm25_index(relid).unwrap_or_else(|| {
        panic!(
            "relation `{}.{}` must have a `USING bm25` index",
            heaprel.namespace(),
            heaprel.name()
        )
    });

    let keys = &(*indexrel.rd_index).indkey;
    let keys = keys.values.as_slice(keys.dim1 as usize);
    if keys[0] != varattno {
        panic!("left-hand side of the @@@ operator must match the first column of the only `USING bm25` index");
    }

    // fabricate a `SearchConfig` from the above relation and query string
    // and get it serialized into a JSONB Datum
    let search_config = SearchConfig::from((query, indexrel));

    let search_config_json =
        serde_json::to_value(&search_config).expect("SearchConfig should serialize to json");
    let jsonb_datum = JsonB(search_config_json).into_datum().unwrap();

    // from which we'll create a new pg_sys::Const node
    let jsonb_const = pg_sys::makeConst(
        pg_sys::JSONBOID,
        -1,
        pg_sys::DEFAULT_COLLATION_OID,
        -1,
        jsonb_datum,
        false,
        false,
    );

    // and assign it to the original argument list
    input_args.replace_ptr(1, jsonb_const.cast());

    // then assign that list to our new OpExpr node
    newopexpr.args = input_args.as_ptr();

    // copy that node into the current memory context and return it
    let node = PgMemoryContexts::CurrentMemoryContext
        .copy_ptr_into(&mut newopexpr, std::mem::size_of::<pg_sys::OpExpr>());

    ReturnedNodePointer(NonNull::new(node.cast()))
}

/// Given a [`pg_sys::Var`] and a [`pg_sys::PlannerInfo`], attempt to find the relation Oid that
/// contains the var.
///
/// It's possible the returned Oid will be [`pg_sys::Oid::INVALID`] if the Var doesn't eventually
/// come from a relation relation.
///
/// The returned [`pg_sys::AttrNumber`] is the physical attribute number in the relation the Var
/// is from.
unsafe fn find_var_relation(
    var: *mut pg_sys::Var,
    root: *mut pg_sys::PlannerInfo,
) -> (pg_sys::Oid, pg_sys::AttrNumber) {
    let query = (*root).parse;
    let rte = pg_sys::rt_fetch((*var).varno as pg_sys::Index, (*query).rtable);

    match (*rte).rtekind {
        // the Var comes from a relation
        pg_sys::RTEKind::RTE_RELATION => ((*rte).relid, (*var).varattno),

        // the Var comes from a subquery, so dig into its target list and find the original
        // table it comes from along with its original column AttributeNumber
        pg_sys::RTEKind::RTE_SUBQUERY => {
            let targetlist = PgList::<pg_sys::TargetEntry>::from_pg((*(*rte).subquery).targetList);
            let te = targetlist
                .get_ptr((*var).varattno as usize - 1)
                .expect("var should exist in subquery TargetList");
            ((*te).resorigtbl, (*te).resorigcol)
        }

        // the Var comes from a CTE, so lookup that CTE and find it in the CTE's target list
        pg_sys::RTEKind::RTE_CTE => {
            let mut levelsup = (*rte).ctelevelsup;
            let mut cteroot = root;
            while levelsup > 0 {
                cteroot = (*cteroot).parent_root;
                if cteroot.is_null() {
                    // shouldn't happen
                    panic!(
                        "bad levelsup for CTE \"{}\"",
                        CStr::from_ptr((*rte).ctename).to_string_lossy()
                    )
                }
                levelsup -= 1;
            }

            let rte_ctename = CStr::from_ptr((*rte).ctename);
            let ctelist = PgList::<pg_sys::CommonTableExpr>::from_pg((*(*cteroot).parse).cteList);
            let mut matching_cte = None;
            for cte in ctelist.iter_ptr() {
                let ctename = CStr::from_ptr((*cte).ctename);

                if ctename == rte_ctename {
                    matching_cte = Some(cte);
                    break;
                }
            }

            let cte = matching_cte.unwrap_or_else(|| {
                panic!(
                    "unable to find cte named \"{}\"",
                    rte_ctename.to_string_lossy()
                )
            });

            if !is_a((*cte).ctequery, pg_sys::NodeTag::T_Query) {
                panic!("CTE is not a query")
            }
            let query = (*cte).ctequery.cast::<pg_sys::Query>();
            let targetlist = if !(*query).returningList.is_null() {
                PgList::<pg_sys::TargetEntry>::from_pg((*query).returningList)
            } else {
                PgList::<pg_sys::TargetEntry>::from_pg((*query).targetList)
            };
            let te = targetlist
                .get_ptr((*var).varattno as usize - 1)
                .expect("var should exist in cte TargetList");

            ((*te).resorigtbl, (*te).resorigcol)
        }
        _ => panic!("unsupported RTEKind: {}", (*rte).rtekind),
    }
}

extension_sql!(
    r#"
ALTER FUNCTION paradedb.search_with_search_config SUPPORT paradedb.search_config_support;
ALTER FUNCTION paradedb.search_with_text SUPPORT paradedb.text_support;
ALTER FUNCTION paradedb.search_with_query_input SUPPORT paradedb.query_input_support;

CREATE OPERATOR pg_catalog.@@@ (
    PROCEDURE = search_with_search_config,
    LEFTARG = anyelement,
    RIGHTARG = jsonb,
    RESTRICT = search_config_restrict
);

CREATE OPERATOR pg_catalog.@@@ (
    PROCEDURE = search_with_text,
    LEFTARG = anyelement,
    RIGHTARG = text,
    RESTRICT = text_restrict
);

CREATE OPERATOR pg_catalog.@@@ (
    PROCEDURE = search_with_query_input,
    LEFTARG = anyelement,
    RIGHTARG = paradedb.searchqueryinput,
    RESTRICT = query_input_restrict
);

CREATE OPERATOR CLASS anyelement_bm25_ops DEFAULT FOR TYPE anyelement USING bm25 AS
    OPERATOR 1 pg_catalog.@@@(anyelement, jsonb),                        /* for querying with a full SearchConfig jsonb object */
    OPERATOR 2 pg_catalog.@@@(anyelement, text),                         /* for querying with a tantivy-compatible text query */
    OPERATOR 3 pg_catalog.@@@(anyelement, paradedb.searchqueryinput),    /* for querying with a paradedb.searchqueryinput structure */
    STORAGE anyelement;

"#,
    name = "bm25_ops_anyelement_operator",
    requires = [
        // for using a SearchConfig on the rhs
        searchconfig::search_with_search_config,
        searchconfig::search_config_restrict,
        searchconfig::search_config_support,
        // for using plain text on the rhs
        text::search_with_text,
        text::text_restrict,
        text::text_support,
        // for using SearchQueryInput on the rhs
        searchqueryinput::search_with_query_input,
        searchqueryinput::query_input_restrict,
        searchqueryinput::query_input_support,
    ]
);
