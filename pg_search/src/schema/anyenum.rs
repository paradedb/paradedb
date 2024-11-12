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
