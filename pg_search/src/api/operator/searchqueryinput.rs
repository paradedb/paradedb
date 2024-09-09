use crate::api::operator::{
    anyelement_query_input_opoid, estimate_selectivity, make_search_config_opexpr_node,
    ReturnedNodePointer, UNKNOWN_SELECTIVITY,
};
use crate::postgres::utils::locate_bm25_index;
use crate::query::SearchQueryInput;
use crate::schema::SearchConfig;
use pgrx::pg_sys::planner_rt_fetch;
use pgrx::{is_a, pg_extern, pg_sys, AnyElement, FromDatum, Internal, PgList};

#[pg_extern(immutable, parallel_safe)]
pub fn search_with_query_input(
    _element: AnyElement,
    query: SearchQueryInput,
    _fcinfo: pg_sys::FunctionCallInfo,
) -> bool {
    panic!("query is incompatible with pg_search's `@@@(key_field, paradedb.searchqueryinput)` operator: `{query:?}`")
}

#[pg_extern(immutable, parallel_safe)]
pub fn query_input_support(arg: Internal) -> ReturnedNodePointer {
    unsafe {
        let node = arg.unwrap().unwrap().cast_mut_ptr::<pg_sys::Node>();

        match (*node).type_ {
            // rewrite this node, which is using the @@@(key_field, paradedb.searchqueryinput) operator
            // to instead use the @@@(key_field, jsonb) operator.  This involves converting the rhs
            // of the operator into the jsonb representation of a SearchConfig, which is built
            // in `make_new_opexpr_node()`
            pg_sys::NodeTag::T_SupportRequestSimplify => {
                let srs = node.cast::<pg_sys::SupportRequestSimplify>();

                let mut input_args = PgList::<pg_sys::Node>::from_pg((*(*srs).fcall).args);
                if let (Some(lhs), Some(rhs)) = (input_args.get_ptr(0), input_args.get_ptr(1)) {
                    if is_a(lhs, pg_sys::NodeTag::T_Var) && is_a(rhs, pg_sys::NodeTag::T_Const) {
                        let var = lhs.cast::<pg_sys::Var>();
                        let const_ = rhs.cast::<pg_sys::Const>();

                        // the query comes from the rhs of the @@@ operator.  we've already proved it's a `pg_sys::Const` node
                        let query = SearchQueryInput::from_datum(
                            (*const_).constvalue,
                            (*const_).constisnull,
                        )
                        .expect("query must not be NULL");

                        return make_search_config_opexpr_node(srs, &mut input_args, var, query);
                    }
                }

                ReturnedNodePointer(None)
            }

            _ => ReturnedNodePointer(None),
        }
    }
}

#[pg_extern(immutable, parallel_safe)]
pub fn query_input_restrict(
    planner_info: Internal, // <pg_sys::PlannerInfo>,
    operator_oid: pg_sys::Oid,
    args: Internal, // <pg_sys::List>,
    _var_relid: i32,
) -> f64 {
    fn inner_query_input(
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

                    let query =
                        SearchQueryInput::from_datum((*const_).constvalue, (*const_).constisnull)?;
                    let search_config = SearchConfig::from((query, indexrel));

                    return estimate_selectivity(heaprelid, &search_config);
                }
            }

            None
        }
    }

    assert!(operator_oid == anyelement_query_input_opoid());

    let mut selectivity = inner_query_input(planner_info, args).unwrap_or(UNKNOWN_SELECTIVITY);
    if selectivity > 1.0 {
        selectivity = UNKNOWN_SELECTIVITY;
    }

    selectivity
}