use deltalake::datafusion::error::DataFusionError;
use deltalake::datafusion::logical_expr::LogicalPlan;

use deltalake::datafusion::sql::parser::DFParser;
use deltalake::datafusion::sql::planner::SqlToRel;

use deltalake::datafusion::sql::sqlparser::dialect::PostgreSqlDialect;
use pgrx::*;
use std::ffi::CStr;

use crate::datafusion::context::ParadeContextProvider;

use crate::errors::{NotSupported, ParadeError};
use crate::hooks::delete::delete;
use crate::hooks::handler::DeltaHandler;
use crate::hooks::select::select;
use crate::hooks::update::update;

pub fn executor_run(
    query_desc: PgBox<pg_sys::QueryDesc>,
    direction: pg_sys::ScanDirection,
    count: u64,
    execute_once: bool,
    prev_hook: fn(
        query_desc: PgBox<pg_sys::QueryDesc>,
        direction: pg_sys::ScanDirection,
        count: u64,
        execute_once: bool,
    ) -> HookResult<()>,
) -> Result<(), ParadeError> {
    unsafe {
        let ps = query_desc.plannedstmt;
        let rtable = (*ps).rtable;

        // Only use this hook for deltalake tables
        if rtable.is_null() || !DeltaHandler::rtable_is_delta(rtable)? {
            prev_hook(query_desc, direction, count, execute_once);
            return Ok(());
        }

        // Execute SELECT, DELETE, UPDATE
        match query_desc.operation {
            pg_sys::CmdType_CMD_DELETE => {
                let logical_plan = create_logical_plan(query_desc.clone())?;
                delete(rtable, query_desc, logical_plan)
            }
            pg_sys::CmdType_CMD_SELECT => {
                let logical_plan = create_logical_plan(query_desc.clone())?;
                select(query_desc, logical_plan)
            }
            pg_sys::CmdType_CMD_UPDATE => {
                let logical_plan = create_logical_plan(query_desc.clone())?;
                update(rtable, query_desc, logical_plan)
            }
            _ => {
                prev_hook(query_desc, direction, count, execute_once);
                Ok(())
            }
        }
    }
}

#[inline]
fn create_logical_plan(query_desc: PgBox<pg_sys::QueryDesc>) -> Result<LogicalPlan, ParadeError> {
    let dialect = PostgreSqlDialect {};
    let query = unsafe { CStr::from_ptr(query_desc.sourceText).to_str()? };
    let ast = DFParser::parse_sql_with_dialect(query, &dialect)
        .map_err(|err| ParadeError::DataFusion(DataFusionError::SQL(err, None)))?;
    let statement = &ast[0];

    // Convert the AST into a logical plan
    let context_provider = ParadeContextProvider::new()?;
    let sql_to_rel = SqlToRel::new(&context_provider);

    Ok(sql_to_rel.statement_to_plan(statement.clone())?)
}
