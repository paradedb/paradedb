use datafusion::logical_expr::WriteOp::{Delete, InsertInto, Update};
use datafusion::logical_expr::{DmlStatement, LogicalPlan};

use pgrx::*;

use crate::nodes::utils::DatafusionPlanTranslator;
use crate::nodes::utils::{
    datafusion_schema_from_table, datafusion_table_from_name, table_name_from_rte,
};

pub struct ModifyTableNode;
impl DatafusionPlanTranslator for ModifyTableNode {
    unsafe fn datafusion_plan(
        plan: *mut pg_sys::Plan,
        rtable: *mut pg_sys::List,
        outer_plan: Option<LogicalPlan>,
    ) -> Result<LogicalPlan, String> {
        let modify = plan as *mut pg_sys::ModifyTable;
        let rte = pg_sys::rt_fetch((*modify).nominalRelation, rtable);
        let table_name = table_name_from_rte(rte)?;
        let table_source = datafusion_table_from_name(&table_name)?;
        let schema = datafusion_schema_from_table(table_source)?;

        Ok(LogicalPlan::Dml(DmlStatement {
            table_name: table_name.into(),
            table_schema: schema.into(),
            op: match (*modify).operation {
                // TODO: WriteOp::InsertOverwrite also exists - handle that properly
                // TODO: Shouldn't we only be supporting inserts?
                pg_sys::CmdType_CMD_INSERT => InsertInto,
                pg_sys::CmdType_CMD_UPDATE => Update,
                pg_sys::CmdType_CMD_DELETE => Delete,
                _ => return Err("Unsupported DML operation".to_string()),
            },
            input: outer_plan.ok_or("ModifyTable has no outer_plan")?.into(),
        }))
    }
}
