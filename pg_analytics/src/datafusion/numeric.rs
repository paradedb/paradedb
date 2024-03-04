use crate::datafusion::datatype::PgTypeMod;
use crate::errors::{NotFound, NotSupported, ParadeError};
use deltalake::datafusion::arrow::datatypes::{DECIMAL128_MAX_PRECISION, DECIMAL128_MAX_SCALE};
use pgrx::*;

const NUMERIC_BASE: i128 = 10;

pub struct PgNumeric(pub AnyNumeric, pub PgNumericTypeMod);
pub struct PgNumericTypeMod(pub PgPrecision, pub PgScale);
pub struct PgPrecision(pub u8);
pub struct PgScale(pub i8);

impl TryInto<PgTypeMod> for PgNumericTypeMod {
    type Error = ParadeError;

    fn try_into(self) -> Result<PgTypeMod, ParadeError> {
        let PgNumericTypeMod(PgPrecision(precision), PgScale(scale)) = self;

        Ok(PgTypeMod(
            ((precision as i32) << 16) | (((scale as i32) & 0x7ff) + pg_sys::VARHDRSZ as i32),
        ))
    }
}

impl TryInto<PgNumericTypeMod> for PgTypeMod {
    type Error = ParadeError;

    fn try_into(self) -> Result<PgNumericTypeMod, ParadeError> {
        let PgTypeMod(typemod) = self;

        match typemod {
            -1 => Ok(PgNumericTypeMod(
                PgPrecision(DECIMAL128_MAX_PRECISION),
                PgScale(DECIMAL128_MAX_SCALE),
            )),
            _ if typemod >= 0 && typemod <= DECIMAL128_MAX_PRECISION as i32 => {
                let precision = ((typemod - pg_sys::VARHDRSZ as i32) >> 16) & 0xffff;
                let scale = (((typemod - pg_sys::VARHDRSZ as i32) & 0x7ff) ^ 1024) - 1024;

                Ok(PgNumericTypeMod(
                    PgPrecision(precision as u8),
                    PgScale(scale as i8),
                ))
            }
            _ => Err(NotSupported::NumericPrecision(typemod).into()),
        }
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
    let PgNumeric(anynumeric, original_typemod) = numeric;
    let PgNumericTypeMod(PgPrecision(precision), PgScale(scale)) = original_typemod;
    let PgTypeMod(original_pg_typemod) = original_typemod.try_into()?;

    let original_anynumeric: AnyNumeric = unsafe {
        direct_function_call(
            pg_sys::numeric,
            &[
                anynumeric.clone().into_datum(),
                original_pg_typemod.into_datum(),
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
    let target_typemod = PgNumericTypeMod(PgPrecision(precision), PgScale(target_scale));
    let PgTypeMod(new_pg_typemod) = target_typemod.try_into()?;

    unsafe {
        direct_function_call(
            pg_sys::numeric,
            &[scaled_anynumeric.into_datum(), new_pg_typemod.into_datum()],
        )
        .ok_or(NotFound::Datum(anynumeric.to_string()).into())
    }
}
