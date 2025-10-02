// Copyright (c) 2023-2025 ParadeDB, Inc.
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

#[derive(Debug, Clone, Arbitrary)]
pub enum ScalarArrayOperator {
    In,
    NotIn,
}

impl ScalarArrayOperator {
    pub fn to_sql(&self) -> &'static str {
        match self {
            ScalarArrayOperator::In => "IN",
            ScalarArrayOperator::NotIn => "NOT IN",
        }
    }
}
