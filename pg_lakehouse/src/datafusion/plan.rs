// Copyright (c) 2023-2024 Retake, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

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
