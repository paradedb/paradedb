use deltalake::arrow::array::{
    Decimal128Array, Float32Array, Float64Array, Int16Array, Int32Array, Int64Array, UInt32Array,
};
use deltalake::datafusion::arrow::datatypes::{
    DataType, Date32Type, Decimal128Type, Float32Type, Float64Type, Int16Type, Int32Type,
    Int64Type, TimeUnit, TimestampMicrosecondType, UInt32Type, DECIMAL128_MAX_PRECISION,
    DECIMAL128_MAX_SCALE,
};
use deltalake::datafusion::common::arrow::array::{
    Array, ArrayRef, AsArray, BooleanArray, Date32Array, StringArray, Time64MicrosecondArray,
    TimestampMicrosecondArray,
};
use deltalake::datafusion::sql::sqlparser::ast::{
    ArrayElemTypeDef, DataType as SQLDataType, ExactNumberInfo, TimezoneInfo,
};
use pgrx::pg_sys::{BuiltinOid, Datum, VARHDRSZ};
use pgrx::*;
use std::any::type_name;
use std::sync::Arc;

use crate::errors::{NotFound, NotSupported, ParadeError};

use super::array::{
    IntoArray, IntoBooleanListArray, IntoGenericBytesListArray, IntoPrimitiveListArray,
};

pub trait DatafusionTypeTranslator {
    fn to_sql_data_type(&self) -> Result<SQLDataType, ParadeError>;
    fn from_sql_data_type(sql_data_type: SQLDataType) -> Result<Self, ParadeError>
    where
        Self: Sized;
}
impl DatafusionTypeTranslator for DataType {
    fn to_sql_data_type(&self) -> Result<SQLDataType, ParadeError> {
        let result = match self {
            DataType::Boolean => SQLDataType::Boolean,
            DataType::Utf8 => SQLDataType::Text,
            DataType::Int16 => SQLDataType::Int2(None),
            DataType::Int32 => SQLDataType::Int4(None),
            DataType::Int64 => SQLDataType::Int8(None),
            DataType::UInt32 => SQLDataType::UnsignedInt4(None),
            DataType::Float32 => SQLDataType::Float4,
            DataType::Float64 => SQLDataType::Float8,
            DataType::Decimal128(precision, scale) => SQLDataType::Numeric(
                ExactNumberInfo::PrecisionAndScale(*precision as u64, *scale as u64),
            ),
            DataType::Timestamp(TimeUnit::Microsecond, timestamp) => SQLDataType::Timestamp(
                None,
                match timestamp {
                    None => TimezoneInfo::WithoutTimeZone,
                    Some(_) => return Err(NotSupported::DataType(self.clone()).into()),
                },
            ),
            DataType::Date32 => SQLDataType::Date,
            DataType::List(field) => {
                let member_type = field.data_type().to_sql_data_type()?;
                SQLDataType::Array(ArrayElemTypeDef::SquareBracket(Box::new(member_type)))
            }
            unsupported => {
                return Err(ParadeError::NotSupported(NotSupported::DataType(
                    unsupported.clone(),
                )))
            }
        };

        Ok(result)
    }

    fn from_sql_data_type(sql_data_type: SQLDataType) -> Result<DataType, ParadeError> {
        let result = match sql_data_type {
            SQLDataType::Boolean => DataType::Boolean,
            SQLDataType::Text => DataType::Utf8,
            SQLDataType::Int2(_) => DataType::Int16,
            SQLDataType::Int4(_) => DataType::Int32,
            SQLDataType::Int8(_) => DataType::Int64,
            SQLDataType::UnsignedInt4(_) => DataType::UInt32,
            SQLDataType::Float4 => DataType::Float32,
            SQLDataType::Float8 => DataType::Float64,
            SQLDataType::Bytea => DataType::Binary,
            SQLDataType::Numeric(ExactNumberInfo::PrecisionAndScale(precision, scale)) => {
                let casted_precision = precision as u8;
                let casted_scale = scale as i8;

                if casted_precision > DECIMAL128_MAX_PRECISION {
                    return Err(ParadeError::Generic(format!(
                        "Precision {} exceeds max precision {}",
                        casted_precision, DECIMAL128_MAX_PRECISION
                    )));
                }
                if casted_scale > DECIMAL128_MAX_SCALE {
                    return Err(ParadeError::Generic(format!(
                        "Scale {} exceeds max scale {}",
                        casted_scale, DECIMAL128_MAX_SCALE
                    )));
                }
                DataType::Decimal128(casted_precision, casted_scale)
            }
            SQLDataType::Timestamp(_, TimezoneInfo::WithoutTimeZone) => {
                DataType::Timestamp(TimeUnit::Microsecond, None)
            }
            SQLDataType::Date => DataType::Date32,
            SQLDataType::Array(typedef) => match typedef {
                ArrayElemTypeDef::AngleBracket(datatype)
                | ArrayElemTypeDef::SquareBracket(datatype) => {
                    DataType::new_list(Self::from_sql_data_type(*datatype)?, false)
                }
                _none_type => {
                    return Err(ParadeError::Generic(
                        "Unexpected ARRAY 'none' type for SqlDataType::Array".into(),
                    ))
                }
            },
            unsupported => {
                return Err(ParadeError::NotSupported(NotSupported::SQLDataType(
                    unsupported,
                )))
            }
        };

        Ok(result)
    }
}

pub trait PostgresTypeTranslator {
    fn to_sql_data_type(&self, typmod: i32) -> Result<SQLDataType, ParadeError>;
    fn from_sql_data_type(sql_data_type: SQLDataType) -> Result<(Self, i32), ParadeError>
    where
        Self: Sized;
}
impl PostgresTypeTranslator for PgOid {
    fn to_sql_data_type(&self, typmod: i32) -> Result<SQLDataType, ParadeError> {
        let result = match self {
            PgOid::BuiltIn(builtin) => match builtin {
                PgBuiltInOids::BOOLOID => SQLDataType::Boolean,
                PgBuiltInOids::TEXTOID | PgBuiltInOids::VARCHAROID | PgBuiltInOids::BPCHAROID => {
                    SQLDataType::Text
                }
                PgBuiltInOids::INT2OID => SQLDataType::Int2(None),
                PgBuiltInOids::INT4OID => SQLDataType::Int4(None),
                PgBuiltInOids::INT8OID => SQLDataType::Int8(None),
                PgBuiltInOids::FLOAT4OID => SQLDataType::Float4,
                PgBuiltInOids::FLOAT8OID => SQLDataType::Float8,
                PgBuiltInOids::TIMESTAMPOID => {
                    SQLDataType::Timestamp(None, TimezoneInfo::WithoutTimeZone)
                }
                PgBuiltInOids::DATEOID => SQLDataType::Date,
                PgBuiltInOids::BYTEAOID => SQLDataType::Bytea,
                PgBuiltInOids::NUMERICOID => {
                    let scale: i32 = (((typmod - VARHDRSZ as i32) & 0x7ff) ^ 1024) - 1024;
                    let precision: i32 = ((typmod - VARHDRSZ as i32) >> 16) & 0xffff;

                    if precision > DECIMAL128_MAX_PRECISION as i32 {
                        return Err(ParadeError::Generic(format!(
                            "Precision {} exceeds max precision {}",
                            precision, DECIMAL128_MAX_PRECISION
                        )));
                    }
                    if scale > DECIMAL128_MAX_SCALE as i32 {
                        return Err(ParadeError::Generic(format!(
                            "Scale {} exceeds max scale {}",
                            scale, DECIMAL128_MAX_SCALE
                        )));
                    }

                    SQLDataType::Numeric(ExactNumberInfo::PrecisionAndScale(
                        precision as u64,
                        scale as u64,
                    ))
                }
                PgBuiltInOids::VOIDOID => {
                    return Err(ParadeError::NotSupported(
                        NotSupported::BuiltinPostgresType(PgBuiltInOids::VOIDOID),
                    ))
                }
                PgBuiltInOids::INT4RANGEOID => {
                    return Err(ParadeError::NotSupported(
                        NotSupported::BuiltinPostgresType(PgBuiltInOids::INT4RANGEOID),
                    ))
                }
                PgBuiltInOids::INT8RANGEOID => {
                    return Err(ParadeError::NotSupported(
                        NotSupported::BuiltinPostgresType(PgBuiltInOids::INT8RANGEOID),
                    ))
                }
                PgBuiltInOids::NUMRANGEOID => {
                    return Err(ParadeError::NotSupported(
                        NotSupported::BuiltinPostgresType(PgBuiltInOids::NUMRANGEOID),
                    ))
                }
                PgBuiltInOids::DATERANGEOID => {
                    return Err(ParadeError::NotSupported(
                        NotSupported::BuiltinPostgresType(PgBuiltInOids::DATERANGEOID),
                    ))
                }
                PgBuiltInOids::TSRANGEOID => {
                    return Err(ParadeError::NotSupported(
                        NotSupported::BuiltinPostgresType(PgBuiltInOids::TSRANGEOID),
                    ))
                }
                PgBuiltInOids::TSTZRANGEOID => {
                    return Err(ParadeError::NotSupported(
                        NotSupported::BuiltinPostgresType(PgBuiltInOids::TSTZRANGEOID),
                    ))
                }
                PgBuiltInOids::UUIDOID => SQLDataType::Uuid,
                PgBuiltInOids::BOOLARRAYOID => sql_array_type(PgBuiltInOids::BOOLOID, typmod)?,
                PgBuiltInOids::BYTEAARRAYOID => sql_array_type(PgBuiltInOids::BYTEAOID, typmod)?,
                PgBuiltInOids::TEXTARRAYOID => sql_array_type(PgBuiltInOids::TEXTOID, typmod)?,
                PgBuiltInOids::VARCHARARRAYOID => {
                    sql_array_type(PgBuiltInOids::VARCHAROID, typmod)?
                }
                PgBuiltInOids::BPCHARARRAYOID => sql_array_type(PgBuiltInOids::BPCHAROID, typmod)?,
                PgBuiltInOids::INT2ARRAYOID => sql_array_type(PgBuiltInOids::INT2OID, typmod)?,
                PgBuiltInOids::INT4ARRAYOID => sql_array_type(PgBuiltInOids::INT4OID, typmod)?,
                PgBuiltInOids::INT8ARRAYOID => sql_array_type(PgBuiltInOids::INT8OID, typmod)?,
                PgBuiltInOids::OIDARRAYOID => sql_array_type(PgBuiltInOids::OIDOID, typmod)?,
                PgBuiltInOids::XIDARRAYOID => sql_array_type(PgBuiltInOids::XIDOID, typmod)?,
                PgBuiltInOids::FLOAT4ARRAYOID => sql_array_type(PgBuiltInOids::FLOAT4OID, typmod)?,
                PgBuiltInOids::FLOAT8ARRAYOID => sql_array_type(PgBuiltInOids::FLOAT8OID, typmod)?,
                PgBuiltInOids::TIMESTAMPARRAYOID => {
                    sql_array_type(PgBuiltInOids::TIMESTAMPOID, typmod)?
                }
                PgBuiltInOids::DATEARRAYOID => sql_array_type(PgBuiltInOids::DATEOID, typmod)?,
                PgBuiltInOids::NUMERICARRAYOID => {
                    sql_array_type(PgBuiltInOids::NUMERICOID, typmod)?
                }
                PgBuiltInOids::UUIDARRAYOID => sql_array_type(PgBuiltInOids::UUIDOID, typmod)?,
                unsupported => {
                    return Err(ParadeError::NotSupported(
                        NotSupported::BuiltinPostgresType(unsupported.clone()),
                    ))
                }
            },
            PgOid::Invalid => return Err(NotSupported::InvalidPostgresType.into()),
            PgOid::Custom(_) => return Err(NotSupported::CustomPostgresType.into()),
        };

        Ok(result)
    }

    fn from_sql_data_type(sql_data_type: SQLDataType) -> Result<(PgOid, i32), ParadeError> {
        let oid = match sql_data_type {
            SQLDataType::Boolean => PgBuiltInOids::BOOLOID,
            SQLDataType::Text => PgBuiltInOids::TEXTOID,
            SQLDataType::Int2(_) => PgBuiltInOids::INT2OID,
            SQLDataType::Int4(_) => PgBuiltInOids::INT4OID,
            SQLDataType::Int8(_) => PgBuiltInOids::INT8OID,
            SQLDataType::Float4 => PgBuiltInOids::FLOAT4OID,
            SQLDataType::Float8 => PgBuiltInOids::FLOAT8OID,
            SQLDataType::Numeric(ExactNumberInfo::PrecisionAndScale(_precision, _scale)) => {
                PgBuiltInOids::NUMERICOID
            }
            SQLDataType::Timestamp(_, TimezoneInfo::WithoutTimeZone) => PgBuiltInOids::TIMESTAMPOID,
            SQLDataType::Date => PgBuiltInOids::DATEOID,
            SQLDataType::Array(ArrayElemTypeDef::SquareBracket(ref array_data_type)) => {
                match **array_data_type {
                    SQLDataType::Boolean => PgBuiltInOids::BOOLARRAYOID,
                    SQLDataType::Text => PgBuiltInOids::TEXTARRAYOID,
                    SQLDataType::Int2(_) => PgBuiltInOids::INT2ARRAYOID,
                    SQLDataType::Int4(_) => PgBuiltInOids::INT4ARRAYOID,
                    SQLDataType::Int8(_) => PgBuiltInOids::INT8ARRAYOID,
                    SQLDataType::Float4 => PgBuiltInOids::FLOAT4ARRAYOID,
                    SQLDataType::Float8 => PgBuiltInOids::FLOAT8ARRAYOID,
                    SQLDataType::Numeric(ExactNumberInfo::PrecisionAndScale(
                        _precision,
                        _scale,
                    )) => PgBuiltInOids::NUMERICARRAYOID,
                    SQLDataType::Timestamp(_, TimezoneInfo::WithoutTimeZone) => {
                        PgBuiltInOids::TIMESTAMPARRAYOID
                    }
                    SQLDataType::Date => PgBuiltInOids::DATEARRAYOID,
                    _ => return Err(NotSupported::SQLDataType(sql_data_type).into()),
                }
            }
            _ => return Err(NotSupported::SQLDataType(sql_data_type).into()),
        };

        let typmod: i32 = match sql_data_type {
            SQLDataType::Numeric(ExactNumberInfo::PrecisionAndScale(precision, scale)) => {
                (((precision as i32) << 16) | ((scale as i32) & 0x7ff)) + VARHDRSZ as i32
            }
            _ => -1,
        };

        Ok((pgrx::PgOid::BuiltIn(oid), typmod))
    }
}

fn sql_array_type(oid: BuiltinOid, typmod: i32) -> Result<SQLDataType, ParadeError> {
    let sql_type = PgOid::BuiltIn(oid).to_sql_data_type(typmod)?;
    Ok(SQLDataType::Array(ArrayElemTypeDef::SquareBracket(
        Box::new(sql_type),
    )))
}

fn scale_anynumeric(
    anynumeric: AnyNumeric,
    precision: i32,
    original_scale: i32,
    unscale: bool, // true means unscale, false means scale
) -> Result<AnyNumeric, ParadeError> {
    const BASE: i128 = 10;

    // First make sure that numeric arithmetic can handle the full span of values (scaled to unscaled)
    let original_typmod =
        (((precision + original_scale) << 16) | (original_scale & 0x7ff)) + VARHDRSZ as i32;
    let original_anynumeric: AnyNumeric = unsafe {
        direct_function_call(
            pg_sys::numeric,
            &[
                anynumeric.clone().into_datum(),
                original_typmod.into_datum(),
            ],
        )
        .ok_or(NotFound::Datum(anynumeric.to_string()))?
    };

    // Scale the anynumeric up or down
    let scale_power = if unscale {
        original_scale
    } else {
        -original_scale
    };
    let scaled_anynumeric: AnyNumeric = if scale_power >= 0 {
        original_anynumeric * BASE.pow(scale_power as u32)
    } else {
        original_anynumeric / BASE.pow(-scale_power as u32)
    };

    // Set the expected anynumeric typmod based on scaling direction
    let target_scale = if unscale { 0 } else { original_scale };
    let target_typmod = ((precision << 16) | (target_scale & 0x7ff)) + VARHDRSZ as i32;
    unsafe {
        direct_function_call(
            pg_sys::numeric,
            &[scaled_anynumeric.into_datum(), target_typmod.into_datum()],
        )
        .ok_or(NotFound::Datum(anynumeric.to_string()).into())
    }
}
unsafe fn tuple_info(
    slots: *mut *mut pg_sys::TupleTableSlot,
    row_idx: usize,
    col_idx: usize,
) -> (*mut Datum, bool) {
    let tuple_table_slot = *slots.add(row_idx);
    let datum = (*tuple_table_slot).tts_values.add(col_idx);
    let is_null = *(*tuple_table_slot).tts_isnull.add(col_idx);

    (datum, is_null)
}

fn tuple_data<T: FromDatum>(
    slots: *mut *mut pg_sys::TupleTableSlot,
    nslots: usize,
    col_idx: usize,
) -> Vec<Option<T>> {
    (0..nslots)
        .map(move |row_idx| unsafe { tuple_info(slots, row_idx, col_idx) })
        .map(|(datum, is_null)| {
            (!is_null)
                .then_some(datum)
                .and_then(|datum| unsafe { T::from_datum(*datum, false) })
        })
        .collect()
}

pub struct DatafusionMapProducer;
impl DatafusionMapProducer {
    pub fn array(
        sql_data_type: SQLDataType,
        is_array: bool,
        slots: *mut *mut pg_sys::TupleTableSlot,
        nslots: usize,
        col_idx: usize,
    ) -> Result<ArrayRef, ParadeError> {
        let datafusion_type = DatafusionTypeTranslator::from_sql_data_type(sql_data_type)?;

        match datafusion_type {
            DataType::Boolean if !is_array => Ok(Arc::new(
                tuple_data::<bool>(slots, nslots, col_idx).into_array(),
            )),
            DataType::Boolean if is_array => Ok(Arc::new(
                tuple_data::<Vec<Option<bool>>>(slots, nslots, col_idx).into_array(),
            )),
            DataType::Utf8 if !is_array => Ok(Arc::new(
                tuple_data::<String>(slots, nslots, col_idx).into_array(),
            )),
            DataType::Utf8 if is_array => Ok(Arc::new(
                tuple_data::<Vec<Option<String>>>(slots, nslots, col_idx).into_array(),
            )),
            DataType::Int16 if !is_array => Ok(Arc::new(
                tuple_data::<i16>(slots, nslots, col_idx).into_array(),
            )),
            DataType::Int16 if is_array => Ok(Arc::new(
                tuple_data::<Vec<Option<i16>>>(slots, nslots, col_idx).into_array(),
            )),
            DataType::Int32 if !is_array => Ok(Arc::new(
                tuple_data::<i32>(slots, nslots, col_idx).into_array(),
            )),
            DataType::Int32 if is_array => Ok(Arc::new(
                tuple_data::<Vec<Option<i32>>>(slots, nslots, col_idx).into_array(),
            )),
            DataType::Int64 if !is_array => Ok(Arc::new(
                tuple_data::<i64>(slots, nslots, col_idx).into_array(),
            )),
            DataType::Int64 if is_array => {
                Ok(Arc::new(IntoPrimitiveListArray::<Int64Type>::into_array(
                    tuple_data::<Vec<Option<i64>>>(slots, nslots, col_idx),
                )))
            }
            DataType::UInt32 if !is_array => Ok(Arc::new(
                tuple_data::<u32>(slots, nslots, col_idx).into_array(),
            )),
            DataType::UInt32 if is_array => Ok(Arc::new(
                tuple_data::<Vec<Option<u32>>>(slots, nslots, col_idx).into_array(),
            )),
            DataType::Float32 if !is_array => Ok(Arc::new(
                tuple_data::<f32>(slots, nslots, col_idx).into_array(),
            )),
            DataType::Float32 if is_array => Ok(Arc::new(
                tuple_data::<Vec<Option<f32>>>(slots, nslots, col_idx).into_array(),
            )),
            DataType::Float64 if !is_array => Ok(Arc::new(
                tuple_data::<f64>(slots, nslots, col_idx).into_array(),
            )),
            DataType::Float64 if is_array => Ok(Arc::new(
                tuple_data::<Vec<Option<f64>>>(slots, nslots, col_idx).into_array(),
            )),
            DataType::Decimal128(precision, scale) if !is_array => {
                let mut values = vec![];
                for numeric_opt in tuple_data::<AnyNumeric>(slots, nslots, col_idx) {
                    if let Some(numeric) = numeric_opt {
                        let with_scale =
                            scale_anynumeric(numeric, precision as i32, scale as i32, true)?;
                        let new_int = i128::try_from(with_scale)?;
                        values.push(Some(new_int));
                    } else {
                        values.push(None)
                    }
                }
                Ok(Arc::new(
                    values
                        .into_array()
                        .with_precision_and_scale(precision, scale)?,
                ))
            }
            DataType::Decimal128(precision, scale) if is_array => {
                let mut vectors = vec![];
                for vec_opt in tuple_data::<Vec<Option<AnyNumeric>>>(slots, nslots, col_idx) {
                    if let Some(vec) = vec_opt {
                        let mut values = vec![];
                        for numeric_opt in vec {
                            if let Some(numeric) = numeric_opt {
                                let with_scale = scale_anynumeric(
                                    numeric,
                                    precision as i32,
                                    scale as i32,
                                    true,
                                )?;
                                let new_int = i128::try_from(with_scale)?;
                                values.push(Some(new_int));
                            } else {
                                values.push(None)
                            }
                        }
                        vectors.push(Some(values))
                    } else {
                        vectors.push(None);
                    }
                }
                Ok(Arc::new(
                    vectors
                        .into_array()
                        .as_any()
                        .downcast_ref::<Decimal128Array>()
                        .cloned()
                        .ok_or(ParadeError::DowncastGenericArray(datafusion_type))?
                        .with_precision_and_scale(precision, scale)?,
                ))
            }
            // NOTE: should never reach here becaues deltalake schema does not support time
            DataType::Time64(TimeUnit::Microsecond) if !is_array => Ok(Arc::new(
                Time64MicrosecondArray::from(tuple_data::<i64>(slots, nslots, col_idx)),
            )),
            DataType::Time64(TimeUnit::Microsecond) if is_array => Ok(Arc::new(
                IntoPrimitiveListArray::<Int64Type>::into_array(tuple_data::<Vec<Option<i64>>>(
                    slots, nslots, col_idx,
                ))
                .as_any()
                .downcast_ref::<Time64MicrosecondArray>()
                .cloned()
                .ok_or(ParadeError::DowncastGenericArray(datafusion_type))?,
            )),
            DataType::Timestamp(TimeUnit::Microsecond, tz) if !is_array => Ok(Arc::new(
                TimestampMicrosecondArray::from(tuple_data::<i64>(slots, nslots, col_idx))
                    .with_timezone_opt(tz),
            )),
            // TODO: Timestamp arrays are throwing a ParadeError::DowncastGenericArray.
            // DataType::Timestamp(TimeUnit::Microsecond, ref tz) if is_array => Ok(Arc::new(
            //     IntoPrimitiveListArray::<TimestampMicrosecondType>::into_array(tuple_data::<
            //         Vec<Option<i64>>,
            //     >(
            //         slots, nslots, col_idx,
            //     ))
            //     .as_any()
            //     .downcast_ref::<TimestampMicrosecondArray>()
            //     .cloned()
            //     .ok_or(ParadeError::DowncastGenericArray(datafusion_type.clone()))?
            //     .with_timezone_opt(tz.clone()),
            // )),
            DataType::Date32 if !is_array => Ok(Arc::new(Date32Array::from(tuple_data::<i32>(
                slots, nslots, col_idx,
            )))),
            // TODO: Timestamp arrays are throwing a ParadeError::DowncastGenericArray.
            // DataType::Date32 if is_array => Ok(Arc::new(
            //     tuple_data::<Vec<Option<i32>>>(slots, nslots, col_idx)
            //         .into_array()
            //         .as_any()
            //         .downcast_ref::<Date32Array>()
            //         .cloned()
            //         .ok_or(ParadeError::DowncastGenericArray(datafusion_type))?,
            // )),
            _ => Err(NotSupported::DataType(datafusion_type).into()),
        }
    }

    pub fn index_datum(
        sql_data_type: SQLDataType,
        array: &Arc<dyn Array>,
        index: usize,
    ) -> Result<Option<Datum>, ParadeError> {
        let datafusion_type = DatafusionTypeTranslator::from_sql_data_type(sql_data_type)?;

        match datafusion_type {
            DataType::Boolean => array
                .as_any()
                .downcast_ref::<BooleanArray>()
                .ok_or(NotFound::Value(type_name::<BooleanArray>().to_string()))?
                .iter()
                .nth(index)
                .map(|nth| nth.into_datum()),
            DataType::Utf8 => array
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or(NotFound::Value(type_name::<StringArray>().to_string()))?
                .iter()
                .nth(index)
                .map(|nth| nth.into_datum()),
            DataType::Int16 => array
                .as_primitive::<Int16Type>()
                .iter()
                .nth(index)
                .map(|nth| nth.into_datum()),
            DataType::Int32 => array
                .as_primitive::<Int32Type>()
                .iter()
                .nth(index)
                .map(|nth| nth.into_datum()),
            DataType::Int64 => array
                .as_primitive::<Int64Type>()
                .iter()
                .nth(index)
                .map(|nth| nth.into_datum()),
            DataType::UInt32 => array
                .as_primitive::<UInt32Type>()
                .iter()
                .nth(index)
                .map(|nth| nth.into_datum()),
            DataType::Float32 => array
                .as_primitive::<Float32Type>()
                .iter()
                .nth(index)
                .map(|nth| nth.into_datum()),
            DataType::Float64 => array
                .as_primitive::<Float64Type>()
                .iter()
                .nth(index)
                .map(|nth| nth.into_datum()),
            DataType::Decimal128(precision, scale) => array
                .as_primitive::<Decimal128Type>()
                .iter()
                .nth(index)
                .map(|nth| match nth {
                    Some(nth) => {
                        let numeric = AnyNumeric::from(nth);
                        let ret = scale_anynumeric(numeric, precision as i32, scale as i32, false)
                            .ok()?;
                        ret.into_datum()
                    }
                    None => None,
                }),
            DataType::Timestamp(TimeUnit::Microsecond, None) => array
                .as_primitive::<TimestampMicrosecondType>()
                .iter()
                .nth(index)
                .map(|nth| nth.into_datum()),
            DataType::Date32 => array
                .as_primitive::<Date32Type>()
                .iter()
                .nth(index)
                .map(|nth| nth.into_datum()),
            DataType::List(ref field) => {
                let data_type = field.data_type().clone();
                match array.as_list::<i32>().iter().nth(index) {
                    Some(Some(list)) => match &data_type {
                        DataType::Boolean => list
                            .as_any()
                            .downcast_ref::<BooleanArray>()
                            .ok_or(ParadeError::DowncastGenericArray(data_type))?
                            .into_iter()
                            .collect::<Vec<_>>()
                            .into_datum(),
                        DataType::Utf8 => list
                            .as_any()
                            .downcast_ref::<StringArray>()
                            .ok_or(ParadeError::DowncastGenericArray(data_type))?
                            .into_iter()
                            .collect::<Vec<_>>()
                            .into_datum(),
                        DataType::Int16 => list
                            .as_any()
                            .downcast_ref::<Int16Array>()
                            .ok_or(ParadeError::DowncastGenericArray(data_type))?
                            .into_iter()
                            .collect::<Vec<_>>()
                            .into_datum(),
                        DataType::Int32 => list
                            .as_any()
                            .downcast_ref::<Int32Array>()
                            .ok_or(ParadeError::DowncastGenericArray(data_type))?
                            .into_iter()
                            .collect::<Vec<_>>()
                            .into_datum(),
                        DataType::Int64 => list
                            .as_any()
                            .downcast_ref::<Int64Array>()
                            .ok_or(ParadeError::DowncastGenericArray(data_type))?
                            .into_iter()
                            .collect::<Vec<_>>()
                            .into_datum(),
                        DataType::UInt32 => list
                            .as_any()
                            .downcast_ref::<UInt32Array>()
                            .ok_or(ParadeError::DowncastGenericArray(data_type))?
                            .into_iter()
                            .collect::<Vec<_>>()
                            .into_datum(),
                        DataType::Float32 => list
                            .as_any()
                            .downcast_ref::<Float32Array>()
                            .ok_or(ParadeError::DowncastGenericArray(data_type))?
                            .into_iter()
                            .collect::<Vec<_>>()
                            .into_datum(),
                        DataType::Float64 => list
                            .as_any()
                            .downcast_ref::<Float64Array>()
                            .ok_or(ParadeError::DowncastGenericArray(data_type))?
                            .into_iter()
                            .collect::<Vec<_>>()
                            .into_datum(),
                        DataType::Decimal128(precision, scale) => {
                            let mut values = vec![];
                            for numeric_opt in list
                                .as_any()
                                .downcast_ref::<Decimal128Array>()
                                .ok_or_else(|| {
                                    ParadeError::DowncastGenericArray(data_type.clone())
                                })?
                                .into_iter()
                            {
                                if let Some(numeric) = numeric_opt {
                                    let scaled = scale_anynumeric(
                                        AnyNumeric::from(numeric),
                                        precision.clone() as i32,
                                        scale.clone() as i32,
                                        false,
                                    )?;
                                    values.push(scaled)
                                }
                            }
                            values.into_datum()
                        }
                        DataType::Timestamp(TimeUnit::Microsecond, None) => list
                            .as_any()
                            .downcast_ref::<TimestampMicrosecondArray>()
                            .ok_or(ParadeError::DowncastGenericArray(data_type))?
                            .into_iter()
                            .collect::<Vec<_>>()
                            .into_datum(),
                        DataType::Date32 => list
                            .as_any()
                            .downcast_ref::<Date32Array>()
                            .ok_or(ParadeError::DowncastGenericArray(data_type))?
                            .into_iter()
                            .collect::<Vec<_>>()
                            .into_datum(),
                        _ => return Err(NotSupported::DataType(data_type).into()),
                    }
                    .map(|datum| Some(datum)),
                    _ => None,
                }
            }
            _ => return Err(NotSupported::DataType(datafusion_type).into()),
        }
        .ok_or(NotFound::Datum(datafusion_type.to_string()).into())
    }
}
