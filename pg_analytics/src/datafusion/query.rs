use deltalake::datafusion::common::ScalarValue;
use deltalake::datafusion::error::DataFusionError;
use deltalake::datafusion::logical_expr::expr::ScalarFunction;
use deltalake::datafusion::logical_expr::{Expr, ScalarFunctionDefinition};
use deltalake::datafusion::sql::parser::{self, DFParser};
use deltalake::datafusion::sql::planner::SqlToRel;
use deltalake::datafusion::sql::sqlparser::dialect::PostgreSqlDialect;
use deltalake::datafusion::sql::sqlparser::parser::ParserError;
use regex::Regex;
use std::collections::VecDeque;
use thiserror::Error;

use super::catalog::CatalogError;
use super::context::QueryContext;
use super::plan::LogicalPlanDetails;
use super::udf::{loadfunction_not_supported, UDFError};

pub struct QueryString<'a>(pub &'a str);
pub struct ASTVec(pub VecDeque<parser::Statement>);

// Parses a query string into an AST
impl TryFrom<QueryString<'_>> for ASTVec {
    type Error = QueryParserError;

    fn try_from(query: QueryString) -> Result<Self, Self::Error> {
        let QueryString(query) = query;

        let dialect = PostgreSqlDialect {};
        Ok(ASTVec(DFParser::parse_sql_with_dialect(query, &dialect)?))
    }
}

// Parses the query string into a DataFusion LogicalPlan
impl TryFrom<QueryString<'_>> for LogicalPlanDetails {
    type Error = QueryParserError;

    fn try_from(query: QueryString) -> Result<Self, Self::Error> {
        let QueryString(query) = query;

        let dialect = PostgreSqlDialect {};
        let ast = DFParser::parse_sql_with_dialect(query, &dialect)?;
        let statement = &ast[0];

        // Convert the AST into a logical plan
        let context_provider = QueryContext::new()?;
        let sql_to_rel = SqlToRel::new(&context_provider);

        // If functions are undefined, then try to find and register the function and then try to get the plan again
        let re = Regex::new(r"Invalid function '(.+)'")?;
        let logical_plan = loop {
            match sql_to_rel.statement_to_plan(statement.clone()) {
                Ok(plan) => break plan,
                Err(DataFusionError::Plan(err_string)) => {
                    // This regex checks for "Invalid function" in the plan error and
                    //     otherwise pushes the plan error up, breaking the loop.
                    let missing_func_name = re
                        .captures(&err_string)
                        .ok_or(QueryParserError::GenericRegexError(err_string.clone()))?
                        .get(1)
                        .ok_or(QueryParserError::GenericRegexError(err_string.clone()))?
                        .as_str();

                    // If we are unable to load the function, we push the error up, breaking the loop
                    loadfunction_not_supported(missing_func_name)?;

                    // Loop again
                }
                Err(err) => return Err(QueryParserError::DataFusion(err)),
            };
        };

        // Pass UDF name as another argument to UDFs
        let exprs = logical_plan.expressions();
        let mut new_exprs = vec![];
        let mut includes_udf = false;
        for expr in exprs.iter() {
            if let Expr::ScalarFunction(ScalarFunction {
                func_def: ScalarFunctionDefinition::UDF(udf),
                args,
            }) = expr
            {
                // Pass funcname to udf
                let mut new_args = args.clone();
                new_args.splice(
                    0..0,
                    vec![Expr::Literal(ScalarValue::Utf8(Some(
                        udf.name().to_string(),
                    )))],
                );
                new_exprs.push(Expr::ScalarFunction(ScalarFunction {
                    func_def: ScalarFunctionDefinition::UDF(udf.clone()),
                    args: new_args,
                }));
                includes_udf = true;
            } else {
                new_exprs.push(expr.clone());
            }
        }

        let new_inputs = logical_plan
            .inputs()
            .iter()
            .cloned()
            .cloned()
            .collect::<Vec<_>>();
        let new_logical_plan = logical_plan.with_new_exprs(new_exprs, new_inputs.as_slice())?;

        Ok(LogicalPlanDetails::new(new_logical_plan, includes_udf))
    }
}

#[derive(Error, Debug)]
pub enum QueryParserError {
    #[error(transparent)]
    Catalog(#[from] CatalogError),

    #[error(transparent)]
    DataFusion(#[from] DataFusionError),

    #[error(transparent)]
    Regex(#[from] regex::Error),

    #[error(transparent)]
    Parser(#[from] ParserError),

    #[error(transparent)]
    Udf(#[from] UDFError),

    #[error("{0}")]
    GenericRegexError(String),
}
