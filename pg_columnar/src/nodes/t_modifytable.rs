use datafusion::arrow::datatypes::Schema;
use datafusion::common::arrow::datatypes::Field;
use datafusion::common::DFSchema;
use datafusion::logical_expr::WriteOp::{Delete, InsertInto, Update};
use datafusion::logical_expr::{DmlStatement, LogicalPlan};

use pgrx::*;

use crate::nodes::utils::DatafusionPlanTranslator;
use crate::nodes::utils::{datafusion_err_to_string, datafusion_table_from_rte};

pub struct ModifyTableNode;
impl DatafusionPlanTranslator for ModifyTableNode {
    unsafe fn datafusion_plan(
        plan: *mut pg_sys::Plan,
        rtable: *mut pg_sys::List,
        outer_plan: Option<LogicalPlan>,
    ) -> Result<LogicalPlan, String> {
        let modify = plan as *mut pg_sys::ModifyTable;
        let rte = pg_sys::rt_fetch((*modify).nominalRelation, rtable);
        let table = datafusion_table_from_rte(rte)?;
        let schema = DFSchema::try_from(Schema::new(
            table
                .schema()
                .fields()
                .iter()
                .map(|f| Field::new(f.name(), f.data_type().clone(), f.is_nullable()))
                .collect::<Vec<_>>(),
        ))
        .map_err(datafusion_err_to_string("Result DFSchema failed"))?;
        let relation = pg_sys::RelationIdGetRelation((*rte).relid);
        let pg_relation = PgRelation::from_pg_owned(relation);
        let tablename = format!("{}", pg_relation.oid());

        Ok(LogicalPlan::Dml(DmlStatement {
            table_name: tablename.into(),
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
