use datafusion::error::DataFusionError;
use datafusion::logical_expr::LogicalPlan;
use datafusion::sql::parser::DFParser;
use datafusion::sql::planner::SqlToRel;
use datafusion::sql::sqlparser::dialect::PostgreSqlDialect;
use datafusion::sql::sqlparser::parser::ParserError;
use thiserror::Error;

use super::context::QueryContext;

pub struct QueryString<'a>(pub &'a str);

// Parses the query string into a DataFusion LogicalPlan
impl TryFrom<QueryString<'_>> for LogicalPlan {
    type Error = LogicalPlanError;

    fn try_from(query: QueryString) -> Result<Self, Self::Error> {
        let QueryString(query) = query;

        let dialect = PostgreSqlDialect {};
        let ast = DFParser::parse_sql_with_dialect(query, &dialect)?;
        let statement = &ast[0];

        // Convert the AST into a logical plan
        let context_provider = QueryContext::new();
        let sql_to_rel = SqlToRel::new(&context_provider);
        Ok(sql_to_rel.statement_to_plan(statement.clone())?)
    }
}

#[derive(Error, Debug)]
pub enum LogicalPlanError {
    #[error(transparent)]
    DataFusion(#[from] DataFusionError),

    #[error(transparent)]
    Parser(#[from] ParserError),
}
