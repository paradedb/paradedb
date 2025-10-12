use crate::api::{HashSet, OrderByFeature, OrderByInfo};
use crate::customscan::builders::custom_path::{CustomPathBuilder, OrderByStyle};
use crate::customscan::CustomScan;
use crate::postgres::customscan::aggregatescan::AggregateClause;
use crate::postgres::customscan::pdbscan::extract_pathkey_styles_with_sortability_check;
use crate::postgres::customscan::pdbscan::PathKeyInfo;
use crate::postgres::var::{find_one_var_and_fieldname, VarContext};
use crate::schema::SearchIndexSchema;
use pgrx::pg_sys;
use pgrx::PgList;

pub(crate) struct OrderByClause {
    sort_clause: *mut pg_sys::List,
    pathkeys: PathKeyInfo,
    orderby_info: Vec<OrderByInfo>,
}

impl OrderByClause {
    pub fn orderby_info(&self) -> Vec<OrderByInfo> {
        self.orderby_info.clone()
    }

    pub unsafe fn sort_clause(&self) -> PgList<pg_sys::SortGroupClause> {
        PgList::from_pg(self.sort_clause)
    }
}

impl AggregateClause for OrderByClause {
    fn add_to_custom_path<CS>(&self, mut builder: CustomPathBuilder<CS>) -> CustomPathBuilder<CS>
    where
        CS: CustomScan,
    {
        if let Some(pathkeys) = self.pathkeys.pathkeys() {
            for pathkey_style in pathkeys {
                builder = builder.add_path_key(pathkey_style);
            }
        };

        builder
    }

    fn from_pg(
        root: *mut pg_sys::PlannerInfo,
        heap_rti: pg_sys::Index,
        schema: &SearchIndexSchema,
    ) -> Option<Self> {
        let parse = unsafe { (*root).parse };

        let sort_clause =
            unsafe { PgList::<pg_sys::SortGroupClause>::from_pg((*parse).sortClause) };

        let sort_fields = unsafe {
            sort_clause
                .iter_ptr()
                .filter_map(|sort_clause| {
                    let expr = pg_sys::get_sortgroupclause_expr(sort_clause, (*parse).targetList);
                    let var_context = VarContext::from_planner(root);
                    if let Some((_, field_name)) = find_one_var_and_fieldname(var_context, expr) {
                        Some(field_name)
                    } else {
                        None
                    }
                })
                .collect::<HashSet<_>>()
        };

        let pathkeys = unsafe {
            extract_pathkey_styles_with_sortability_check(
                root,
                heap_rti,
                schema,
                |f| f.is_fast(),
                |_| false,
            )
        };

        let orderby_info = OrderByStyle::extract_orderby_info(pathkeys.pathkeys())
            .into_iter()
            .filter(|info| {
                if let OrderByFeature::Field(field_name) = &info.feature {
                    sort_fields.contains(field_name)
                } else {
                    false
                }
            })
            .collect::<Vec<_>>();

        Some(Self {
            sort_clause: unsafe { (*parse).sortClause },
            pathkeys,
            orderby_info,
        })
    }
}
