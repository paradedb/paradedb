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

use crate::api::operator::{
    anyelement_jsonb_opoid, estimate_selectivity, find_var_relation, ReturnedNodePointer,
};
use crate::index::SearchIndex;
use crate::postgres::index::open_search_index;
use crate::postgres::types::TantivyValue;
use crate::postgres::utils::locate_bm25_index;
use crate::query::SearchQueryInput;
use crate::{nodecast, UNKNOWN_SELECTIVITY};
use pgrx::{pg_extern, pg_sys, AnyElement, FromDatum, Internal, JsonB, PgList};

use super::{
    anyelement_query_input_opoid, anyelement_query_input_procoid,
    make_search_query_input_opexpr_node,
};
use rustc_hash::FxHashSet;
use std::ptr::NonNull;

#[pg_extern(immutable, parallel_safe, cost = 1000000000)]
pub fn search_with_search_config(
    _element: AnyElement,
    _config_json: JsonB,
    _fcinfo: pg_sys::FunctionCallInfo,
) -> bool {
    todo!("figure this out");
    //     let default_hash_set = || {
    //         let JsonB(search_config_json) = &config_json;
    //         let search_config: SearchConfig = serde_json::from_value(search_config_json.clone())
    //             .expect("could not parse search config");

    //         let search_index = open_search_index(unsafe {
    //             &PgRelation::with_lock(
    //                 pg_sys::Oid::from(search_config.index_oid),
    //                 pg_sys::AccessShareLock as pg_sys::LOCKMODE,
    //             )
    //         })
    //         .expect("should be able to open search index");

    //         let scan_state = search_index.get_reader().unwrap();
    //         let query = search_index.query(&search_config, &scan_state);
    //         let top_docs =
    //             scan_state.search_minimal(true, SearchIndex::executor(), &search_config, &query);
    //         let mut hs = FxHashSet::default();

    //         for (scored, _) in top_docs {
    //             check_for_interrupts!();
    //             hs.insert(scored.key.expect("key should have been retrieved"));
    //         }

    //         (search_config, hs)
    //     };

    //     let cached = unsafe { pg_func_extra(fcinfo, default_hash_set) };
    //     let search_config = &cached.0;
    //     let hash_set = &cached.1;

    //     let key_field_value = match unsafe {
    //         TantivyValue::try_from_datum(element.datum(), PgOid::from_untagged(element.oid()))
    //     } {
    //         Err(err) => panic!(
    //             "no value present in key_field {} in tuple: {err}",
    //             &search_config.key_field
    //         ),
    //         Ok(value) => value,
    //     };

    //     hash_set.contains(&key_field_value)
}

#[pg_extern(immutable, parallel_safe)]
pub unsafe fn search_config_support(arg: Internal) -> ReturnedNodePointer {
    search_config_support_request_simplify(arg).unwrap_or(ReturnedNodePointer(None))
}

fn search_config_support_request_simplify(arg: Internal) -> Option<ReturnedNodePointer> {
    unsafe {
        let srs = nodecast!(
            SupportRequestSimplify,
            T_SupportRequestSimplify,
            arg.unwrap()?.cast_mut_ptr::<pg_sys::Node>()
        )?;

        let mut input_args = PgList::<pg_sys::Node>::from_pg((*(*srs).fcall).args);
        let var = nodecast!(Var, T_Var, input_args.get_ptr(0)?)?;

        let rhs = input_args.get_ptr(1)?;
        let query = nodecast!(Const, T_Const, rhs)
            .and_then(|const_| JsonB::from_datum((*const_).constvalue, (*const_).constisnull))
            .map(|jsonb| {
                SearchQueryInput::from_json(&jsonb.0).expect("search query input should be valid")
            });

        Some(make_search_query_input_opexpr_node(
            srs,
            &mut input_args,
            var,
            query,
            anyelement_query_input_opoid(),
            anyelement_query_input_procoid(),
        ))
    }
}

#[pg_extern(immutable, parallel_safe)]
pub fn search_config_restrict(
    planner_info: Internal, // <pg_sys::PlannerInfo>,
    operator_oid: pg_sys::Oid,
    args: Internal, // <pg_sys::List>,
    _var_relid: i32,
) -> f64 {
    fn inner_jsonb(
        planner_info: Internal, // <pg_sys::PlannerInfo>,
        args: Internal,         // <pg_sys::List>,
    ) -> Option<f64> {
        unsafe {
            let info = planner_info.unwrap()?.cast_mut_ptr::<pg_sys::PlannerInfo>();
            let args =
                PgList::<pg_sys::Node>::from_pg(args.unwrap()?.cast_mut_ptr::<pg_sys::List>());

            let var = nodecast!(Var, T_Var, args.get_ptr(0)?)?;
            let const_ = nodecast!(Const, T_Const, args.get_ptr(1)?)?;
            let (heaprelid, _, _) = find_var_relation(var, info);
            let indexrel = locate_bm25_index(heaprelid)?;

            let jsonb = JsonB::from_datum((*const_).constvalue, (*const_).constisnull)?;
            let search_query_input =
                SearchQueryInput::from_json(&jsonb.0).expect("search query input should be valid");

            estimate_selectivity(&indexrel, &search_query_input)
        }
    }

    assert!(operator_oid == anyelement_jsonb_opoid());

    let mut selectivity = inner_jsonb(planner_info, args).unwrap_or(UNKNOWN_SELECTIVITY);

    if selectivity > 1.0 {
        selectivity = UNKNOWN_SELECTIVITY;
    }

    selectivity
}
