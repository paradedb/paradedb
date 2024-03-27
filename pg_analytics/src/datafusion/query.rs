use deltalake::datafusion::error::DataFusionError;
use deltalake::datafusion::logical_expr::LogicalPlan;
use deltalake::datafusion::sql::parser::{self, DFParser};
use deltalake::datafusion::sql::planner::SqlToRel;
use deltalake::datafusion::sql::sqlparser::dialect::PostgreSqlDialect;
use std::collections::VecDeque;

use crate::datafusion::context::QueryContext;
use crate::errors::ParadeError;

pub struct QueryString<'a>(pub &'a str);
pub struct ASTVec(pub VecDeque<parser::Statement>);

// Parses a query string into an AST
impl TryFrom<QueryString<'_>> for ASTVec {
    type Error = ParadeError;

    fn try_from(query: QueryString) -> Result<Self, Self::Error> {
        let QueryString(query) = query;

        let dialect = PostgreSqlDialect {};
        Ok(ASTVec(
            DFParser::parse_sql_with_dialect(query, &dialect)
                .map_err(|err| ParadeError::DataFusion(DataFusionError::SQL(err, None)))?,
        ))
    }
}

// Parses the query string into a DataFusion LogicalPlan
impl TryFrom<QueryString<'_>> for LogicalPlan {
    type Error = ParadeError;

    fn try_from(query: QueryString) -> Result<Self, Self::Error> {
        let QueryString(query) = query;

        let dialect = PostgreSqlDialect {};
        let ast = DFParser::parse_sql_with_dialect(query, &dialect)
            .map_err(|err| ParadeError::DataFusion(DataFusionError::SQL(err, None)))?;
        let statement = &ast[0];

        let context_provider = QueryContext::new()?;
        let sql_to_rel = SqlToRel::new(&context_provider);

        Ok(sql_to_rel.statement_to_plan(statement.clone())?)
    }
}
