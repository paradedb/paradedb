use datafusion::logical_expr::{DdlStatement, DropTable, LogicalPlan};
use datafusion::sql::TableReference;

use pgrx::*;

use std::sync::Arc;

use crate::datafusion::table::DatafusionTable;
use crate::hooks::columnar::ColumnarStmt;
use crate::nodes::producer::DatafusionPlansProducer;

pub struct DropStmtNode;
impl DatafusionPlansProducer for DropStmtNode {
    unsafe fn datafusion_plan(
        plan: *mut pg_sys::Plan,
        rtable: *mut pg_sys::List,
        _outer_plan: Option<LogicalPlan>,
    ) -> Result<Vec<LogicalPlan>, String> {
        let drop_stmt = plan as *mut pg_sys::DropStmt;
        let elements = (*rtable).elements;
        let mut drop_plans = vec![];

        for i in 0..(*rtable).length {
            let mut relation_data: *mut pg_sys::RelationData = std::ptr::null_mut();
            let obj = (*elements.offset(i as isize)).ptr_value as *mut pg_sys::Node;
            let _ = pg_sys::get_object_address(
                (*drop_stmt).removeType,
                obj,
                &mut relation_data,
                pg_sys::AccessShareLock as i32,
                (*drop_stmt).missing_ok,
            );

            if ColumnarStmt::relation_is_columnar(relation_data).unwrap_or(false) {
                let relation = pg_sys::RelationIdGetRelation((*relation_data).rd_id);
                let pg_relation = PgRelation::from_pg_owned(relation);
                let table = DatafusionTable::new(&pg_relation).unwrap();
                let table_name = table.name().unwrap().clone();
                let reference = TableReference::from(table_name);
                let schema = Arc::new(table.schema().unwrap());

                drop_plans.push(LogicalPlan::Ddl(DdlStatement::DropTable(DropTable {
                    if_exists: (*drop_stmt).missing_ok,
                    name: reference,
                    schema,
                })));
            }

            pg_sys::table_close(relation_data, pg_sys::NoLock as i32);
        }

        Ok(drop_plans)
    }
}
