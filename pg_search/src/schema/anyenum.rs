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

use crate::pgrx_sql_entity_graph::metadata::{
    ArgumentError, Returns, ReturnsError, SqlMapping, SqlTranslatable,
};
use pgrx::*;
use std::ffi::CStr;
use std::fmt::{Display, Formatter};

pub struct AnyEnum {
    datum: pg_sys::Datum,
    typoid: pg_sys::Oid,
}

impl FromDatum for AnyEnum {
    unsafe fn from_polymorphic_datum(
        datum: pg_sys::Datum,
        is_null: bool,
        typoid: pg_sys::Oid,
    ) -> Option<Self> {
        if is_null {
            None
        } else {
            Some(AnyEnum { datum, typoid })
        }
    }
}

unsafe impl SqlTranslatable for AnyEnum {
    fn argument_sql() -> Result<SqlMapping, ArgumentError> {
        Ok(SqlMapping::As("anyenum".into()))
    }

    fn return_sql() -> Result<Returns, ReturnsError> {
        Ok(Returns::One(SqlMapping::literal("anyenum")))
    }
}

unsafe impl<'fcx> callconv::ArgAbi<'fcx> for AnyEnum
where
    Self: 'fcx,
{
    unsafe fn unbox_arg_unchecked(arg: callconv::Arg<'_, 'fcx>) -> Self {
        let index = arg.index();
        unsafe {
            arg.unbox_arg_using_from_datum()
                .unwrap_or_else(|| panic!("argument {index} must not be null"))
        }
    }
}

impl Display for AnyEnum {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        unsafe {
            let strval = pg_sys::OidOutputFunctionCall(self.typoid, self.datum);
            CStr::from_ptr(strval).to_str().unwrap().fmt(f)
        }
    }
}

impl AnyEnum {
    pub fn ordinal(&self) -> Option<f32> {
        match unsafe { pg_sys::Oid::from_datum(self.datum, self.datum.is_null()) } {
            Some(oid) => {
                let (_, _, ordinal) = enum_helper::lookup_enum_by_oid(oid);
                Some(ordinal)
            }
            None => None,
        }
    }
}
