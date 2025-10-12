use crate::nodecast;
use crate::postgres::customscan::aggregatescan::{AggregateClause, AggregateType};
use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
use crate::postgres::customscan::CreateUpperPathsHookArgs;
use crate::postgres::customscan::qual_inspect::QualExtractState;
use crate::postgres::customscan::CustomScan;
use crate::postgres::PgSearchRelation;
use crate::postgres::var::{find_one_var_and_fieldname, find_var_relation, VarContext};
use crate::schema::SearchIndexSchema;
use pgrx::pg_sys;
use pgrx::PgList;

pub(crate) struct TargetList {
    aggregates: Vec<AggregateType>,
    uses_our_operator: bool,
}

impl TargetList {
    pub fn aggregates(&self) -> Vec<AggregateType> {
        self.aggregates.clone()
    }

    pub fn uses_our_operator(&self) -> bool {
        self.uses_our_operator
    }
}

impl AggregateClause for TargetList {
    fn add_to_custom_path<CS>(&self, builder: CustomPathBuilder<CS>) -> CustomPathBuilder<CS>
    where
        CS: CustomScan,
    {
        builder
    }

    fn from_pg(
        args: &CreateUpperPathsHookArgs,
        heap_rti: pg_sys::Index,
        index: &PgSearchRelation,
    ) -> Option<Self> {
        let schema = index.schema().ok()?;

        let target_list =
            unsafe { PgList::<pg_sys::Expr>::from_pg((*args.output_rel().reltarget).exprs) };

        if target_list.is_empty() {
            return None;
        }

        let heap_rte = unsafe {
            let rt = PgList::<pg_sys::RangeTblEntry>::from_pg((*args.root().parse).rtable);
            rt.get_ptr((heap_rti - 1) as usize)?
        };
        let heap_oid = unsafe { (*heap_rte).relid };

        let mut aggregates = Vec::new();
        let mut uses_our_operator = false;

        for expr in target_list.iter_ptr() {
            unsafe {
                let node_tag = (*expr).type_;

                if let Some(_var) = nodecast!(Var, T_Var, expr) {
                    continue;
                } else if let Some(_opexpr) = nodecast!(OpExpr, T_OpExpr, expr) {
                    let var_context = VarContext::from_planner(args.root() as *const _ as *mut _);
                    if find_one_var_and_fieldname(var_context, expr as *mut pg_sys::Node).is_some() {
                        continue;
                    } else {
                        return None;
                    }
                } else if let Some(aggref) = nodecast!(Aggref, T_Aggref, expr) {
                    // TODO: Support DISTINCT
                    if !(*aggref).aggdistinct.is_null() {
                        return None;
                    }

                    let mut qual_state = QualExtractState::default();
                    let aggregate =
                        AggregateType::try_from(aggref, heap_oid, index, args.root, heap_rti, &mut qual_state)?;
                    uses_our_operator = uses_our_operator || qual_state.uses_our_operator;

                    if let Some(field_name) = aggregate.field_name() {
                        if let Some(search_field) = schema.search_field(&field_name) {
                            if !search_field.is_fast() {
                                return None;
                            }
                        } else {
                            return None;
                        }
                    }

                    aggregates.push(aggregate);
                } else {
                    return None;
                }
            }
        }

        Some(TargetList {
            aggregates,
            uses_our_operator,
        })
    }
}
