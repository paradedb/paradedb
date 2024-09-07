use crate::api::operator::{
    anyelement_jsonb_opoid, anyelement_jsonb_procoid, anyelement_text_opoid, estimate_selectivity,
    ReturnedNodePointer, UNKNOWN_SELECTIVITY,
};
use crate::postgres::utils::locate_bm25_index;
use crate::schema::SearchConfig;
use pgrx::pg_sys::planner_rt_fetch;
use pgrx::{
    is_a, pg_extern, pg_sys, AnyElement, FromDatum, Internal, IntoDatum, JsonB, PgList,
    PgMemoryContexts,
};
use std::ptr::NonNull;

#[pg_extern(immutable, parallel_safe)]
pub fn search_with_text(
    _element: AnyElement,
    query: &str,
    _fcinfo: pg_sys::FunctionCallInfo,
) -> bool {
    panic!("query is incompatible with pg_search's `@@@(key_field, TEXT)` operator: `{query}`")
}

#[pg_extern(immutable, parallel_safe)]
pub fn text_support(arg: Internal) -> ReturnedNodePointer {
    unsafe {
        let node = arg.unwrap().unwrap().cast_mut_ptr::<pg_sys::Node>();

        match (*node).type_ {
            pg_sys::NodeTag::T_SupportRequestSimplify => {
                let srs = node.cast::<pg_sys::SupportRequestSimplify>();

                let mut input_args = PgList::<pg_sys::Node>::from_pg((*(*srs).fcall).args);
                if let (Some(lhs), Some(rhs)) = (input_args.get_ptr(0), input_args.get_ptr(1)) {
                    if is_a(lhs, pg_sys::NodeTag::T_Var) && is_a(rhs, pg_sys::NodeTag::T_Const) {
                        let var = lhs.cast::<pg_sys::Var>();
                        let const_ = rhs.cast::<pg_sys::Const>();

                        // we're aboute to fabricate a new pg_sys::OpExpr node to return
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

                        // we need to use what should be the only `USING bm25` index on the table
                        let rte = planner_rt_fetch((*var).varno as pg_sys::Index, (*srs).root);
                        let indexrel = locate_bm25_index((*rte).relid).unwrap_or_else(|| {
                            panic!(
                                "relation `oid={}` must have a `USING bm25` index",
                                (*rte).relid.as_u32()
                            )
                        });

                        // the query comes from the rhs of the @@@ operator.  we've already proved it's a `pg_sys::Const` node
                        let query = String::from_datum((*const_).constvalue, (*const_).constisnull)
                            .expect("query must not be NULL");

                        // fabricate a `SearchConfig` from the above relation and query string
                        // and get it serialized into a JSONB Datum
                        let search_config = SearchConfig::from((query, indexrel));
                        let search_config_json = serde_json::to_value(&search_config)
                            .expect("SearchConfig should serialize to json");
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
                            .copy_ptr_into(&mut newopexpr, size_of::<pg_sys::OpExpr>());
                        return ReturnedNodePointer(NonNull::new(node.cast()));
                    }
                }

                ReturnedNodePointer(None)
            }

            pg_sys::NodeTag::T_SupportRequestCost => {
                let src = node.cast::<pg_sys::SupportRequestCost>();

                // our `search_with_*` functions are *incredibly* expensive.  So much so that
                // we really don't ever want Postgres to prefer them.  As such, hardcode in some
                // big numbers.
                (*src).per_tuple = 1_000_000.0;
                ReturnedNodePointer(NonNull::new(node))
            }

            _ => ReturnedNodePointer(None),
        }
    }
}

#[pg_extern(immutable, parallel_safe)]
pub fn text_restrict(
    planner_info: Internal, // <pg_sys::PlannerInfo>,
    operator_oid: pg_sys::Oid,
    args: Internal, // <pg_sys::List>,
    _var_relid: i32,
) -> f64 {
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

    assert!(operator_oid == anyelement_text_opoid());

    let mut selectivity = inner_text(planner_info, args).unwrap_or(UNKNOWN_SELECTIVITY);
    if selectivity > 1.0 {
        selectivity = UNKNOWN_SELECTIVITY;
    }

    selectivity
}
