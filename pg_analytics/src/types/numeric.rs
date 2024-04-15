use crate::types::datatype::PgTypeMod;
use deltalake::datafusion::arrow::datatypes::{DECIMAL128_MAX_PRECISION, DECIMAL128_MAX_SCALE};
use pgrx::*;
use thiserror::Error;

const NUMERIC_BASE: i128 = 10;

#[derive(Clone, Debug)]
pub struct PgNumeric(pub AnyNumeric, pub PgNumericTypeMod);

#[derive(Copy, Clone, Debug)]
pub struct PgNumericTypeMod(pub PgPrecision, pub PgScale);

#[derive(Copy, Clone, Debug)]
pub struct PgPrecision(pub u8);

#[derive(Copy, Clone, Debug)]
pub struct PgScale(pub i8);

impl TryFrom<PgNumericTypeMod> for PgTypeMod {
    type Error = NumericError;

    fn try_from(typemod: PgNumericTypeMod) -> Result<Self, Self::Error> {
        let PgNumericTypeMod(PgPrecision(precision), PgScale(scale)) = typemod;

        Ok(PgTypeMod(
            ((precision as i32) << 16) | (((scale as i32) & 0x7ff) + pg_sys::VARHDRSZ as i32),
        ))
    }
}

impl TryFrom<PgTypeMod> for PgNumericTypeMod {
    type Error = NumericError;

    fn try_from(typemod: PgTypeMod) -> Result<Self, Self::Error> {
        let PgTypeMod(typemod) = typemod;

        match typemod {
            -1 => Err(NumericError::UnboundedNumeric),
            _ => {
                let precision = ((typemod - pg_sys::VARHDRSZ as i32) >> 16) & 0xffff;
                let scale = (((typemod - pg_sys::VARHDRSZ as i32) & 0x7ff) ^ 1024) - 1024;

                if precision > DECIMAL128_MAX_PRECISION.into() {
                    return Err(NumericError::UnsupportedPrecision(precision));
                }

                if scale > DECIMAL128_MAX_SCALE.into() {
                    return Err(NumericError::UnsupportedScale(scale));
                }

                Ok(PgNumericTypeMod(
                    PgPrecision(precision as u8),
                    PgScale(scale as i8),
                ))
            }
        }
    }
}

impl TryFrom<PgNumeric> for AnyNumeric {
    type Error = NumericError;

    fn try_from(numeric: PgNumeric) -> Result<Self, Self::Error> {
        let PgNumeric(numeric, PgNumericTypeMod(PgPrecision(precision), PgScale(scale))) = numeric;
        scale_anynumeric(numeric, precision, scale, false)
    }
}

pub fn scale_anynumeric(
    numeric: AnyNumeric,
    precision: u8,
    scale: i8,
    scale_down: bool,
) -> Result<AnyNumeric, NumericError> {
    let original_typemod = PgNumericTypeMod(PgPrecision(precision + (scale as u8)), PgScale(scale));
    let PgTypeMod(original_pg_typemod) = original_typemod.try_into()?;

    let original_anynumeric: AnyNumeric = unsafe {
        direct_function_call(
            pg_sys::numeric,
            &[
                numeric.clone().into_datum(),
                original_pg_typemod.into_datum(),
            ],
        )
        .ok_or(NumericError::ConvertNumeric(numeric.to_string()))?
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
            &[
                scaled_anynumeric.clone().into_datum(),
                new_pg_typemod.into_datum(),
            ],
        )
        .ok_or(NumericError::ConvertNumeric(scaled_anynumeric.to_string()))
    }
}

#[derive(Error, Debug)]
pub enum NumericError {
    #[error("Failed to convert {0} to numeric")]
    ConvertNumeric(String),

    #[error("Unsupported typemod {0}")]
    UnsupportedTypeMod(i32),

    #[error("Precision {0} exceeds max precision {}", DECIMAL128_MAX_PRECISION)]
    UnsupportedPrecision(i32),

    #[error("Scale {0} exceeds max scale {}", DECIMAL128_MAX_SCALE)]
    UnsupportedScale(i32),

    #[error("Unbounded numeric types are not yet supported. A precision and scale must be provided, i.e. numeric(precision, scale).")]
    UnboundedNumeric,
}
