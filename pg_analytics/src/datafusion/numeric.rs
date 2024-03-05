use crate::datafusion::datatype::PgTypeMod;
use deltalake::datafusion::arrow::datatypes::{DECIMAL128_MAX_PRECISION, DECIMAL128_MAX_SCALE};
use pgrx::*;
use thiserror::Error;

const NUMERIC_BASE: i128 = 10;

pub struct PgNumeric(pub AnyNumeric, pub PgNumericTypeMod);
pub struct PgNumericTypeMod(pub PgPrecision, pub PgScale);
pub struct PgPrecision(pub u8);
pub struct PgScale(pub i8);

impl TryInto<PgTypeMod> for PgNumericTypeMod {
    type Error = NumericError;

    fn try_into(self) -> Result<PgTypeMod, NumericError> {
        let PgNumericTypeMod(PgPrecision(precision), PgScale(scale)) = self;

        Ok(PgTypeMod(
            ((precision as i32) << 16) | (((scale as i32) & 0x7ff) + pg_sys::VARHDRSZ as i32),
        ))
    }
}

impl TryInto<PgNumericTypeMod> for PgTypeMod {
    type Error = NumericError;

    fn try_into(self) -> Result<PgNumericTypeMod, NumericError> {
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
            _ => Err(NumericError::UnsupportedTypeMod(typemod)),
        }
    }
}

impl TryInto<Option<pg_sys::Datum>> for PgNumeric {
    type Error = NumericError;

    fn try_into(self) -> Result<Option<pg_sys::Datum>, NumericError> {
        let PgNumeric(numeric, PgNumericTypeMod(PgPrecision(precision), PgScale(scale))) = self;

        Ok(scale_anynumeric(numeric, precision, scale, false).into_datum())
    }
}

pub trait IntoNumericArray {
    fn into_numeric_array(self, typemod: PgTypeMod) -> Vec<Option<i128>>;
}

impl<T> IntoNumericArray for T
where
    T: Iterator<Item = pg_sys::Datum>,
{
    fn into_numeric_array(self, typemod: PgTypeMod) -> Vec<Option<i128>> {
        let PgNumericTypeMod(PgPrecision(precision), PgScale(scale)) = typemod.try_into().unwrap();

        self.map(|datum| {
            (!datum.is_null()).then_some(datum).and_then(|datum| {
                unsafe { AnyNumeric::from_datum(datum, false) }.map(|numeric| {
                    i128::try_from(scale_anynumeric(numeric, precision, scale, true).unwrap())
                        .unwrap()
                })
            })
        })
        .collect::<Vec<Option<i128>>>()
    }
}

#[inline]
fn scale_anynumeric(
    numeric: AnyNumeric,
    precision: u8,
    scale: i8,
    scale_down: bool,
) -> Result<AnyNumeric, NumericError> {
    let original_typemod = PgNumericTypeMod(PgPrecision(precision), PgScale(scale));
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
}
