use crate::nodes::producer::DatafusionExprProducer;
use crate::nodes::t_const::ConstNode;
use crate::nodes::t_var::VarNode;
use datafusion::logical_expr::{BinaryExpr, Expr, Operator};
use pgrx::pg_sys;
use std::ffi::CStr;

pub struct OpExpr;
impl DatafusionExprProducer for OpExpr {
    unsafe fn datafusion_expr(
        node: *mut pg_sys::Node,
        rtable: Option<*mut pg_sys::List>,
    ) -> Result<Expr, String> {
        if let Some(r) = rtable {
            match (*node).type_ {
                pg_sys::NodeTag::T_OpExpr => {
                    let operator_expr = node as *mut pg_sys::OpExpr;
                    let args = (*operator_expr).args;
                    let elements = (*args).elements;

                    // Args len can be 1 or 2
                    if (*args).length == 1 {
                        Ok(OpExpr::datafusion_expr(node, Some(r))?)
                    } else {
                        let larg = (*elements.offset(0)).ptr_value as *mut pg_sys::Node;
                        let rarg = (*elements.offset(1)).ptr_value as *mut pg_sys::Node;

                        // Get operator
                        let operator_tuple = pg_sys::SearchSysCache1(
                            pg_sys::SysCacheIdentifier_OPEROID as i32,
                            pg_sys::Datum::from((*operator_expr).opno),
                        );
                        let operator_form =
                            pg_sys::GETSTRUCT(operator_tuple) as *mut pg_sys::FormData_pg_operator;
                        let operator_name = CStr::from_ptr((*operator_form).oprname.data.as_ptr())
                            .to_string_lossy()
                            .into_owned();

                        // Make sure to avoid cache ref leaks
                        pg_sys::ReleaseSysCache(operator_tuple);

                        // Recursively get expressions from left and right sides of the operation
                        let larg_expr = OpExpr::datafusion_expr(larg, Some(r))?;
                        let rarg_expr = OpExpr::datafusion_expr(rarg, Some(r))?;

                        Ok(Expr::BinaryExpr(BinaryExpr {
                            left: Box::new(larg_expr),
                            right: Box::new(rarg_expr),
                            op: match operator_name.as_str() {
                                // Arithmetic Operators
                                "+" => Operator::Plus,
                                "-" => Operator::Minus,
                                "*" => Operator::Multiply,
                                "/" => Operator::Divide,
                                "%" => Operator::Modulo,

                                // Comparison Operators
                                "=" => Operator::Eq,
                                "<>" => Operator::NotEq,
                                "<" => Operator::Lt,
                                ">" => Operator::Gt,
                                "<=" => Operator::LtEq,
                                ">=" => Operator::GtEq,

                                // Logical Operators
                                "AND" => Operator::And,
                                "OR" => Operator::Or,

                                // Bitwise Operators
                                "&" => Operator::BitwiseAnd,
                                "|" => Operator::BitwiseOr,
                                "#" => Operator::BitwiseXor,
                                "<<" => Operator::BitwiseShiftLeft,
                                ">>" => Operator::BitwiseShiftRight,

                                // String Operators
                                "||" => Operator::StringConcat,

                                _ => {
                                    return Err(format!(
                                        "operator {} not supported yet",
                                        operator_name
                                    ))
                                }
                            },
                        }))
                    }
                }
                pg_sys::NodeTag::T_Var => Ok(VarNode::datafusion_expr(node, Some(r))?),
                pg_sys::NodeTag::T_Const => Ok(ConstNode::datafusion_expr(node, Some(r))?),
                _ => Err(format!("Unsupported NodeTag {:?}", (*node).type_)),
            }
        } else {
            Err("No range table provided".into())
        }
    }
}
