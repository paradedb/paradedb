use deltalake::datafusion::common::ScalarValue;
use deltalake::datafusion::error::DataFusionError;
use deltalake::datafusion::logical_expr::expr::ScalarFunction;
use deltalake::datafusion::logical_expr::{Expr, ScalarFunctionDefinition};
use deltalake::datafusion::sql::parser::{self, DFParser};
use deltalake::datafusion::sql::planner::SqlToRel;
use deltalake::datafusion::sql::sqlparser::dialect::PostgreSqlDialect;
use regex::Regex;
use std::collections::VecDeque;

use crate::datafusion::context::QueryContext;
use crate::datafusion::plan::LogicalPlanDetails;
use crate::datafusion::udf::loadfunction;
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
impl TryFrom<QueryString<'_>> for LogicalPlanDetails {
    type Error = ParadeError;

    fn try_from(query: QueryString) -> Result<Self, Self::Error> {
        let QueryString(query) = query;

        let dialect = PostgreSqlDialect {};
        let ast = DFParser::parse_sql_with_dialect(query, &dialect)
            .map_err(|err| ParadeError::DataFusion(DataFusionError::SQL(err, None)))?;
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
                        .ok_or(DataFusionError::Plan(err_string.clone()))?
                        .get(1)
                        .ok_or(DataFusionError::Plan(err_string.clone()))?
                        .as_str();

                    // If we are unable to load the function, we push the error up, breaking the loop
                    unsafe { loadfunction(missing_func_name)? };

                    // Loop again
                }
                Err(err) => return Err(ParadeError::DataFusion(err)),
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

        Ok(LogicalPlanDetails {
            logical_plan: new_logical_plan,
            includes_udf,
        })
    }
}
