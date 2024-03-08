use deltalake::datafusion::common::ScalarValue;
use deltalake::datafusion::error::DataFusionError;
use deltalake::datafusion::logical_expr::{Expr, LogicalPlan, ScalarFunctionDefinition};
use deltalake::datafusion::logical_expr::expr::ScalarFunction;
use deltalake::datafusion::sql::parser::{self, DFParser};
use deltalake::datafusion::sql::planner::SqlToRel;
use deltalake::datafusion::sql::sqlparser::dialect::PostgreSqlDialect;
use pgrx::*;
use regex::Regex;
use std::collections::VecDeque;
use std::ffi::CStr;

use crate::datafusion::context::QueryContext;
use crate::errors::ParadeError;
use crate::hooks::createfunction::loadfunction;

pub trait Query {
    // Extracts the query string from a PlannedStmt,
    // accounting for multi-line queries where we only want a
    // specific line of the entire query.
    fn get_query_string(self, source_text: &CStr) -> Result<String, ParadeError>;

    // Parses the query string into an AST
    fn get_ast(self, query_string: &str) -> Result<VecDeque<parser::Statement>, ParadeError>;

    // Parses the query string into a DataFusion LogicalPlan
    fn get_logical_plan(self, query_string: &str) -> Result<(LogicalPlan, bool), ParadeError>;
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

    fn get_logical_plan(self, query: &str) -> Result<(LogicalPlan, bool), ParadeError> {
        let dialect = PostgreSqlDialect {};
        let ast = DFParser::parse_sql_with_dialect(query, &dialect)
            .map_err(|err| ParadeError::DataFusion(DataFusionError::SQL(err, None)))?;
        let statement = &ast[0];

        // Convert the AST into a logical plan
        let context_provider = QueryContext::new()?;
        let sql_to_rel = SqlToRel::new(&context_provider);

        let logical_plan: LogicalPlan;

        // If functions are undefined, then try to find and register the function and then try to get the plan again
        let re = Regex::new(r"Invalid function '(.+)'")?;
        loop {
            match sql_to_rel.statement_to_plan(statement.clone()) {
                Ok(plan) => {
                    logical_plan = plan;
                    break;
                }
                Err(err) => match err {
                    DataFusionError::Plan(err_string) => {
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
                    _ => return Err(ParadeError::DataFusion(err)),
                },
            };
        }

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

        let mut new_inputs = vec![];
        for input in logical_plan.inputs().iter() {
            #[allow(suspicious_double_ref_op)]
            new_inputs.push(input.clone().clone());
        }

        let new_logical_plan = logical_plan.with_new_exprs(new_exprs, new_inputs.as_slice())?;

        Ok((new_logical_plan, includes_udf))
    }
}
