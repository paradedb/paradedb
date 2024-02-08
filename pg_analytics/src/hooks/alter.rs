use deltalake::datafusion::error::DataFusionError;
use deltalake::datafusion::sql::parser::DFParser;
use deltalake::datafusion::sql::planner::SqlToRel;
use deltalake::datafusion::sql::sqlparser::dialect::PostgreSqlDialect;
use pgrx::*;

use crate::errors::{NotSupported, ParadeError};
use crate::hooks::handler::DeltaHandler;

pub unsafe fn alter(alter_stmt: *mut pg_sys::AlterTableStmt) -> Result<(), ParadeError> {
    let rangevar = (*alter_stmt).relation;
    let rangevar_oid = pg_sys::RangeVarGetRelidExtended(
        rangevar,
        pg_sys::ShareUpdateExclusiveLock as i32,
        0,
        None,
        std::ptr::null_mut(),
    );
    let relation = pg_sys::RelationIdGetRelation(rangevar_oid);

    if relation.is_null() {
        return Ok(());
    }

    if !DeltaHandler::relation_is_delta(relation)? {
        pg_sys::RelationClose(relation);
        return Ok(());
    }

    let pg_relation = PgRelation::from_pg(relation);
    let tupdesc = pg_relation.tuple_desc();
    for attribute in tupdesc.iter() {
        if attribute.is_dropped() {
            continue;
        }

        let attname = attribute.name();
        info!("Attribute name: {}", attname);
    }

    Err(NotSupported::AlterTable.into())
}
