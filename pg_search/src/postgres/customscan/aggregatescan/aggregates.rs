use crate::postgres::customscan::aggregatescan::{AggregateClause, AggregateType};
use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
use crate::postgres::customscan::CreateUpperPathsHookArgs;
use crate::postgres::customscan::CustomScan;
use crate::postgres::var::{find_one_var_and_fieldname, find_var_relation, VarContext};
use crate::schema::SearchIndexSchema;
use pgrx::pg_sys;
use pgrx::PgList;

pub(crate) struct AggregatesClause {
    aggregates: Vec<AggregateType>,
}

impl AggregatesClause {
    pub fn aggregates(&self) -> Vec<AggregateType> {
        self.aggregates.clone()
    }
}

impl AggregateClause for AggregatesClause {
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
        None
    }
}
