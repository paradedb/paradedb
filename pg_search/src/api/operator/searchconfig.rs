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
use crate::globals::WriterGlobal;
use crate::index::SearchIndex;
use crate::postgres::types::TantivyValue;
use crate::postgres::utils::relfilenode_from_index_oid;
use crate::postgres::utils::{locate_bm25_index, relfilenode_from_pg_relation};
use crate::schema::SearchConfig;
use crate::writer::WriterDirectory;
use crate::{GUCS, UNKNOWN_SELECTIVITY};
use pgrx::{
    check_for_interrupts, is_a, pg_extern, pg_func_extra, pg_sys, AnyElement, FromDatum, Internal,
    JsonB, PgList, PgOid,
};
use rustc_hash::FxHashSet;
use shared::gucs::GlobalGucSettings;
use std::ptr::NonNull;

#[pg_extern(immutable, parallel_safe)]
pub fn search_with_search_config(
    element: AnyElement,
    config_json: JsonB,
    fcinfo: pg_sys::FunctionCallInfo,
) -> bool {
    let default_hash_set = || {
        let JsonB(search_config_json) = &config_json;
        let search_config: SearchConfig = serde_json::from_value(search_config_json.clone())
            .expect("could not parse search config");

        let index_oid = search_config.index_oid;
        let database_oid = search_config.database_oid;
        let relfilenode = relfilenode_from_index_oid(index_oid);

        let writer_client = WriterGlobal::client();
        let directory = WriterDirectory::from_oids(database_oid, index_oid, relfilenode.as_u32());
        let search_index = SearchIndex::from_cache(&directory, &search_config.uuid)
            .unwrap_or_else(|err| panic!("error loading index from directory: {err}"));
        let scan_state = search_index
            .search_state(&writer_client, &search_config)
            .unwrap();
        let top_docs = scan_state.search_minimal(true, SearchIndex::executor());
        let mut hs = FxHashSet::default();

        for (scored, _) in top_docs {
            check_for_interrupts!();
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

#[pg_extern(immutable, parallel_safe)]
pub fn search_config_support(arg: Internal) -> ReturnedNodePointer {
    unsafe {
        let node = arg.unwrap().unwrap().cast_mut_ptr::<pg_sys::Node>();

        match (*node).type_ {
            // our `search_with_*` functions are *incredibly* expensive.  So much so that
            // we really don't ever want Postgres to prefer them.
            //
            // The higher the `per_tuple` cost is here, the better.
            pg_sys::NodeTag::T_SupportRequestCost => {
                let src = node.cast::<pg_sys::SupportRequestCost>();

                // it can cost a lot to startup the `@@@` operator outside of an IndexScan because
                // ultimately we have to hash all the resulting ctids in memory.  For lack of a better
                // value, we say it costs as much as the `GUCS.per_tuple_cost()`.  This is an arbitrary
                // number that we've documented as needing to be big.
                (*src).startup = GUCS.per_tuple_cost();

                // similarly, use the same GUC here.  Postgres will then add this into its per-tuple
                // cost evaluations for whatever scan it's considering using for the `@@@` operator.
                // our IAM provides more intelligent costs for the IndexScan situation.
                (*src).per_tuple = GUCS.per_tuple_cost();

                ReturnedNodePointer(NonNull::new(node))
            }

            _ => ReturnedNodePointer(None),
        }
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

            let (lhs, rhs) = (args.get_ptr(0)?, args.get_ptr(1)?);
            if is_a(lhs, pg_sys::NodeTag::T_Var) && is_a(rhs, pg_sys::NodeTag::T_Const) {
                let var = lhs.cast::<pg_sys::Var>();
                let const_ = rhs.cast::<pg_sys::Const>();

                let (heaprelid, _, _) = find_var_relation(var, info);
                let indexrel = locate_bm25_index(heaprelid)?;
                let relfilenode = relfilenode_from_pg_relation(&indexrel);

                let search_config_jsonb =
                    JsonB::from_datum((*const_).constvalue, (*const_).constisnull)?;
                let search_config = SearchConfig::from_jsonb(search_config_jsonb)
                    .expect("SearchConfig should be valid");

                return estimate_selectivity(heaprelid, relfilenode, &search_config);
            }

            None
        }
    }

    assert!(operator_oid == anyelement_jsonb_opoid());

    let mut selectivity = inner_jsonb(planner_info, args).unwrap_or(UNKNOWN_SELECTIVITY);

    if selectivity > 1.0 {
        selectivity = UNKNOWN_SELECTIVITY;
    }

    selectivity
}
