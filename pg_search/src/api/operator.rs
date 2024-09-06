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

use crate::env::needs_commit;
use crate::globals::WriterGlobal;
use crate::index::SearchIndex;
use crate::postgres::types::TantivyValue;
use crate::postgres::utils::locate_bm25_index;
use crate::schema::SearchConfig;
use crate::writer::WriterDirectory;
use lazy_static::lazy_static;
use pgrx::callconv::{BoxRet, FcInfo};
use pgrx::datum::Datum;
use pgrx::pg_sys::{lookup_type_cache, planner_rt_fetch, rt_fetch, Oid};
use pgrx::pgrx_sql_entity_graph::metadata::{
    ArgumentError, Returns, ReturnsError, SqlMapping, SqlTranslatable,
};
use pgrx::*;
use rustc_hash::FxHashSet;
use std::collections::HashMap;
use std::ptr::NonNull;
use std::sync::{Arc, LazyLock, Mutex, RwLock};

const UNKNOWN_SELECTIVITY: f64 = 0.00001;

#[repr(transparent)]
struct NodeWrapper(Option<NonNull<pg_sys::Node>>);

unsafe impl BoxRet for NodeWrapper {
    unsafe fn box_into<'fcx>(self, fcinfo: &mut FcInfo<'fcx>) -> Datum<'fcx> {
        self.0
            .map(|nonnull| {
                let datum = pg_sys::Datum::from(nonnull.as_ptr());
                fcinfo.return_raw_datum(datum)
            })
            .unwrap_or_else(|| Datum::null())
    }
}

unsafe impl SqlTranslatable for NodeWrapper {
    fn argument_sql() -> Result<SqlMapping, ArgumentError> {
        Ok(SqlMapping::As("internal".into()))
    }

    fn return_sql() -> Result<Returns, ReturnsError> {
        Ok(Returns::One(SqlMapping::As("internal".into())))
    }
}

#[pg_extern]
fn search_tantivy_text(element: AnyElement, query: &str, fcinfo: pg_sys::FunctionCallInfo) -> bool {
    let default_hash_set = || unsafe {
        let info = *planner_info_hack()
            .get(&(*(*fcinfo).flinfo).fn_expr)
            .expect("planner info for function node should exist in cache");

        let opexpr = (*(*fcinfo).flinfo).fn_expr.cast::<pg_sys::OpExpr>();
        let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);

        let (lhs, rhs) = (args.get_ptr(0)?, args.get_ptr(1)?);
        let search_config = (is_a(lhs, pg_sys::NodeTag::T_Var)
            && is_a(rhs, pg_sys::NodeTag::T_Const))
        .then(|| {
            let var = lhs.cast::<pg_sys::Var>();
            let const_ = rhs.cast::<pg_sys::Const>();

            let rte = planner_rt_fetch((*var).varno as pg_sys::Index, info);
            (!rte.is_null()).then(|| {
                let heaprelid = (*rte).relid;
                let indexrel = locate_bm25_index(heaprelid)?;

                let query = String::from_datum((*const_).constvalue, (*const_).constisnull)?;
                Some(SearchConfig::from((query, indexrel)))
            })
        })???;

        let writer_client = WriterGlobal::client();
        let directory = WriterDirectory::from_index_oid(search_config.index_oid);
        let search_index = SearchIndex::from_cache(&directory, &search_config.uuid)
            .unwrap_or_else(|err| panic!("error loading index from directory: {err}"));
        let scan_state = search_index
            .search_state(
                &writer_client,
                &search_config,
                needs_commit(search_config.index_oid),
            )
            .unwrap();
        let top_docs = scan_state.search(SearchIndex::executor());
        let mut hs = FxHashSet::default();

        for (scored, _) in top_docs {
            hs.insert(scored.key.expect("key should have been retrieved"));
        }

        Some((search_config, hs))
    };

    let cached = unsafe { pg_func_extra(fcinfo, default_hash_set) };
    match cached.as_ref() {
        None => panic!("oh no"),
        Some((search_config, hash_set)) => {
            let key_field_value = match unsafe {
                TantivyValue::try_from_datum(element.datum(), PgOid::from_untagged(element.oid()))
            } {
                Err(err) => panic!(
                    "no value present in key_field {} in tuple: {err}",
                    &search_config.key_field
                ),
                Ok(value) => value,
            };

            hash_set.contains(&key_field_value)
        }
    }
}

#[pg_extern]
fn search_tantivy(
    element: AnyElement,
    config_json: JsonB,
    fcinfo: pg_sys::FunctionCallInfo,
) -> bool {
    let default_hash_set = || {
        let JsonB(search_config_json) = &config_json;
        let search_config: SearchConfig = serde_json::from_value(search_config_json.clone())
            .expect("could not parse search config");

        let writer_client = WriterGlobal::client();
        let directory = WriterDirectory::from_index_oid(search_config.index_oid);
        let search_index = SearchIndex::from_cache(&directory, &search_config.uuid)
            .unwrap_or_else(|err| panic!("error loading index from directory: {err}"));
        let scan_state = search_index
            .search_state(
                &writer_client,
                &search_config,
                needs_commit(search_config.index_oid),
            )
            .unwrap();
        let top_docs = scan_state.search(SearchIndex::executor());
        let mut hs = FxHashSet::default();

        for (scored, _) in top_docs {
            hs.insert(scored.key.expect("key should have been retrieved"));
        }

        (search_config, hs)
    };

    let cached = unsafe { pg_func_extra(fcinfo, default_hash_set) };
    let search_config = &cached.0;
    let hash_set = &cached.1;

    let key_field_value = match unsafe {
        TantivyValue::try_from_datum(element.datum(), PgOid::from_untagged(element.oid()))
    } {
        Err(err) => panic!(
            "no value present in key_field {} in tuple: {err}",
            &search_config.key_field
        ),
        Ok(value) => value,
    };

    hash_set.contains(&key_field_value)
}

#[inline(always)]
fn planner_info_hack() -> &'static mut HashMap<*mut pg_sys::Node, *mut pg_sys::PlannerInfo> {
    unsafe {
        static mut PLANNER_INFO_HACK: Option<HashMap<*mut pg_sys::Node, *mut pg_sys::PlannerInfo>> =
            None;

        // SAFETY:  We're single threaded
        PLANNER_INFO_HACK.get_or_insert_with(|| Default::default())
    }
}

#[pg_extern(immutable, parallel_safe)]
fn search_tantivy_support(arg: Internal) -> NodeWrapper {
    unsafe {
        let node = arg.unwrap().unwrap().cast_mut_ptr::<pg_sys::Node>();

        pgrx::warning!("{:?}", (*node).type_);

        if is_a(node, pg_sys::NodeTag::T_SupportRequestCost) {
            let src = node.cast::<pg_sys::SupportRequestCost>();

            // our `search_tantivy_*` functions are *incredibly* expensive.  So much so that
            // we really don't ever want Postgres to prefer them.  As such, hardcode in some
            // big numbers.
            (*src).per_tuple = 1_000_000.0;

            planner_info_hack().insert((*src).node, (*src).root);
            register_xact_callback(PgXactCallbackEvent::Commit, || planner_info_hack().clear());
            register_xact_callback(PgXactCallbackEvent::Abort, || planner_info_hack().clear());
            NodeWrapper(NonNull::new(node))
        } else {
            NodeWrapper(None)
        }
    }
}

#[pg_extern(immutable, parallel_safe)]
fn search_tantivy_restrict(
    planner_info: Internal, // <pg_sys::PlannerInfo>,
    operator_oid: pg_sys::Oid,
    args: Internal, // <pg_sys::List>,
    var_relid: i32,
) -> f64 {
    fn estimate_selectivity(heaprelid: Oid, search_config: &SearchConfig) -> Option<f64> {
        let directory = WriterDirectory::from_index_oid(search_config.index_oid);
        let search_index = SearchIndex::from_cache(&directory, &search_config.uuid)
            .unwrap_or_else(|err| panic!("error loading index from directory: {err}"));
        let writer_client = WriterGlobal::client();
        let state = search_index
            .search_state(&writer_client, &search_config, false)
            .expect("SearchState creation should not fail");

        let reltuples = unsafe { PgRelation::open(heaprelid).reltuples().unwrap_or(1.0) as f64 };
        let estimate = state.estimate_docs().unwrap_or(1) as f64;

        let mut selectivity = estimate / reltuples;
        if selectivity > 1.0 {
            selectivity = 1.0;
        }

        Some(selectivity)
    }

    fn inner_text(
        planner_info: Internal, // <pg_sys::PlannerInfo>,
        args: Internal,         // <pg_sys::List>,
    ) -> Option<f64> {
        unsafe {
            let info = planner_info.unwrap()?.cast_mut_ptr::<pg_sys::PlannerInfo>();
            let args =
                PgList::<pg_sys::Node>::from_pg(args.unwrap()?.cast_mut_ptr::<pg_sys::List>());

            let (lhs, rhs) = (args.get_ptr(0)?, args.get_ptr(1)?);
            if is_a(lhs, pg_sys::NodeTag::T_Var) && is_a(rhs, pg_sys::NodeTag::T_Const) {
                let var = lhs.cast::<pg_sys::Var>();
                let const_ = rhs.cast::<pg_sys::Const>();

                let rte = planner_rt_fetch((*var).varno as pg_sys::Index, info);
                if !rte.is_null() {
                    let heaprelid = (*rte).relid;
                    let indexrel = locate_bm25_index(heaprelid)?;

                    let query = String::from_datum((*const_).constvalue, (*const_).constisnull)?;
                    let search_config = SearchConfig::from((query, indexrel));

                    return estimate_selectivity(heaprelid, &search_config);
                }
            }

            None
        }
    }
    fn inner_jsonb(
        planner_info: Internal, // <pg_sys::PlannerInfo>,
        args: Internal,         // <pg_sys::List>,
    ) -> Option<f64> {
        unsafe {
            let info = planner_info.unwrap()?.cast_mut_ptr::<pg_sys::PlannerInfo>();
            let args =
                PgList::<pg_sys::Node>::from_pg(args.unwrap()?.cast_mut_ptr::<pg_sys::List>());

            let (lhs, rhs) = (args.get_ptr(0)?, args.get_ptr(1)?);
            if is_a(lhs, pg_sys::NodeTag::T_Var) && is_a(rhs, pg_sys::NodeTag::T_Const) {
                let var = lhs.cast::<pg_sys::Var>();
                let const_ = rhs.cast::<pg_sys::Const>();

                let rte = planner_rt_fetch((*var).varno as pg_sys::Index, info);
                if !rte.is_null() {
                    let search_config_jsonb =
                        JsonB::from_datum((*const_).constvalue, (*const_).constisnull)?;
                    let search_config = SearchConfig::from_jsonb(search_config_jsonb)
                        .expect("SearchConfig should be valid");

                    return estimate_selectivity(
                        pg_sys::Oid::from(search_config.table_oid),
                        &search_config,
                    );
                }
            }

            None
        }
    }

    unsafe {
        let textopid = direct_function_call::<pg_sys::Oid>(
            pg_sys::regoperatorin,
            &[c"@@@(anyelement, text)".into_datum()],
        )
        .expect("the `@@@(anyelement, text)` operator should exist");

        let jsonbopid = direct_function_call::<pg_sys::Oid>(
            pg_sys::regoperatorin,
            &[c"@@@(anyelement, jsonb)".into_datum()],
        )
        .expect("the `@@@(anyelement, jsonb)` operator should exist");

        let mut selectivity = if textopid == operator_oid {
            inner_text(planner_info, args).unwrap_or(UNKNOWN_SELECTIVITY)
        } else if jsonbopid == operator_oid {
            inner_jsonb(planner_info, args).unwrap_or(UNKNOWN_SELECTIVITY)
        } else {
            UNKNOWN_SELECTIVITY
        };

        if selectivity > 1.0 {
            selectivity = UNKNOWN_SELECTIVITY;
        }

        selectivity
    }
}

extension_sql!(
    r#"
ALTER FUNCTION paradedb.search_tantivy SUPPORT paradedb.search_tantivy_support;
ALTER FUNCTION paradedb.search_tantivy_text SUPPORT paradedb.search_tantivy_support;


CREATE OPERATOR pg_catalog.@@@ (
    PROCEDURE = search_tantivy,
    LEFTARG = anyelement,
    RIGHTARG = jsonb,
    RESTRICT = search_tantivy_restrict
);

CREATE OPERATOR pg_catalog.@@@ (
    PROCEDURE = search_tantivy_text,
    LEFTARG = anyelement,
    RIGHTARG = text,
    RESTRICT = search_tantivy_restrict
);

CREATE OPERATOR CLASS anyelement_bm25_ops DEFAULT FOR TYPE anyelement USING bm25 AS
    OPERATOR 1 pg_catalog.@@@(anyelement, jsonb),   /* for querying with a full SearchConfig jsonb object */
    OPERATOR 2 pg_catalog.@@@(anyelement, text),    /* for querying with a tantivy-compatible text query */
    STORAGE anyelement;

"#,
    name = "bm25_ops_anyelement_operator",
    requires = [
        search_tantivy,
        search_tantivy_text,
        search_tantivy_support,
        search_tantivy_restrict
    ]
);
