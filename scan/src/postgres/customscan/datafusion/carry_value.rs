// Copyright (c) 2023-2026 ParadeDB, Inc.
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

//! `pdb_carry_value`: the value of a column that the group keys decide.
//!
//! When the keys of a `GROUP BY` hold a table's primary key, every other column of that
//! table has one value per group (`check_functional_grouping`). The Aggregate Scan reads
//! such a column with this aggregate instead of grouping on it, and any row of the group
//! gives the answer.
//!
//! That is what lets a deferred string pass through the aggregate undecoded: every row
//! of a group comes from one table row, so they all carry the same packed term, and the
//! one this aggregate returns decodes to the group's value above the aggregate. Any other
//! aggregate over a packed term would compare the terms instead of the strings, so the
//! late materialization rules let a deferred column into an aggregate only as the input
//! of this one. It is emitted only by the planner, never from user SQL, so it can not be
//! taken for a real `MAX`.
//!
//! The accumulators are `max`'s, which keep one value per group in a typed groups
//! accumulator; since the values of a group are all equal, the largest is any of them.

use std::sync::{Arc, LazyLock};

use arrow_schema::DataType;
use datafusion::error::Result;
use datafusion::functions_aggregate::min_max::Max;
use datafusion::logical_expr::function::AccumulatorArgs;
use datafusion::logical_expr::utils::AggregateOrderSensitivity;
use datafusion::logical_expr::{
    Accumulator, AggregateUDF, AggregateUDFImpl, Expr, GroupsAccumulator, Signature,
};

use super::reject_distinct;

pub const CARRY_VALUE_NAME: &str = "pdb_carry_value";

static CARRY_VALUE: LazyLock<Arc<AggregateUDF>> =
    LazyLock::new(|| Arc::new(AggregateUDF::from(CarryValue::default())));

pub fn carry_value_udaf() -> Arc<AggregateUDF> {
    Arc::clone(&CARRY_VALUE)
}

/// `pdb_carry_value(column)`, for a column the group keys decide.
pub fn carry_value(column: Expr) -> Expr {
    carry_value_udaf().call(vec![column])
}

#[derive(Debug, Default, PartialEq, Eq, Hash)]
pub struct CarryValue {
    inner: Max,
}

impl AggregateUDFImpl for CarryValue {
    fn name(&self) -> &str {
        CARRY_VALUE_NAME
    }

    fn signature(&self) -> &Signature {
        self.inner.signature()
    }

    fn coerce_types(&self, arg_types: &[DataType]) -> Result<Vec<DataType>> {
        self.inner.coerce_types(arg_types)
    }

    fn return_type(&self, arg_types: &[DataType]) -> Result<DataType> {
        Ok(arg_types[0].clone())
    }

    fn accumulator(&self, args: AccumulatorArgs) -> Result<Box<dyn Accumulator>> {
        reject_distinct(&args, CARRY_VALUE_NAME)?;
        self.inner.accumulator(args)
    }

    fn groups_accumulator_supported(&self, args: AccumulatorArgs) -> bool {
        !args.is_distinct && self.inner.groups_accumulator_supported(args)
    }

    fn create_groups_accumulator(
        &self,
        args: AccumulatorArgs,
    ) -> Result<Box<dyn GroupsAccumulator>> {
        self.inner.create_groups_accumulator(args)
    }

    fn order_sensitivity(&self) -> AggregateOrderSensitivity {
        AggregateOrderSensitivity::Insensitive
    }
}
