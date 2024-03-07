use deltalake::datafusion::error::DataFusionError;
use deltalake::datafusion::logical_expr::LogicalPlan;
use deltalake::datafusion::sql::parser::DFParser;
use deltalake::datafusion::sql::planner::SqlToRel;
use deltalake::datafusion::sql::sqlparser::dialect::PostgreSqlDialect;

use crate::datafusion::context::QueryContext;
use crate::errors::ParadeError;

#[inline]
pub fn create_logical_plan(query: &str) -> Result<LogicalPlan, ParadeError> {
    let dialect = PostgreSqlDialect {};
    let ast = DFParser::parse_sql_with_dialect(query, &dialect)
        .map_err(|err| ParadeError::DataFusion(DataFusionError::SQL(err, None)))?;
    let statement = &ast[0];

    // Convert the AST into a logical plan
    let context_provider = QueryContext::new()?;
    let sql_to_rel = SqlToRel::new(&context_provider);

    Ok(sql_to_rel.statement_to_plan(statement.clone())?)
}
