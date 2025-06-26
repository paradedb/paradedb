use proptest::prelude::*;
use proptest::sample;
use proptest_derive::Arbitrary;

#[derive(Debug, Clone, Arbitrary)]
pub enum Operator {
    Eq, // =
    Ne, // <>
    Lt, // <
    Le, // <=
    Gt, // >
    Ge, // >=
}

impl Operator {
    pub fn to_sql(&self) -> &'static str {
        match self {
            Operator::Eq => "=",
            Operator::Ne => "<>",
            Operator::Lt => "<",
            Operator::Le => "<=",
            Operator::Gt => ">",
            Operator::Ge => ">=",
        }
    }
}

#[derive(Debug, Clone, Arbitrary)]
pub enum ArrayQuantifier {
    Any,
    All,
}

impl ArrayQuantifier {
    pub fn to_sql(&self) -> &'static str {
        match self {
            ArrayQuantifier::Any => "ANY",
            ArrayQuantifier::All => "ALL",
        }
    }
}
