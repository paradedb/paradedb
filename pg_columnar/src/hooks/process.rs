use async_std::task;

use pgrx::pg_sys::NodeTag;
use pgrx::*;
use std::ffi::CStr;

use crate::datafusion::context::DatafusionContext;
use crate::datafusion::registry::{PARADE_CATALOG, PARADE_SCHEMA};
use crate::datafusion::schema::ParadeSchemaProvider;
use crate::hooks::utils::ColumnarStmt;
use crate::nodes::t_dropstmt::DropStmtNode;
use crate::nodes::utils::DatafusionPlansProducer;
use crate::tableam::utils::{BulkInsertState, BULK_INSERT_STATE};

#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)]
pub fn process_utility(
    pstmt: PgBox<pg_sys::PlannedStmt>,
    query_string: &CStr,
    read_only_tree: Option<bool>,
    context: pg_sys::ProcessUtilityContext,
    params: PgBox<pg_sys::ParamListInfoData>,
    query_env: PgBox<pg_sys::QueryEnvironment>,
    dest: PgBox<pg_sys::DestReceiver>,
    completion_tag: *mut pg_sys::QueryCompletion,
    prev_hook: fn(
        pstmt: PgBox<pg_sys::PlannedStmt>,
        query_string: &CStr,
        read_only_tree: Option<bool>,
        context: pg_sys::ProcessUtilityContext,
        params: PgBox<pg_sys::ParamListInfoData>,
        query_env: PgBox<pg_sys::QueryEnvironment>,
        dest: PgBox<pg_sys::DestReceiver>,
        completion_tag: *mut pg_sys::QueryCompletion,
    ) -> HookResult<()>,
) -> HookResult<()> {
    let plan = pstmt.utilityStmt;

    match unsafe { (*plan).type_ } {
        NodeTag::T_CopyStmt => {
            let copy_stmt = plan as *mut pg_sys::CopyStmt;

            if unsafe { ColumnarStmt::copy_is_columnar(copy_stmt).unwrap_or(false) } {
                let mut bulk_insert_state = BULK_INSERT_STATE.write();
                *bulk_insert_state = BulkInsertState::new();
            }
        }
        NodeTag::T_DropStmt => unsafe {
            let plans = DropStmtNode::datafusion_plan(
                plan as *mut pg_sys::Plan,
                (*(plan as *mut pg_sys::DropStmt)).objects,
                None,
            )
            .expect("Failed to create DropTable plan");

            if !plans.is_empty() {
                DatafusionContext::with_read(|context| {
                    for plan in plans {
                        let dataframe = task::block_on(context.execute_logical_plan(plan)).unwrap();
                        let _ = task::block_on(dataframe.collect()).unwrap();
                    }
                });
            }
        },
        NodeTag::T_VacuumStmt => unsafe {
            let vacuum_stmt = plan as *mut pg_sys::VacuumStmt;
            let rels = (*vacuum_stmt).rels;
            // Rels is null if VACUUM was called, not null if VACUUM <table> was called
            let vacuum_all = rels.is_null();
            // VacuumStmt can also be used for other statements, so we need to check if it's actually VACUUM
            let is_vacuum = (*vacuum_stmt).is_vacuumcmd;

            if is_vacuum && vacuum_all {
                DatafusionContext::with_read(|context| {
                    let schema_provider = context
                        .catalog(PARADE_CATALOG)
                        .expect("Catalog not found")
                        .schema(PARADE_SCHEMA)
                        .expect("Schema not found");

                    let lister = schema_provider
                        .as_any()
                        .downcast_ref::<ParadeSchemaProvider>()
                        .expect("Failed to downcast schema provider");

                    task::block_on(lister.vacuum_tables(&context.state()))
                        .expect("Failed to vacuum tables");
                });
            }
        },
        _ => {}
    }

    prev_hook(
        pstmt,
        query_string,
        read_only_tree,
        context,
        params,
        query_env,
        dest,
        completion_tag,
    );

    HookResult::new(())
}
