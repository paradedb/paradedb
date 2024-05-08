#![allow(unused)]
pub mod datatype {
    use arrow::datatypes::DataType::*;
    use arrow::datatypes::*;
    use arrow::error::ArrowError;
    use datafusion::common::DataFusionError;
    use pgrx::pg_sys::BuiltinOid::*;
    use pgrx::*;
    use thiserror::Error;

    use super::date::DateError;
    use super::datum::DatumError;
    use super::numeric::{NumericError, PgNumericTypeMod, PgPrecision, PgScale};
    use super::time::{TimeError, TimePrecision};
    use super::timestamp::{TimestampError, TimestampPrecision};

    // By default, unspecified type mods in Postgres are -1
    pub static DEFAULT_TYPE_MOD: i32 = -1;

    #[derive(Copy, Clone, Debug)]
    pub struct PgTypeMod(pub i32);

    #[derive(Copy, Clone, Debug)]
    pub struct PgAttribute(pub PgOid, pub PgTypeMod);

    #[derive(Clone, Debug)]
    pub struct ArrowDataType(pub DataType);

    impl TryFrom<PgAttribute> for ArrowDataType {
        type Error = DataTypeError;

        fn try_from(attribute: PgAttribute) -> Result<Self, Self::Error> {
            let PgAttribute(oid, typemod) = attribute;

            let datatype = match oid {
                PgOid::BuiltIn(builtin) => match builtin {
                    BOOLOID => Boolean,
                    TEXTOID => Utf8,
                    VARCHAROID => Utf8,
                    BPCHAROID => Utf8,
                    INT2OID => Int16,
                    INT4OID => Int32,
                    INT8OID => Int64,
                    FLOAT4OID => Float32,
                    FLOAT8OID => Float64,
                    DATEOID => Date32,
                    TIMEOID => Time64(TimePrecision::try_from(typemod)?.0),
                    TIMESTAMPOID => Timestamp(TimestampPrecision::try_from(typemod)?.0, None),
                    NUMERICOID => {
                        let PgNumericTypeMod(PgPrecision(precision), PgScale(scale)) =
                            typemod.try_into()?;
                        Decimal128(precision, scale)
                    }
                    UUIDOID => Utf8,
                    unsupported => return Err(DataTypeError::UnsupportedPostgresType(unsupported)),
                },
                PgOid::Invalid => return Err(DataTypeError::InvalidPostgresOid),
                PgOid::Custom(_) => return Err(DataTypeError::UnsupportedCustomType),
            };

            Ok(ArrowDataType(datatype))
        }
    }

    impl TryFrom<ArrowDataType> for PgAttribute {
        type Error = DataTypeError;

        fn try_from(datatype: ArrowDataType) -> Result<Self, Self::Error> {
            let ArrowDataType(datatype) = datatype;

            let result = match datatype {
                Boolean => (BOOLOID, PgTypeMod(DEFAULT_TYPE_MOD)),
                Utf8 => (TEXTOID, PgTypeMod(DEFAULT_TYPE_MOD)),
                Int16 => (INT2OID, PgTypeMod(DEFAULT_TYPE_MOD)),
                Int32 => (INT4OID, PgTypeMod(DEFAULT_TYPE_MOD)),
                Int64 => (INT8OID, PgTypeMod(DEFAULT_TYPE_MOD)),
                Float32 => (FLOAT4OID, PgTypeMod(DEFAULT_TYPE_MOD)),
                Float64 => (FLOAT8OID, PgTypeMod(DEFAULT_TYPE_MOD)),
                Date32 => (DATEOID, PgTypeMod(DEFAULT_TYPE_MOD)),
                Time64(timeunit) => (TIMEOID, PgTypeMod::try_from(TimePrecision(timeunit))?),
                Timestamp(timeunit, None) => (
                    TIMESTAMPOID,
                    PgTypeMod::try_from(TimestampPrecision(timeunit))?,
                ),
                Decimal128(precision, scale) => (
                    NUMERICOID,
                    PgTypeMod::try_from(PgNumericTypeMod(PgPrecision(precision), PgScale(scale)))?,
                ),
                List(ref field) => match field.data_type() {
                    Boolean => (BOOLARRAYOID, PgTypeMod(DEFAULT_TYPE_MOD)),
                    Utf8 => (TEXTARRAYOID, PgTypeMod(DEFAULT_TYPE_MOD)),
                    Int16 => (INT2ARRAYOID, PgTypeMod(DEFAULT_TYPE_MOD)),
                    Int32 => (INT4ARRAYOID, PgTypeMod(DEFAULT_TYPE_MOD)),
                    Int64 => (INT8ARRAYOID, PgTypeMod(DEFAULT_TYPE_MOD)),
                    Float32 => (FLOAT4ARRAYOID, PgTypeMod(DEFAULT_TYPE_MOD)),
                    Float64 => (FLOAT8ARRAYOID, PgTypeMod(DEFAULT_TYPE_MOD)),
                    Date32 => (DATEARRAYOID, PgTypeMod(DEFAULT_TYPE_MOD)),
                    unsupported => {
                        return Err(DataTypeError::UnsupportedArrowArrayType(
                            unsupported.clone(),
                        ))
                    }
                },
                unsupported => return Err(DataTypeError::UnsupportedArrowType(unsupported)),
            };

            Ok(PgAttribute(PgOid::BuiltIn(result.0), result.1))
        }
    }

    #[derive(Error, Debug)]
    pub enum DataTypeError {
        #[error(transparent)]
        Arrow(#[from] ArrowError),

        #[error(transparent)]
        DataFusion(#[from] DataFusionError),

        #[error(transparent)]
        Date(#[from] DateError),

        #[error(transparent)]
        Datum(#[from] DatumError),

        #[error(transparent)]
        Time(#[from] TimeError),

        #[error(transparent)]
        Timestamp(#[from] TimestampError),

        #[error(transparent)]
        Numeric(#[from] NumericError),

        #[error("Invalid Postgres OID")]
        InvalidPostgresOid,

        #[error("Postgres type {0:?} is not yet supported")]
        UnsupportedPostgresType(PgBuiltInOids),

        #[error("Custom Postgres types are not supported")]
        UnsupportedCustomType,

        #[error("Could not convert arrow type {0:?} to Postgres type")]
        UnsupportedArrowType(DataType),

        #[error("Could not convert arrow array with type {0:?} to Postgres array")]
        UnsupportedArrowArrayType(DataType),

        #[error("Datums converted to &str should be valid UTF-8")]
        InvalidUTF8,
    }
}

mod date {
    use chrono::{Datelike, Duration, NaiveDate};
    use pgrx::*;
    use thiserror::Error;

    const EPOCH_YEAR: i32 = 1970;
    const EPOCH_MONTH: u32 = 1;
    const EPOCH_DAY: u32 = 1;

    #[derive(Copy, Clone, Debug)]
    pub struct DayUnix(pub i32);

    impl TryFrom<datum::Date> for DayUnix {
        type Error = DateError;

        fn try_from(date: datum::Date) -> Result<Self, Self::Error> {
            Ok(DayUnix(date.to_unix_epoch_days()))
        }
    }

    impl TryFrom<DayUnix> for datum::Date {
        type Error = DateError;

        fn try_from(day: DayUnix) -> Result<Self, Self::Error> {
            let DayUnix(days_since_epoch) = day;
            let epoch = NaiveDate::from_ymd_opt(EPOCH_YEAR, EPOCH_MONTH, EPOCH_DAY)
                .ok_or(DateError::InvalidEpoch)?;
            let date = epoch + Duration::days(days_since_epoch.into());

            Ok(datum::Date::new(
                date.year(),
                date.month() as u8,
                date.day() as u8,
            )?)
        }
    }

    #[derive(Error, Debug)]
    pub enum DateError {
        #[error(transparent)]
        DateTimeConversion(#[from] datum::datetime_support::DateTimeConversionError),

        #[error("Failed to set epoch {}-{}-{}", EPOCH_YEAR, EPOCH_MONTH, EPOCH_DAY)]
        InvalidEpoch,
    }
}

mod datum {
    use deltalake::arrow::array::{Array, *};
    use deltalake::arrow::datatypes::*;
    use deltalake::datafusion::arrow::datatypes::DataType::*;
    use deltalake::datafusion::common::{downcast_value, DataFusionError};
    use pgrx::pg_sys::BuiltinOid::*;
    use pgrx::*;
    use std::fmt::Debug;
    use thiserror::Error;

    use super::datatype::DataTypeError;
    use super::date::DayUnix;
    use super::numeric::{PgNumeric, PgNumericTypeMod, PgPrecision, PgScale};
    use super::time::NanosecondDay;
    use super::timestamp::{MicrosecondUnix, MillisecondUnix, SecondUnix};

    pub trait GetDatumPrimitive
    where
        Self: Array + AsArray,
    {
        fn get_primitive_datum<A>(&self, index: usize) -> Result<Option<pg_sys::Datum>, DatumError>
        where
            A: Array + Debug + 'static,
            for<'a> &'a A: ArrayAccessor + IntoIterator,
            for<'a> <&'a A as ArrayAccessor>::Item: IntoDatum,
        {
            let downcast_array = downcast_value!(self, A);
            match downcast_array.is_null(index) {
                false => Ok(downcast_array.value(index).into_datum()),
                true => Ok(None),
            }
        }
    }

    pub trait GetDatumUInt
    where
        Self: Array + AsArray,
    {
        fn get_uint_datum<A>(&self, index: usize) -> Result<Option<pg_sys::Datum>, DatumError>
        where
            A: ArrowPrimitiveType,
            i64: TryFrom<A::Native>,
        {
            let downcast_array = self.as_primitive::<A>();
            match downcast_array.is_null(index) {
                false => {
                    let value: A::Native = downcast_array.value(index);
                    Ok(i64::try_from(value)
                        .map_err(|_| DatumError::UIntConversionError)?
                        .into_datum())
                }
                true => Ok(None),
            }
        }
    }

    pub trait GetDatumPrimitiveList
    where
        Self: Array + AsArray,
    {
        fn get_primitive_list_datum<A>(
            &self,
            index: usize,
        ) -> Result<Option<pg_sys::Datum>, DatumError>
        where
            A: Array + Debug + 'static,
            for<'a> &'a A: IntoIterator,
            for<'a> <&'a A as IntoIterator>::Item: IntoDatum,
        {
            let downcast_array = self.as_list::<i32>();
            match downcast_array.is_null(index) {
                false => Ok(downcast_value!(downcast_array.value(index), A)
                    .into_iter()
                    .collect::<Vec<_>>()
                    .into_datum()),
                true => Ok(None),
            }
        }
    }

    pub trait GetDatumDate
    where
        Self: Array + AsArray,
    {
        fn get_date_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, DataTypeError> {
            let downcast_array = downcast_value!(self, Date32Array);
            match downcast_array.is_null(index) {
                false => {
                    Ok(datum::Date::try_from(DayUnix(downcast_array.value(index))).into_datum())
                }
                true => Ok(None),
            }
        }
    }

    pub trait GetDatumDateFromInt32
    where
        Self: Array + AsArray,
    {
        fn get_date_from_int32_datum(
            &self,
            index: usize,
        ) -> Result<Option<pg_sys::Datum>, DataTypeError> {
            let downcast_array = downcast_value!(self, Int32Array);
            let date_array = downcast_array.reinterpret_cast::<Date32Type>();

            match date_array.nulls().is_some() && date_array.is_null(index) {
                false => Ok(datum::Date::try_from(DayUnix(date_array.value(index))).into_datum()),
                true => Ok(None),
            }
        }
    }

    pub trait GetDatumTimestampMicrosecond
    where
        Self: Array + AsArray,
    {
        fn get_ts_micro_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, DataTypeError> {
            let downcast_array = downcast_value!(self, TimestampMicrosecondArray);
            match downcast_array.is_null(index) {
                false => Ok(datum::Timestamp::try_from(MicrosecondUnix(
                    downcast_array.value(index),
                ))
                .into_datum()),
                true => Ok(None),
            }
        }
    }

    pub trait GetDatumTimestampMillisecond
    where
        Self: Array + AsArray,
    {
        fn get_ts_milli_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, DataTypeError> {
            let downcast_array = downcast_value!(self, TimestampMillisecondArray);
            match downcast_array.is_null(index) {
                false => Ok(datum::Timestamp::try_from(MillisecondUnix(
                    downcast_array.value(index),
                ))
                .into_datum()),
                true => Ok(None),
            }
        }
    }

    pub trait GetDatumTimestampSecond
    where
        Self: Array + AsArray,
    {
        fn get_ts_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, DataTypeError> {
            let downcast_array = downcast_value!(self, TimestampSecondArray);
            match downcast_array.is_null(index) {
                false => Ok(
                    datum::Timestamp::try_from(SecondUnix(downcast_array.value(index)))
                        .into_datum(),
                ),
                true => Ok(None),
            }
        }
    }

    pub trait GetDatumTimestampFromInt64
    where
        Self: Array + AsArray,
    {
        fn get_ts_from_int64_datum(
            &self,
            index: usize,
        ) -> Result<Option<pg_sys::Datum>, DataTypeError> {
            let downcast_array = downcast_value!(self, Int64Array);
            let timestamp_array = downcast_array.reinterpret_cast::<TimestampMicrosecondType>();

            match timestamp_array.nulls().is_some() && timestamp_array.is_null(index) {
                false => Ok(datum::Timestamp::try_from(MicrosecondUnix(
                    timestamp_array.value(index),
                ))
                .into_datum()),
                true => Ok(None),
            }
        }
    }

    pub trait GetDatumTime
    where
        Self: Array + AsArray,
    {
        fn get_time_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, DataTypeError> {
            let downcast_array = downcast_value!(self, Time64NanosecondArray);
            match downcast_array.is_null(index) {
                false => Ok(
                    datum::Time::try_from(NanosecondDay(downcast_array.value(index))).into_datum(),
                ),
                true => Ok(None),
            }
        }
    }

    pub trait GetDatumNumericFromDecimal
    where
        Self: Array + AsArray,
    {
        fn get_numeric_datum_from_decimal(
            &self,
            index: usize,
            precision: &u8,
            scale: &i8,
        ) -> Result<Option<pg_sys::Datum>, DataTypeError> {
            let downcast_array = downcast_value!(self, Decimal128Array);
            match downcast_array.is_null(index) {
                false => {
                    let value = downcast_array.value(index);
                    Ok(AnyNumeric::try_from(PgNumeric(
                        AnyNumeric::from(value),
                        PgNumericTypeMod(PgPrecision(*precision), PgScale(*scale)),
                    ))
                    .into_datum())
                }
                true => Ok(None),
            }
        }
    }

    pub trait GetDatumNumeric
    where
        Self: Array + AsArray,
    {
        fn get_numeric_datum<A>(
            &self,
            index: usize,
            typemod: i32,
            func: unsafe fn(pg_sys::FunctionCallInfo) -> pg_sys::Datum,
        ) -> Result<Option<pg_sys::Datum>, DatumError>
        where
            A: Array + Debug + 'static,
            for<'a> &'a A: ArrayAccessor + IntoIterator,
            for<'a> <&'a A as ArrayAccessor>::Item: IntoDatum,
        {
            let downcast_array = downcast_value!(self, A);
            match downcast_array.is_null(index) {
                false => {
                    let numeric: Option<AnyNumeric> = unsafe {
                        direct_function_call(
                            func,
                            &[
                                downcast_array.value(index).into_datum(),
                                typemod.into_datum(),
                            ],
                        )
                    };
                    Ok(numeric.into_datum())
                }
                true => Ok(None),
            }
        }
    }

    pub trait GetDatumUuid
    where
        Self: Array + AsArray,
    {
        fn get_uuid_datum(&self, index: usize) -> Result<Option<pg_sys::Datum>, DatumError> {
            let downcast_array = downcast_value!(self, StringArray);
            match downcast_array.is_null(index) {
                false => {
                    let value = downcast_array.value(index);
                    let uuid = uuid::Uuid::parse_str(value)?;
                    Ok(datum::Uuid::from_slice(uuid.as_bytes()).into_datum())
                }
                true => Ok(None),
            }
        }
    }

    pub trait GetDatum
    where
        Self: Array
            + AsArray
            + GetDatumDate
            + GetDatumDateFromInt32
            + GetDatumPrimitive
            + GetDatumPrimitiveList
            + GetDatumNumeric
            + GetDatumNumericFromDecimal
            + GetDatumTimestampMicrosecond
            + GetDatumTimestampMillisecond
            + GetDatumTimestampSecond
            + GetDatumTimestampFromInt64
            + GetDatumTime
            + GetDatumUInt
            + GetDatumUuid,
    {
        fn get_datum(
            &self,
            index: usize,
            oid: PgOid,
            typemod: i32,
        ) -> Result<Option<pg_sys::Datum>, DataTypeError> {
            let result = match oid {
                PgOid::BuiltIn(builtin) => match builtin {
                    BOOLOID => self.get_primitive_datum::<BooleanArray>(index)?,
                    TEXTOID | VARCHAROID | BPCHAROID => {
                        self.get_primitive_datum::<StringArray>(index)?
                    }
                    INT2OID | INT4OID | INT8OID | FLOAT4OID | FLOAT8OID => match self.data_type() {
                        Float32 => self.get_primitive_datum::<Float32Array>(index)?,
                        Float64 => self.get_primitive_datum::<Float64Array>(index)?,
                        Int8 => self.get_primitive_datum::<Int8Array>(index)?,
                        Int16 => self.get_primitive_datum::<Int16Array>(index)?,
                        Int32 => self.get_primitive_datum::<Int32Array>(index)?,
                        Int64 => self.get_primitive_datum::<Int64Array>(index)?,
                        UInt8 => self.get_uint_datum::<UInt8Type>(index)?,
                        UInt16 => self.get_uint_datum::<UInt16Type>(index)?,
                        UInt32 => self.get_uint_datum::<UInt32Type>(index)?,
                        UInt64 => self.get_uint_datum::<UInt64Type>(index)?,
                        unsupported => {
                            return Err(DatumError::IntError(unsupported.clone(), oid).into())
                        }
                    },
                    DATEOID => match self.data_type() {
                        Date32 => self.get_date_datum(index)?,
                        Int32 => self.get_date_from_int32_datum(index)?,
                        unsupported => {
                            return Err(DatumError::DateError(unsupported.clone()).into())
                        }
                    },
                    TIMEOID => self.get_time_datum(index)?,
                    TIMESTAMPOID => match self.data_type() {
                        Timestamp(TimeUnit::Microsecond, None) => self.get_ts_micro_datum(index)?,
                        Timestamp(TimeUnit::Millisecond, None) => self.get_ts_milli_datum(index)?,
                        Timestamp(TimeUnit::Second, None) => self.get_ts_datum(index)?,
                        Int64 => self.get_ts_from_int64_datum(index)?,
                        unsupported => {
                            return Err(DatumError::TimestampError(unsupported.clone()).into())
                        }
                    },
                    NUMERICOID => match self.data_type() {
                        Decimal128(p, s) => self.get_numeric_datum_from_decimal(index, p, s)?,
                        Float32 => self.get_numeric_datum::<Float32Array>(
                            index,
                            typemod,
                            pg_sys::float4_numeric,
                        )?,
                        Float64 => self.get_numeric_datum::<Float64Array>(
                            index,
                            typemod,
                            pg_sys::float8_numeric,
                        )?,
                        Int16 => self.get_numeric_datum::<Int16Array>(
                            index,
                            typemod,
                            pg_sys::int2_numeric,
                        )?,
                        Int32 => self.get_numeric_datum::<Int32Array>(
                            index,
                            typemod,
                            pg_sys::int4_numeric,
                        )?,
                        Int64 => self.get_numeric_datum::<Int64Array>(
                            index,
                            typemod,
                            pg_sys::int8_numeric,
                        )?,
                        unsupported => {
                            return Err(DatumError::NumericError(unsupported.clone()).into())
                        }
                    },
                    UUIDOID => self.get_uuid_datum(index)?,
                    BOOLARRAYOID => self.get_primitive_list_datum::<BooleanArray>(index)?,
                    TEXTARRAYOID | VARCHARARRAYOID | BPCHARARRAYOID => {
                        self.get_primitive_list_datum::<StringArray>(index)?
                    }
                    INT2ARRAYOID | INT4ARRAYOID | INT8ARRAYOID | FLOAT4ARRAYOID
                    | FLOAT8ARRAYOID => match self.data_type() {
                        List(ref field) => match field.data_type().clone() {
                            Float32 => self.get_primitive_list_datum::<Float32Array>(index)?,
                            Float64 => self.get_primitive_list_datum::<Float64Array>(index)?,
                            Int16 => self.get_primitive_list_datum::<Int16Array>(index)?,
                            Int32 => self.get_primitive_list_datum::<Int32Array>(index)?,
                            Int64 => self.get_primitive_list_datum::<Int64Array>(index)?,
                            unsupported => {
                                return Err(
                                    DatumError::IntArrayError(unsupported.clone(), oid).into()
                                )
                            }
                        },
                        unsupported => {
                            return Err(DatumError::IntArrayError(unsupported.clone(), oid).into())
                        }
                    },
                    DATEARRAYOID => self.get_primitive_list_datum::<Date32Array>(index)?,
                    unsupported => return Err(DataTypeError::UnsupportedPostgresType(unsupported)),
                },
                PgOid::Invalid => return Err(DataTypeError::InvalidPostgresOid),
                PgOid::Custom(_) => return Err(DataTypeError::UnsupportedCustomType),
            };

            Ok(result)
        }
    }

    impl GetDatum for ArrayRef {}
    impl GetDatumDate for ArrayRef {}
    impl GetDatumDateFromInt32 for ArrayRef {}
    impl GetDatumPrimitive for ArrayRef {}
    impl GetDatumPrimitiveList for ArrayRef {}
    impl GetDatumNumeric for ArrayRef {}
    impl GetDatumNumericFromDecimal for ArrayRef {}
    impl GetDatumTimestampMicrosecond for ArrayRef {}
    impl GetDatumTimestampMillisecond for ArrayRef {}
    impl GetDatumTimestampSecond for ArrayRef {}
    impl GetDatumTimestampFromInt64 for ArrayRef {}
    impl GetDatumTime for ArrayRef {}
    impl GetDatumUInt for ArrayRef {}
    impl GetDatumUuid for ArrayRef {}

    #[derive(Error, Debug)]
    pub enum DatumError {
        #[error(transparent)]
        DataFusion(#[from] DataFusionError),

        #[error(transparent)]
        UuidError(#[from] uuid::Error),

        #[error("Error converting {0:?} into DATE")]
        DateError(DataType),

        #[error("Error converting {0:?} into {1:?}")]
        IntError(DataType, PgOid),

        #[error("Error converting {0:?} array into {1:?}")]
        IntArrayError(DataType, PgOid),

        #[error("Error converting {0:?} into NUMERIC")]
        NumericError(DataType),

        #[error("Error converting {0:?} into TIMESTAMP")]
        TimestampError(DataType),

        #[error("Failed to convert UInt to i64")]
        UIntConversionError,
    }
}

mod numeric {
    use deltalake::datafusion::arrow::datatypes::{DECIMAL128_MAX_PRECISION, DECIMAL128_MAX_SCALE};
    use pgrx::*;
    use thiserror::Error;

    use super::datatype::PgTypeMod;

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
            let PgNumeric(numeric, PgNumericTypeMod(PgPrecision(precision), PgScale(scale))) =
                numeric;
            scale_anynumeric(numeric, precision, scale, false)
        }
    }

    pub fn scale_anynumeric(
        numeric: AnyNumeric,
        precision: u8,
        scale: i8,
        scale_down: bool,
    ) -> Result<AnyNumeric, NumericError> {
        let original_typemod =
            PgNumericTypeMod(PgPrecision(precision + (scale as u8)), PgScale(scale));
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
}

mod time {
    use chrono::{NaiveTime, TimeDelta, Timelike};
    use deltalake::datafusion::arrow::datatypes::TimeUnit;
    use pgrx::*;
    use thiserror::Error;

    use super::datatype::PgTypeMod;

    const NANOSECONDS_IN_SECOND: i64 = 1_000_000_000;
    const NANOSECONDS_IN_MINUTE: i64 = NANOSECONDS_IN_SECOND * 60;
    const NANOSECONDS_IN_HOUR: i64 = NANOSECONDS_IN_MINUTE * 60;

    #[derive(Copy, Clone, Debug)]
    pub struct NanosecondDay(pub i64);

    #[derive(Clone, Debug)]
    pub struct TimePrecision(pub TimeUnit);

    #[derive(Copy, Clone)]
    pub enum PgTimePrecision {
        Default = -1,
        Microsecond = 6,
    }

    impl PgTimePrecision {
        pub fn value(&self) -> i32 {
            *self as i32
        }
    }

    impl TryFrom<PgTypeMod> for PgTimePrecision {
        type Error = TimeError;

        fn try_from(typemod: PgTypeMod) -> Result<Self, Self::Error> {
            let PgTypeMod(typemod) = typemod;

            match typemod {
                -1 => Ok(PgTimePrecision::Default),
                6 => Ok(PgTimePrecision::Microsecond),
                unsupported => Err(TimeError::UnsupportedTypeMod(unsupported)),
            }
        }
    }

    // Tech Debt: DataFusion defaults time fields with no specified precision to nanosecond,
    // whereas Postgres defaults to microsecond. DataFusion errors when we try to compare
    // Time64(Nanosecond) with Time64(Microsecond), so we just store microsecond precision
    // times as nanosecond as a workaround
    impl TryFrom<PgTypeMod> for TimePrecision {
        type Error = TimeError;

        fn try_from(typemod: PgTypeMod) -> Result<Self, Self::Error> {
            match PgTimePrecision::try_from(typemod)? {
                PgTimePrecision::Default => Ok(TimePrecision(TimeUnit::Nanosecond)),
                PgTimePrecision::Microsecond => Ok(TimePrecision(TimeUnit::Nanosecond)),
            }
        }
    }

    impl TryFrom<TimePrecision> for PgTypeMod {
        type Error = TimeError;

        fn try_from(unit: TimePrecision) -> Result<Self, Self::Error> {
            let TimePrecision(unit) = unit;

            match unit {
                TimeUnit::Nanosecond => Ok(PgTypeMod(PgTimePrecision::Microsecond.value())),
                unsupported => Err(TimeError::UnsupportedTimeUnit(unsupported)),
            }
        }
    }

    impl TryFrom<datum::Time> for NanosecondDay {
        type Error = TimeError;

        fn try_from(time: datum::Time) -> Result<Self, Self::Error> {
            let nanos_elapsed = (time.microseconds() as i64) * 1000
                + (time.minute() as i64) * NANOSECONDS_IN_MINUTE
                + (time.hour() as i64) * NANOSECONDS_IN_HOUR;

            Ok(NanosecondDay(nanos_elapsed))
        }
    }

    impl TryFrom<NanosecondDay> for datum::Time {
        type Error = TimeError;

        fn try_from(nanos: NanosecondDay) -> Result<Self, Self::Error> {
            let NanosecondDay(nanos) = nanos;

            let time_delta = TimeDelta::nanoseconds(nanos);
            let time = NaiveTime::from_hms_nano_opt(0, 0, 0, 0)
                .ok_or(TimeError::MidnightNotFound)?
                + time_delta;
            let total_seconds =
                time.second() as f64 + time.nanosecond() as f64 / NANOSECONDS_IN_SECOND as f64;

            Ok(datum::Time::new(
                time.hour() as u8,
                time.minute() as u8,
                total_seconds,
            )?)
        }
    }

    #[derive(Error, Debug)]
    pub enum TimeError {
        #[error(transparent)]
        DateTimeConversion(#[from] datum::datetime_support::DateTimeConversionError),

        #[error("Could not convert midnight to NaiveTime")]
        MidnightNotFound,

        #[error("Only time and time(6), not time({0}), are supported")]
        UnsupportedTypeMod(i32),

        #[error("Unexpected precision {0:?} for time")]
        UnsupportedTimeUnit(TimeUnit),
    }
}

mod timestamp {
    use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
    use deltalake::datafusion::arrow::datatypes::*;
    use pgrx::*;
    use thiserror::Error;

    use super::datatype::PgTypeMod;

    const MICROSECONDS_IN_SECOND: u32 = 1_000_000;
    const NANOSECONDS_IN_SECOND: u32 = 1_000_000_000;

    #[derive(Copy, Clone, Debug)]
    pub struct MicrosecondUnix(pub i64);

    #[derive(Copy, Clone, Debug)]
    pub struct MillisecondUnix(pub i64);

    #[derive(Copy, Clone, Debug)]
    pub struct SecondUnix(pub i64);

    #[derive(Clone, Debug)]
    pub struct TimestampPrecision(pub TimeUnit);

    #[derive(Copy, Clone)]
    pub enum PgTimestampPrecision {
        Default = -1,
        Second = 0,
        Millisecond = 3,
        Microsecond = 6,
    }

    impl PgTimestampPrecision {
        pub fn value(&self) -> i32 {
            *self as i32
        }
    }

    impl TryFrom<PgTypeMod> for PgTimestampPrecision {
        type Error = TimestampError;

        fn try_from(typemod: PgTypeMod) -> Result<Self, Self::Error> {
            let PgTypeMod(typemod) = typemod;

            match typemod {
                -1 => Ok(PgTimestampPrecision::Default),
                1 => Ok(PgTimestampPrecision::Second),
                3 => Ok(PgTimestampPrecision::Millisecond),
                6 => Ok(PgTimestampPrecision::Microsecond),
                unsupported => Err(TimestampError::UnsupportedTypeMod(unsupported)),
            }
        }
    }

    impl TryFrom<PgTypeMod> for TimestampPrecision {
        type Error = TimestampError;

        fn try_from(typemod: PgTypeMod) -> Result<Self, Self::Error> {
            match PgTimestampPrecision::try_from(typemod)? {
                PgTimestampPrecision::Default => Ok(TimestampPrecision(TimeUnit::Microsecond)),
                PgTimestampPrecision::Second => Ok(TimestampPrecision(TimeUnit::Second)),
                PgTimestampPrecision::Millisecond => Ok(TimestampPrecision(TimeUnit::Millisecond)),
                PgTimestampPrecision::Microsecond => Ok(TimestampPrecision(TimeUnit::Microsecond)),
            }
        }
    }

    impl TryFrom<TimestampPrecision> for PgTypeMod {
        type Error = TimestampError;

        fn try_from(unit: TimestampPrecision) -> Result<Self, Self::Error> {
            let TimestampPrecision(unit) = unit;

            match unit {
                TimeUnit::Second => Ok(PgTypeMod(PgTimestampPrecision::Second.value())),
                TimeUnit::Millisecond => Ok(PgTypeMod(PgTimestampPrecision::Millisecond.value())),
                TimeUnit::Microsecond => Ok(PgTypeMod(PgTimestampPrecision::Microsecond.value())),
                TimeUnit::Nanosecond => Ok(PgTypeMod(PgTimestampPrecision::Microsecond.value())),
            }
        }
    }

    impl TryFrom<datum::Timestamp> for MicrosecondUnix {
        type Error = TimestampError;

        fn try_from(timestamp: datum::Timestamp) -> Result<Self, Self::Error> {
            let date = get_naive_date(&timestamp)?;
            let time = get_naive_time(&timestamp)?;
            let unix = TimestampMicrosecondType::make_value(NaiveDateTime::new(date, time))
                .ok_or(TimestampError::ParseDateTime())?;

            Ok(MicrosecondUnix(unix))
        }
    }

    impl TryFrom<datum::Timestamp> for MillisecondUnix {
        type Error = TimestampError;

        fn try_from(timestamp: datum::Timestamp) -> Result<Self, Self::Error> {
            let date = get_naive_date(&timestamp)?;
            let time = get_naive_time(&timestamp)?;
            let unix = TimestampMillisecondType::make_value(NaiveDateTime::new(date, time))
                .ok_or(TimestampError::ParseDateTime())?;

            Ok(MillisecondUnix(unix))
        }
    }

    impl TryFrom<datum::Timestamp> for SecondUnix {
        type Error = TimestampError;

        fn try_from(timestamp: datum::Timestamp) -> Result<Self, Self::Error> {
            let date = get_naive_date(&timestamp)?;
            let time = get_naive_time(&timestamp)?;
            let unix = TimestampSecondType::make_value(NaiveDateTime::new(date, time))
                .ok_or(TimestampError::ParseDateTime())?;

            Ok(SecondUnix(unix))
        }
    }

    impl TryFrom<MicrosecondUnix> for datum::Timestamp {
        type Error = TimestampError;

        fn try_from(micros: MicrosecondUnix) -> Result<Self, Self::Error> {
            let MicrosecondUnix(unix) = micros;
            let datetime = DateTime::from_timestamp_micros(unix)
                .ok_or(TimestampError::MicrosecondsConversion(unix))?;

            to_timestamp(&datetime)
        }
    }

    impl TryFrom<MillisecondUnix> for datum::Timestamp {
        type Error = TimestampError;

        fn try_from(millis: MillisecondUnix) -> Result<Self, Self::Error> {
            let MillisecondUnix(unix) = millis;
            let datetime = DateTime::from_timestamp_millis(unix)
                .ok_or(TimestampError::MillisecondsConversion(unix))?;

            to_timestamp(&datetime)
        }
    }

    impl TryFrom<SecondUnix> for datum::Timestamp {
        type Error = TimestampError;

        fn try_from(seconds: SecondUnix) -> Result<Self, Self::Error> {
            let SecondUnix(unix) = seconds;
            let datetime =
                DateTime::from_timestamp(unix, 0).ok_or(TimestampError::SecondsConversion(unix))?;

            to_timestamp(&datetime)
        }
    }

    #[inline]
    fn get_naive_date(timestamp: &datum::Timestamp) -> Result<NaiveDate, TimestampError> {
        NaiveDate::from_ymd_opt(
            timestamp.year(),
            timestamp.month().into(),
            timestamp.day().into(),
        )
        .ok_or(TimestampError::ParseDate(timestamp.to_iso_string()))
    }

    #[inline]
    fn get_naive_time(timestamp: &datum::Timestamp) -> Result<NaiveTime, TimestampError> {
        NaiveTime::from_hms_micro_opt(
            timestamp.hour().into(),
            timestamp.minute().into(),
            timestamp.second() as u32,
            timestamp.microseconds() % MICROSECONDS_IN_SECOND,
        )
        .ok_or(TimestampError::ParseTime(timestamp.to_iso_string()))
    }

    #[inline]
    fn to_timestamp<Tz: chrono::TimeZone>(
        datetime: &DateTime<Tz>,
    ) -> Result<datum::Timestamp, TimestampError> {
        Ok(datum::Timestamp::new(
            datetime.year(),
            datetime.month() as u8,
            datetime.day() as u8,
            datetime.hour() as u8,
            datetime.minute() as u8,
            (datetime.second() + datetime.nanosecond() / NANOSECONDS_IN_SECOND).into(),
        )?)
    }

    #[derive(Error, Debug)]
    pub enum TimestampError {
        #[error(transparent)]
        DateTimeConversion(#[from] datum::datetime_support::DateTimeConversionError),

        #[error("Failed to parse time from {0:?}")]
        ParseTime(String),

        #[error("Failed to parse date from {0:?}")]
        ParseDate(String),

        #[error("Failed to make datetime")]
        ParseDateTime(),

        #[error("Failed to convert {0} microseconds to datetime")]
        MicrosecondsConversion(i64),

        #[error("Failed to convert {0} milliseconds to datetime")]
        MillisecondsConversion(i64),

        #[error("Failed to convert {0} seconds to datetime")]
        SecondsConversion(i64),

        #[error("Only timestamp and timestamp(6), not timestamp({0}), are supported")]
        UnsupportedTypeMod(i32),
    }
}
