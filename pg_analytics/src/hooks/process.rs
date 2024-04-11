use crate::datafusion::query::{ASTVec, QueryString};
use async_std::task;
use pgrx::pg_sys::NodeTag;
use pgrx::*;
use std::ffi::CStr;
use thiserror::Error;

use super::alter::{alter, AlterHookError};
use super::create::{CreateClassifier, PartitionDescriptor, CreateHookError};
use super::drop::{drop_relations, DropHookError};
use super::query::{Query, QueryStringError};
use super::rename::{rename, RenameHookError};
use super::truncate::{truncate, TruncateHookError};
use super::vacuum::{vacuum, VacuumHookError};
use crate::datafusion::catalog::CatalogError;
use crate::datafusion::udf::{createfunction, UDFError};
use crate::tableam::CREATE_TABLE_PARTITIONS;

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
) -> Result<(), ProcessHookError> {
    unsafe {
        let plan = pstmt.utilityStmt;

        // Parse the query into an AST
        let pg_plan = pstmt.clone().into_pg();
        let query = pg_plan.get_query_string(query_string)?;

        let ast = ASTVec::try_from(QueryString(&query));

        match (*plan).type_ {
            NodeTag::T_AlterTableStmt => {
                if let Ok(ASTVec(ast)) = ast {
                    task::block_on(alter(plan as *mut pg_sys::AlterTableStmt, &ast[0]))?;
                }
            }
            NodeTag::T_CreateFunctionStmt => {
                createfunction(plan as *mut pg_sys::CreateFunctionStmt)?;
            }
            NodeTag::T_CreateStmt => {
                // If creating a partitioned table with parquet table AM, allow
                //     PARTITION BY JOIN with the columns to partition on.
                let mut partitions = task::block_on(CREATE_TABLE_PARTITIONS.lock());
                (*partitions).clear();

                let stmt = plan as *mut pg_sys::CreateStmt;
                if stmt.is_parquet()? && stmt.has_partition_strategy() {
                    if !stmt.partition_columns_ordered_at_end()? {
                        return Err(ProcessHookError::PartitionOrder);
                    }
                    (*partitions).append(&mut stmt.partition_list_columns()?);
                    // nullify the partspec to get postgres to ignore the partition when processing the query,
                    //     because otherwise postgres will fail on a PARTITION BY with table am
                    (*stmt).partspec = std::ptr::null_mut();
                }

                // prev_hook can panic, which means this function doesn't return and the lock doesn't get dropped,
                //     so we need to drop the lock explicitly.
                drop(partitions);
            }
            NodeTag::T_DropStmt => {
                drop_relations(plan as *mut pg_sys::DropStmt)?;
            }
            NodeTag::T_RenameStmt => {
                if let Ok(ASTVec(ast)) = ast {
                    rename(plan as *mut pg_sys::RenameStmt, &ast[0])?;
                }
            }
            NodeTag::T_TruncateStmt => {
                task::block_on(truncate(plan as *mut pg_sys::TruncateStmt))?;
            }
            NodeTag::T_VacuumStmt => {
                vacuum(plan as *mut pg_sys::VacuumStmt)?;
            }
            _ => {}
        };

        let _ = prev_hook(
            pstmt,
            query_string,
            read_only_tree,
            context,
            params,
            query_env,
            dest,
            completion_tag,
        );

        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum ProcessHookError {
    #[error(transparent)]
    Catalog(#[from] CatalogError),

    #[error(transparent)]
    AlterHook(#[from] AlterHookError),

    #[error(transparent)]
    CreateHook(#[from] CreateHookError),

    #[error(transparent)]
    DropHook(#[from] DropHookError),

    #[error(transparent)]
    QueryString(#[from] QueryStringError),

    #[error(transparent)]
    RenameHook(#[from] RenameHookError),

    #[error(transparent)]
    TruncateHook(#[from] TruncateHookError),

    #[error(transparent)]
    VacuumHook(#[from] VacuumHookError),

    #[error(transparent)]
    Udf(#[from] UDFError),

    #[error("PARTITION BY on parquet tables requires partition columns to be in order at the end of the field list")]
    PartitionOrder,
}
