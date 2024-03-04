use crate::datafusion::datatype::PgTypeMod;
use crate::errors::{NotFound, NotSupported, ParadeError};
use deltalake::datafusion::arrow::datatypes::{DECIMAL128_MAX_PRECISION, DECIMAL128_MAX_SCALE};
use pgrx::*;

const NUMERIC_BASE: i128 = 10;

pub struct PgNumeric(pub AnyNumeric, pub NumericTypeMod);
pub struct NumericTypeMod(pub Precision, pub Scale);
pub struct Precision(pub u8);
pub struct Scale(pub i8);

impl TryInto<NumericTypeMod> for PgTypeMod {
    type Error = ParadeError;

    fn try_into(self) -> Result<NumericTypeMod, ParadeError> {
        let PgTypeMod(typemod) = self;
        let max_precision = DECIMAL128_MAX_PRECISION as i32;

        match typemod {
            -1 => Ok(NumericTypeMod(
                Precision(DECIMAL128_MAX_PRECISION),
                Scale(DECIMAL128_MAX_SCALE),
            )),
            _ if typemod >= 0 && typemod <= max_precision => {
                let precision = ((typemod - pg_sys::VARHDRSZ as i32) >> 16) & 0xffff;
                let scale = (((typemod - pg_sys::VARHDRSZ as i32) & 0x7ff) ^ 1024) - 1024;

                Ok(NumericTypeMod(
                    Precision(precision as u8),
                    Scale(scale as i8),
                ))
            }
            _ => Err(NotSupported::NumericPrecision(typemod).into()),
        }
    }
}

impl TryInto<PgTypeMod> for NumericTypeMod {
    type Error = ParadeError;

    fn try_into(self) -> Result<PgTypeMod, ParadeError> {
        let NumericTypeMod(Precision(precision), Scale(scale)) = self;

        Ok(PgTypeMod(
            ((precision as i32) << 16) | (((scale as i32) & 0x7ff) + pg_sys::VARHDRSZ as i32),
        ))
    }
}

impl TryInto<Option<pg_sys::Datum>> for PgNumeric {
    type Error = ParadeError;

    fn try_into(self) -> Result<Option<pg_sys::Datum>, ParadeError> {
        Ok(scale_anynumeric(self, false).into_datum())
    }
}

#[inline]
fn scale_anynumeric(numeric: PgNumeric, scale_down: bool) -> Result<AnyNumeric, ParadeError> {
    let PgNumeric(anynumeric, NumericTypeMod(Precision(precision), Scale(scale))) = numeric;
    let (precision, scale) = (precision as i32, scale as i32);

    let original_typemod =
        (((precision + scale) << 16) | (scale & 0x7ff)) + pg_sys::VARHDRSZ as i32;
    let original_anynumeric: AnyNumeric = unsafe {
        direct_function_call(
            pg_sys::numeric,
            &[
                anynumeric.clone().into_datum(),
                original_typemod.into_datum(),
            ],
        )
        .ok_or(NotFound::Datum(anynumeric.to_string()))?
    };

    // Scale the anynumeric up or down
    let scale_power = if scale_down { scale } else { -scale };
    let scaled_anynumeric: AnyNumeric = if scale_power >= 0 {
        original_anynumeric * NUMERIC_BASE.pow(scale_power as u32)
    } else {
        original_anynumeric / NUMERIC_BASE.pow(-scale_power as u32)
    };

    // Set the expected anynumeric typemod based on scaling direction
    let target_scale = if scale_down { 0 } else { scale };
    let target_typemod = ((precision << 16) | (target_scale & 0x7ff)) + pg_sys::VARHDRSZ as i32;

    unsafe {
        direct_function_call(
            pg_sys::numeric,
            &[scaled_anynumeric.into_datum(), target_typemod.into_datum()],
        )
        .ok_or(NotFound::Datum(anynumeric.to_string()).into())
    }
}
