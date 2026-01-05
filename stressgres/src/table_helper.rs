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

use cursive_core::align::HAlign;
use cursive_table_view::TableViewItem;
use postgres::types::{FromSql, Type};
use postgres::Row;
use std::cmp::Ordering;
use std::error::Error;
use std::fmt::{Display, Formatter};

/// A row wrapper so we can display query results in a cursive_table_view.
#[derive(Clone)]
pub struct ArbitraryTableRow(pub Row);

impl TableViewItem<usize> for ArbitraryTableRow {
    fn to_column(&self, column: usize) -> String {
        get_str_value(&self.0, column)
    }

    /// We won't worry about sorting; always return `Ordering::Greater`.
    fn cmp(&self, _other: &Self, _column: usize) -> Ordering
    where
        Self: Sized,
    {
        Ordering::Greater
    }
}

#[allow(clippy::if_same_then_else)]
impl ArbitraryTableRow {
    pub fn halign(&self, i: usize) -> HAlign {
        if self.0.try_get::<_, String>(i).is_ok() {
            HAlign::Left
        } else if self.0.try_get::<_, Option<i64>>(i).is_ok() {
            HAlign::Right
        } else if self.0.try_get::<_, Option<i32>>(i).is_ok() {
            HAlign::Right
        } else if self.0.try_get::<_, Option<f64>>(i).is_ok() {
            HAlign::Right
        } else if self.0.try_get::<_, Option<f32>>(i).is_ok() {
            HAlign::Right
        } else if self.0.try_get::<_, Option<bool>>(i).is_ok() {
            HAlign::Left
        } else if self
            .0
            .try_get::<_, Option<rust_decimal::Decimal>>(i)
            .is_ok()
        {
            HAlign::Right
        } else if self.0.try_get::<_, Option<Vec<String>>>(i).is_ok() {
            HAlign::Left
        } else if self.0.try_get::<_, Option<Xid>>(i).is_ok() {
            HAlign::Right
        } else {
            HAlign::Left
        }
    }

    pub fn col_width(&self, i: usize) -> Option<usize> {
        if self.0.try_get::<_, String>(i).is_ok() {
            None
        } else if self.0.try_get::<_, Option<i64>>(i).is_ok() {
            Some(10)
        } else if self.0.try_get::<_, Option<i32>>(i).is_ok() {
            Some(10)
        } else if self.0.try_get::<_, Option<f64>>(i).is_ok() {
            Some(10)
        } else if self.0.try_get::<_, Option<f32>>(i).is_ok() {
            Some(10)
        } else if self.0.try_get::<_, Option<bool>>(i).is_ok() {
            Some(5)
        } else if self
            .0
            .try_get::<_, Option<rust_decimal::Decimal>>(i)
            .is_ok()
        {
            Some(10)
        } else if self.0.try_get::<_, Option<Vec<String>>>(i).is_ok() {
            None
        } else if self.0.try_get::<_, Option<Xid>>(i).is_ok() {
            Some(10)
        } else {
            None
        }
    }
}

fn get_str_value(row: &Row, i: usize) -> String {
    // Attempt some common types
    if let Ok(v) = row.try_get::<_, Option<String>>(i) {
        v.unwrap_or_default()
    } else if let Ok(v) = row.try_get::<_, Option<i64>>(i) {
        v.map(|x| x.to_string()).unwrap_or_default()
    } else if let Ok(v) = row.try_get::<_, Option<i32>>(i) {
        v.map(|x| x.to_string()).unwrap_or_default()
    } else if let Ok(v) = row.try_get::<_, Option<f64>>(i) {
        v.map(|x| x.to_string()).unwrap_or_default()
    } else if let Ok(v) = row.try_get::<_, Option<f32>>(i) {
        v.map(|x| x.to_string()).unwrap_or_default()
    } else if let Ok(v) = row.try_get::<_, Option<bool>>(i) {
        v.map(|x| x.to_string()).unwrap_or_default()
    } else if let Ok(v) = row.try_get::<_, Option<rust_decimal::Decimal>>(i) {
        // for NUMERIC support
        v.map(|x| x.to_string()).unwrap_or_default()
    } else if let Ok(v) = row.try_get::<_, Option<Vec<String>>>(i) {
        v.map(|x| format!("{x:?}")).unwrap_or_default()
    } else if let Ok(v) = row.try_get::<_, Option<Vec<i32>>>(i) {
        v.map(|x| format!("{x:?}")).unwrap_or_default()
    } else if let Ok(v) = row.try_get::<_, Option<Vec<i64>>>(i) {
        v.map(|x| format!("{x:?}")).unwrap_or_default()
    } else if let Ok(v) = row.try_get::<_, Option<Vec<f32>>>(i) {
        v.map(|x| format!("{x:?}")).unwrap_or_default()
    } else if let Ok(v) = row.try_get::<_, Option<Vec<f64>>>(i) {
        v.map(|x| format!("{x:?}")).unwrap_or_default()
    } else if let Ok(v) = row.try_get::<_, Option<Xid>>(i) {
        v.map(|x| format!("{x}")).unwrap_or_default()
    } else if row.columns()[i].type_().name() == "void" {
        "(void)".to_string()
    } else {
        format!("{}: type unknown", row.columns()[i].type_().name())
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[repr(transparent)]
struct Xid(u32);

impl Display for Xid {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'a> FromSql<'a> for Xid {
    fn from_sql(_ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
        let xid = u32::from_be_bytes(raw.try_into()?);
        Ok(Xid(xid))
    }

    fn accepts(ty: &Type) -> bool {
        ty.name() == "xid"
    }
}
