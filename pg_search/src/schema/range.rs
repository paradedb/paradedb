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

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TantivyRange<T> {
    lower: Option<T>,
    upper: Option<T>,
    empty: bool,
    lower_inclusive: bool,
    upper_inclusive: bool,
    lower_unbounded: bool,
    upper_unbounded: bool,
}

pub struct TantivyRangeBuilder<T> {
    lower: Option<T>,
    upper: Option<T>,
    empty: Option<bool>,
    lower_inclusive: Option<bool>,
    upper_inclusive: Option<bool>,
    lower_unbounded: Option<bool>,
    upper_unbounded: Option<bool>,
}

impl<T> TantivyRangeBuilder<T>
where
    T: Clone,
{
    pub fn new() -> Self {
        Self {
            lower: None,
            upper: None,
            empty: None,
            lower_inclusive: None,
            upper_inclusive: None,
            lower_unbounded: None,
            upper_unbounded: None,
        }
    }

    pub fn empty(mut self, empty: bool) -> Self {
        self.empty = Some(empty);
        self
    }

    pub fn lower(mut self, lower: Option<T>) -> Self {
        self.lower = lower;
        self
    }

    pub fn upper(mut self, upper: Option<T>) -> Self {
        self.upper = upper;
        self
    }

    pub fn lower_inclusive(mut self, lower_inclusive: bool) -> Self {
        self.lower_inclusive = Some(lower_inclusive);
        self
    }

    pub fn upper_inclusive(mut self, upper_inclusive: bool) -> Self {
        self.upper_inclusive = Some(upper_inclusive);
        self
    }

    pub fn lower_unbounded(mut self, lower_unbounded: bool) -> Self {
        self.lower_unbounded = Some(lower_unbounded);
        self
    }

    pub fn upper_unbounded(mut self, upper_unbounded: bool) -> Self {
        self.upper_unbounded = Some(upper_unbounded);
        self
    }

    pub fn build(self) -> TantivyRange<T> {
        TantivyRange {
            lower: self.lower,
            upper: self.upper,
            empty: self.empty.unwrap_or(false),
            lower_inclusive: self.lower_inclusive.unwrap_or(true),
            upper_inclusive: self.upper_inclusive.unwrap_or(false),
            lower_unbounded: self.lower_unbounded.unwrap_or(false),
            upper_unbounded: self.upper_unbounded.unwrap_or(false),
        }
    }
}
