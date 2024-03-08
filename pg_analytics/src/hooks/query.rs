use deltalake::datafusion::error::DataFusionError;
use deltalake::datafusion::logical_expr::LogicalPlan;
use deltalake::datafusion::sql::parser::{self, DFParser};
use deltalake::datafusion::sql::planner::SqlToRel;
use deltalake::datafusion::sql::sqlparser::dialect::PostgreSqlDialect;
use pgrx::*;
use std::collections::VecDeque;
use std::ffi::CStr;

use crate::datafusion::context::QueryContext;
use crate::errors::ParadeError;

pub trait Query {
    // Extracts the query string from a PlannedStmt,
    // accounting for multi-line queries where we only want a
    // specific line of the entire query.
    fn get_query_string(self, source_text: &CStr) -> Result<String, ParadeError>;

    // Parses the query string into an AST
    fn get_ast(self, query_string: &str) -> Result<VecDeque<parser::Statement>, ParadeError>;

    // Parses the query string into a DataFusion LogicalPlan
    fn get_logical_plan(self, query_string: &str) -> Result<LogicalPlan, ParadeError>;
}

impl Query for *mut pg_sys::PlannedStmt {
    fn get_query_string(self, source_text: &CStr) -> Result<String, ParadeError> {
        let query_start_index = unsafe { (*self).stmt_location };
        let query_len = unsafe { (*self).stmt_len };
        let mut query = source_text.to_str()?;

        if query_start_index != -1 {
            if query_len == 0 {
                query = &query[(query_start_index as usize)..query.len()];
            } else {
                query = &query
                    [(query_start_index as usize)..((query_start_index + query_len) as usize)];
            }
        }

        Ok(query.to_string())
    }

    fn get_ast(self, query: &str) -> Result<VecDeque<parser::Statement>, ParadeError> {
        let dialect = PostgreSqlDialect {};
        DFParser::parse_sql_with_dialect(query, &dialect)
            .map_err(|err| ParadeError::DataFusion(DataFusionError::SQL(err, None)))
    }

    fn get_logical_plan(self, query: &str) -> Result<LogicalPlan, ParadeError> {
        let dialect = PostgreSqlDialect {};
        let ast = DFParser::parse_sql_with_dialect(query, &dialect)
            .map_err(|err| ParadeError::DataFusion(DataFusionError::SQL(err, None)))?;
        let statement = &ast[0];

        let context_provider = QueryContext::new()?;
        let sql_to_rel = SqlToRel::new(&context_provider);

        Ok(sql_to_rel.statement_to_plan(statement.clone())?)
    }
}
