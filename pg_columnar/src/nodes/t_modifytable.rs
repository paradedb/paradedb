use datafusion::logical_expr::WriteOp::{Delete, InsertInto, Update};
use datafusion::logical_expr::{DmlStatement, LogicalPlan};
use pgrx::*;

use crate::datafusion::table::DatafusionTable;
use crate::nodes::utils::DatafusionPlanProducer;
use crate::tableam::utils::get_pg_relation;

pub struct ModifyTableNode;
impl DatafusionPlanProducer for ModifyTableNode {
    unsafe fn datafusion_plan(
        plan: *mut pg_sys::Plan,
        rtable: *mut pg_sys::List,
        outer_plan: Option<LogicalPlan>,
    ) -> Result<LogicalPlan, String> {
        let modify = plan as *mut pg_sys::ModifyTable;
        let rte = pg_sys::rt_fetch((*modify).nominalRelation, rtable);
        let pg_relation = get_pg_relation(rte)?;
        let table = DatafusionTable::new(&pg_relation)?;

        Ok(LogicalPlan::Dml(DmlStatement {
            table_name: table.name()?.into(),
            table_schema: table.schema()?.into(),
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
