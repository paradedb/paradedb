use crate::postgres::customscan::aggregatescan::AggregateClause;
use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
use crate::postgres::customscan::CreateUpperPathsHookArgs;
use crate::postgres::customscan::CustomScan;
use crate::postgres::var::{find_one_var_and_fieldname, find_var_relation, VarContext};
use crate::schema::SearchIndexSchema;
use pgrx::pg_sys;
use pgrx::PgList;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GroupingColumn {
    pub field_name: String,
    pub attno: pg_sys::AttrNumber,
}

pub(crate) struct GroupByClause {
    grouping_columns: Vec<GroupingColumn>,
}

impl GroupByClause {
    pub fn grouping_columns(&self) -> Vec<GroupingColumn> {
        self.grouping_columns.clone()
    }
}

impl AggregateClause for GroupByClause {
    fn add_to_custom_path<CS>(&self, builder: CustomPathBuilder<CS>) -> CustomPathBuilder<CS>
    where
        CS: CustomScan,
    {
        builder
    }

    fn from_pg(
        args: &CreateUpperPathsHookArgs,
        heap_rti: pg_sys::Index,
        schema: &SearchIndexSchema,
    ) -> Option<Self> {
        let mut grouping_columns = Vec::new();

        let pathkeys = if unsafe { (*args.root()).group_pathkeys.is_null() } {
            PgList::<pg_sys::PathKey>::new()
        } else {
            unsafe { PgList::<pg_sys::PathKey>::from_pg((*args.root()).group_pathkeys) }
        };

        for pathkey in pathkeys.iter_ptr() {
            unsafe {
                let equivclass = (*pathkey).pk_eclass;
                let members =
                    PgList::<pg_sys::EquivalenceMember>::from_pg((*equivclass).ec_members);

                let mut found_valid_column = false;
                for member in members.iter_ptr() {
                    let expr = (*member).em_expr;

                    let var_context = VarContext::from_planner(args.root);

                    let (field_name, attno) = if let Some((var, field_name)) =
                        find_one_var_and_fieldname(var_context, expr as *mut pg_sys::Node)
                    {
                        // JSON operator expression or complex field access
                        let (heaprelid, attno, _) = find_var_relation(var, args.root);
                        if heaprelid == pg_sys::InvalidOid {
                            continue;
                        }
                        (field_name.to_string(), attno)
                    } else {
                        continue;
                    };

                    // Check if this field exists in the index schema as a fast field
                    if let Some(search_field) = schema.search_field(&field_name) {
                        if search_field.is_fast() {
                            grouping_columns.push(GroupingColumn { field_name, attno });
                            found_valid_column = true;
                            break; // Found a valid grouping column for this pathkey
                        }
                    }
                }

                if !found_valid_column {
                    return None;
                }
            }
        }

        Some(Self { grouping_columns })
    }
}
