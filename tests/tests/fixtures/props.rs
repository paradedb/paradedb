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

use std::fmt::Debug;

use chrono::{NaiveDate, NaiveDateTime};
use proptest::prelude::*;
use proptest::sample::SizeRange;
use proptest_derive::Arbitrary;

pub trait Row: Arbitrary {
    // Primary key.
    type Key: Copy + Debug + Eq + Ord;

    fn key(&self) -> Self::Key;
}

#[derive(Arbitrary, Debug, Clone)]
pub enum Op<R: Row> {
    Insert(R),
    // TODO: Add `Update` and `Delete`.
    // Update(R::Key, R),
    // Delete(R::Key),
}

impl<R: Row> Op<R> {
    fn key(&self) -> R::Key {
        match self {
            Op::Insert(r) => r.key(),
        }
    }
}

pub fn arb_date() -> impl Strategy<Value = NaiveDate> {
    // TODO: More dates.
    Just(NaiveDate::from_ymd_opt(1969, 1, 1).unwrap())
}

pub fn arb_date_time() -> impl Strategy<Value = NaiveDateTime> {
    // TODO: More datetimes.
    arb_date().prop_map(|d| d.and_hms_opt(1, 1, 1).unwrap())
}

pub fn arb_json_value() -> impl Strategy<Value = serde_json::Value> {
    // TODO: Make recursive with nested structures.
    prop_oneof![
        Just(serde_json::Value::Null),
        any::<bool>().prop_map(serde_json::Value::from),
        any::<i64>().prop_map(|n| serde_json::json!(n)),
        "[a-zA-Z0-9]{0,10}".prop_map(|s| serde_json::json!(s)),
    ]
}

// Note: Will sometimes return fewer than the target size due to id collisions.
pub fn arb_ops<R: Row>(size_range: impl Into<SizeRange>) -> impl Strategy<Value = Vec<Op<R>>> {
    proptest::collection::vec(any::<Op<R>>(), size_range).prop_map(|mut ops| {
        ops.sort_by_key(|op| op.key());
        ops.dedup_by_key(|op| op.key());
        ops
    })
}
