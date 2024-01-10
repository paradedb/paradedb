use deltalake::datafusion::arrow::datatypes::{
    DataType, Date32Type, Float32Type, Float64Type, Int16Type, Int32Type, Int64Type, Int8Type,
    Time64MicrosecondType, TimeUnit, TimestampMicrosecondType, UInt32Type,
};
use deltalake::datafusion::common::arrow::array::{
    Array, ArrayRef, AsArray, BooleanArray, Date32Array, Float32Array, Float64Array, Int16Array,
    Int32Array, Int64Array, StringArray, Time64MicrosecondArray, TimestampMicrosecondArray,
    UInt32Array,
};
use deltalake::datafusion::common::ScalarValue;
use deltalake::datafusion::logical_expr::Expr;
use pgrx::pg_sys::Datum;
use pgrx::*;
use std::sync::Arc;
use substrait::proto::r#type as substrait_type_mod;
use substrait::proto::Type as SubstraitType;

#[allow(clippy::type_complexity)]
pub struct DatafusionMap {
    pub literal: Box<dyn Fn(*mut pg_sys::Datum, bool) -> Expr>,
    pub array: Box<dyn Fn(*mut *mut pg_sys::TupleTableSlot, usize, usize) -> ArrayRef>,
    pub index_datum: Box<dyn Fn(&Arc<dyn Array>, usize) -> Result<Datum, String>>,
}

const SUBSTRAIT_USER_DEFINED_U32: u32 = 1;

fn substrait_user_defined_type_from_reference(type_reference: u32) -> substrait_type_mod::Kind {
    let mut result = substrait_type_mod::UserDefined::default();

    match type_reference {
        SUBSTRAIT_USER_DEFINED_U32 => {
            result.type_reference = SUBSTRAIT_USER_DEFINED_U32;
            result.type_parameters = vec![substrait_type_mod::Parameter {
                parameter: Some(substrait_type_mod::parameter::Parameter::DataType(
                    SubstraitType {
                        kind: Some(substrait_type_mod::Kind::FixedChar(
                            substrait_type_mod::FixedChar {
                                length: 4,
                                type_variation_reference: 0,
                                nullability: substrait_type_mod::Nullability::Required.into(),
                            },
                        )),
                    },
                )),
            }];
        }
        _ => todo!(),
    };

    substrait_type_mod::Kind::UserDefined(result)
}

pub trait SubstraitTranslator {
    fn to_substrait(&self) -> Result<SubstraitType, String>;
    fn from_substrait(substrait_type: SubstraitType) -> Result<Self, String>
    where
        Self: Sized;
}

impl SubstraitTranslator for DataType {
    fn to_substrait(&self) -> Result<SubstraitType, String> {
        let result = SubstraitType {
            kind: match self {
                DataType::Boolean => Some(substrait_type_mod::Kind::Bool(
                    substrait_type_mod::Boolean::default(),
                )),
                DataType::Utf8 => Some(substrait_type_mod::Kind::String(
                    substrait_type_mod::String::default(),
                )),
                DataType::Int16 => Some(substrait_type_mod::Kind::I16(
                    substrait_type_mod::I16::default(),
                )),
                DataType::Int32 => Some(substrait_type_mod::Kind::I32(
                    substrait_type_mod::I32::default(),
                )),
                DataType::Int64 => Some(substrait_type_mod::Kind::I64(
                    substrait_type_mod::I64::default(),
                )),
                DataType::UInt32 => Some(substrait_user_defined_type_from_reference(
                    SUBSTRAIT_USER_DEFINED_U32,
                )),
                DataType::Float32 => Some(substrait_type_mod::Kind::Fp32(
                    substrait_type_mod::Fp32::default(),
                )),
                DataType::Float64 => Some(substrait_type_mod::Kind::Fp64(
                    substrait_type_mod::Fp64::default(),
                )),
                DataType::Time64(TimeUnit::Microsecond) => Some(substrait_type_mod::Kind::Time(
                    substrait_type_mod::Time::default(),
                )),
                DataType::Timestamp(TimeUnit::Microsecond, None) => Some(
                    substrait_type_mod::Kind::Timestamp(substrait_type_mod::Timestamp::default()),
                ),
                DataType::Date32 => Some(substrait_type_mod::Kind::Date(
                    substrait_type_mod::Date::default(),
                )),
                _ => todo!(),
            },
        };

        Ok(result)
    }

    fn from_substrait(substrait_type: SubstraitType) -> Result<DataType, String> {
        let result = match substrait_type.kind {
            Some(kind) => match kind {
                substrait_type_mod::Kind::Bool(_) => DataType::Boolean,
                substrait_type_mod::Kind::String(_) => DataType::Utf8,
                substrait_type_mod::Kind::I16(_) => DataType::Int16,
                substrait_type_mod::Kind::I32(_) => DataType::Int32,
                substrait_type_mod::Kind::I64(_) => DataType::Int64,
                substrait_type_mod::Kind::Fp32(_) => DataType::Float32,
                substrait_type_mod::Kind::Fp64(_) => DataType::Float64,
                substrait_type_mod::Kind::Time(_) => DataType::Time64(TimeUnit::Microsecond),
                substrait_type_mod::Kind::Timestamp(_) => {
                    DataType::Timestamp(TimeUnit::Microsecond, None)
                }
                substrait_type_mod::Kind::Date(_) => DataType::Date32,
                substrait_type_mod::Kind::UserDefined(user_defined) => {
                    match user_defined.type_reference {
                        SUBSTRAIT_USER_DEFINED_U32 => DataType::UInt32,
                        _ => todo!(),
                    }
                }
                _ => todo!(),
            },
            None => todo!(),
        };

        Ok(result)
    }
}

impl SubstraitTranslator for PgOid {
    fn to_substrait(&self) -> Result<SubstraitType, String> {
        let result = SubstraitType {
            kind: match self {
                PgOid::BuiltIn(builtin) => match builtin {
                    PgBuiltInOids::BOOLOID => Some(substrait_type_mod::Kind::Bool(
                        substrait_type_mod::Boolean::default(),
                    )),
                    PgBuiltInOids::BPCHAROID
                    | PgBuiltInOids::TEXTOID
                    | PgBuiltInOids::VARCHAROID => Some(substrait_type_mod::Kind::String(
                        substrait_type_mod::String::default(),
                    )),
                    PgBuiltInOids::INT2OID => Some(substrait_type_mod::Kind::I16(
                        substrait_type_mod::I16::default(),
                    )),
                    PgBuiltInOids::INT4OID => Some(substrait_type_mod::Kind::I32(
                        substrait_type_mod::I32::default(),
                    )),
                    PgBuiltInOids::INT8OID => Some(substrait_type_mod::Kind::I64(
                        substrait_type_mod::I64::default(),
                    )),
                    PgBuiltInOids::OIDOID | PgBuiltInOids::XIDOID => Some(
                        substrait_user_defined_type_from_reference(SUBSTRAIT_USER_DEFINED_U32),
                    ),
                    PgBuiltInOids::FLOAT4OID => Some(substrait_type_mod::Kind::Fp32(
                        substrait_type_mod::Fp32::default(),
                    )),
                    PgBuiltInOids::FLOAT8OID => Some(substrait_type_mod::Kind::Fp64(
                        substrait_type_mod::Fp64::default(),
                    )),
                    PgBuiltInOids::TIMEOID => Some(substrait_type_mod::Kind::Time(
                        substrait_type_mod::Time::default(),
                    )),
                    PgBuiltInOids::TIMESTAMPOID => Some(substrait_type_mod::Kind::Timestamp(
                        substrait_type_mod::Timestamp::default(),
                    )),
                    PgBuiltInOids::DATEOID => Some(substrait_type_mod::Kind::Date(
                        substrait_type_mod::Date::default(),
                    )),
                    _ => todo!(),
                },
                _ => todo!(),
            },
        };

        Ok(result)
    }

    fn from_substrait(substrait_type: SubstraitType) -> Result<PgOid, String> {
        let result = match substrait_type.kind {
            Some(kind) => match kind {
                substrait_type_mod::Kind::Bool(_) => PgBuiltInOids::BOOLOID,
                substrait_type_mod::Kind::String(_) => PgBuiltInOids::TEXTOID,
                substrait_type_mod::Kind::I16(_) => PgBuiltInOids::INT2OID,
                substrait_type_mod::Kind::I32(_) => PgBuiltInOids::INT4OID,
                substrait_type_mod::Kind::I64(_) => PgBuiltInOids::INT8OID,
                substrait_type_mod::Kind::Fp32(_) => PgBuiltInOids::FLOAT4OID,
                substrait_type_mod::Kind::Fp64(_) => PgBuiltInOids::FLOAT8OID,
                substrait_type_mod::Kind::Time(_) => PgBuiltInOids::TIMEOID,
                substrait_type_mod::Kind::Timestamp(_) => PgBuiltInOids::TIMESTAMPOID,
                substrait_type_mod::Kind::Date(_) => PgBuiltInOids::DATEOID,
                substrait_type_mod::Kind::UserDefined(user_defined) => {
                    match user_defined.type_reference {
                        SUBSTRAIT_USER_DEFINED_U32 => PgBuiltInOids::OIDOID,
                        _ => todo!(),
                    }
                }
                _ => todo!(),
            },
            None => todo!(),
        };

        Ok(pgrx::PgOid::BuiltIn(result))
    }
}

pub struct DatafusionMapProducer;
impl DatafusionMapProducer {
    pub fn map<F, R>(substrait_type: SubstraitType, mut f: F) -> Result<R, String>
    where
        F: FnMut(DatafusionMap) -> R,
    {
        let datafusion_type = DataType::from_substrait(substrait_type)?;

        let result = match datafusion_type {
            DataType::Boolean => f(DatafusionMap {
                literal: Box::new(|datum: *mut pg_sys::Datum, is_null: bool| -> Expr {
                    if is_null {
                        Expr::Literal(ScalarValue::Boolean(None))
                    } else {
                        unsafe {
                            Expr::Literal(ScalarValue::Boolean(bool::from_datum(*datum, false)))
                        }
                    }
                }),
                array: Box::new(
                    |slots: *mut *mut pg_sys::TupleTableSlot,
                     nslots: usize,
                     col_idx: usize|
                     -> ArrayRef {
                        let mut vec = Vec::with_capacity(nslots);

                        for row_idx in 0..nslots {
                            let tuple_table_slot = unsafe { *slots.add(row_idx) };
                            let datum = unsafe { (*tuple_table_slot).tts_values.add(col_idx) };
                            let is_null = unsafe { *(*tuple_table_slot).tts_isnull.add(col_idx) };

                            if is_null {
                                vec.push(None);
                            } else {
                                vec.push(unsafe { bool::from_datum(*datum, false) });
                            }
                        }

                        Arc::new(BooleanArray::from(vec))
                    },
                ),
                index_datum: Box::new(
                    |array: &Arc<dyn Array>, index: usize| -> Result<Datum, String> {
                        Ok(array
                            .as_primitive::<Int8Type>()
                            .value(index)
                            .into_datum()
                            .ok_or("Could not convert Boolean into datum")?)
                    },
                ),
            }),
            DataType::Utf8 => f(DatafusionMap {
                literal: Box::new(|datum: *mut pg_sys::Datum, is_null: bool| -> Expr {
                    if is_null {
                        Expr::Literal(ScalarValue::Utf8(None))
                    } else {
                        unsafe {
                            Expr::Literal(ScalarValue::Utf8(String::from_datum(*datum, false)))
                        }
                    }
                }),
                array: Box::new(
                    |slots: *mut *mut pg_sys::TupleTableSlot,
                     nslots: usize,
                     col_idx: usize|
                     -> ArrayRef {
                        let mut vec = Vec::with_capacity(nslots);

                        for row_idx in 0..nslots {
                            let tuple_table_slot = unsafe { *slots.add(row_idx) };
                            let datum = unsafe { (*tuple_table_slot).tts_values.add(col_idx) };
                            let is_null = unsafe { *(*tuple_table_slot).tts_isnull.add(col_idx) };

                            if is_null {
                                vec.push(None);
                            } else {
                                vec.push(unsafe { String::from_datum(*datum, false) });
                            }
                        }

                        Arc::new(StringArray::from(vec))
                    },
                ),
                index_datum: Box::new(
                    |array: &Arc<dyn Array>, index: usize| -> Result<Datum, String> {
                        Ok(array
                            .as_any()
                            .downcast_ref::<StringArray>()
                            .ok_or("Could not downcast Utf8 into string array")?
                            .value(index)
                            .into_datum()
                            .ok_or("Could not convert Utf8 into datum")?)
                    },
                ),
            }),
            DataType::Int16 => f(DatafusionMap {
                literal: Box::new(|datum: *mut pg_sys::Datum, is_null: bool| -> Expr {
                    if is_null {
                        Expr::Literal(ScalarValue::Int16(None))
                    } else {
                        unsafe { Expr::Literal(ScalarValue::Int16(i16::from_datum(*datum, false))) }
                    }
                }),
                array: Box::new(
                    |slots: *mut *mut pg_sys::TupleTableSlot,
                     nslots: usize,
                     col_idx: usize|
                     -> ArrayRef {
                        let mut vec = Vec::with_capacity(nslots);

                        for row_idx in 0..nslots {
                            let tuple_table_slot = unsafe { *slots.add(row_idx) };
                            let datum = unsafe { (*tuple_table_slot).tts_values.add(col_idx) };
                            let is_null = unsafe { *(*tuple_table_slot).tts_isnull.add(col_idx) };

                            if is_null {
                                vec.push(None);
                            } else {
                                vec.push(unsafe { i16::from_datum(*datum, false) });
                            }
                        }

                        Arc::new(Int16Array::from(vec))
                    },
                ),
                index_datum: Box::new(
                    |array: &Arc<dyn Array>, index: usize| -> Result<Datum, String> {
                        Ok(array
                            .as_primitive::<Int16Type>()
                            .value(index)
                            .into_datum()
                            .ok_or("Could not convert Int16 into datum")?)
                    },
                ),
            }),
            DataType::Int32 => f(DatafusionMap {
                literal: Box::new(|datum: *mut pg_sys::Datum, is_null: bool| -> Expr {
                    if is_null {
                        Expr::Literal(ScalarValue::Int32(None))
                    } else {
                        unsafe { Expr::Literal(ScalarValue::Int32(i32::from_datum(*datum, false))) }
                    }
                }),
                array: Box::new(
                    |slots: *mut *mut pg_sys::TupleTableSlot,
                     nslots: usize,
                     col_idx: usize|
                     -> ArrayRef {
                        let mut vec = Vec::with_capacity(nslots);

                        for row_idx in 0..nslots {
                            let tuple_table_slot = unsafe { *slots.add(row_idx) };
                            let datum = unsafe { (*tuple_table_slot).tts_values.add(col_idx) };
                            let is_null = unsafe { *(*tuple_table_slot).tts_isnull.add(col_idx) };

                            if is_null {
                                vec.push(None);
                            } else {
                                vec.push(unsafe { i32::from_datum(*datum, false) });
                            }
                        }

                        Arc::new(Int32Array::from(vec))
                    },
                ),
                index_datum: Box::new(
                    |array: &Arc<dyn Array>, index: usize| -> Result<Datum, String> {
                        Ok(array
                            .as_primitive::<Int32Type>()
                            .value(index)
                            .into_datum()
                            .ok_or("Could not convert Int32 into datum")?)
                    },
                ),
            }),
            DataType::Int64 => f(DatafusionMap {
                literal: Box::new(|datum: *mut pg_sys::Datum, is_null: bool| -> Expr {
                    if is_null {
                        Expr::Literal(ScalarValue::Int64(None))
                    } else {
                        unsafe { Expr::Literal(ScalarValue::Int64(i64::from_datum(*datum, false))) }
                    }
                }),
                array: Box::new(
                    |slots: *mut *mut pg_sys::TupleTableSlot,
                     nslots: usize,
                     col_idx: usize|
                     -> ArrayRef {
                        let mut vec = Vec::with_capacity(nslots);

                        for row_idx in 0..nslots {
                            let tuple_table_slot = unsafe { *slots.add(row_idx) };
                            let datum = unsafe { (*tuple_table_slot).tts_values.add(col_idx) };
                            let is_null = unsafe { *(*tuple_table_slot).tts_isnull.add(col_idx) };

                            if is_null {
                                vec.push(None);
                            } else {
                                vec.push(unsafe { i64::from_datum(*datum, false) });
                            }
                        }

                        Arc::new(Int64Array::from(vec))
                    },
                ),
                index_datum: Box::new(
                    |array: &Arc<dyn Array>, index: usize| -> Result<Datum, String> {
                        Ok(array
                            .as_primitive::<Int64Type>()
                            .value(index)
                            .into_datum()
                            .ok_or("Could not convert Int64 into datum")?)
                    },
                ),
            }),
            DataType::UInt32 => f(DatafusionMap {
                literal: Box::new(|datum: *mut pg_sys::Datum, is_null: bool| -> Expr {
                    if is_null {
                        Expr::Literal(ScalarValue::UInt32(None))
                    } else {
                        unsafe {
                            Expr::Literal(ScalarValue::UInt32(u32::from_datum(*datum, false)))
                        }
                    }
                }),
                array: Box::new(
                    |slots: *mut *mut pg_sys::TupleTableSlot,
                     nslots: usize,
                     col_idx: usize|
                     -> ArrayRef {
                        let mut vec = Vec::with_capacity(nslots);

                        for row_idx in 0..nslots {
                            let tuple_table_slot = unsafe { *slots.add(row_idx) };
                            let datum = unsafe { (*tuple_table_slot).tts_values.add(col_idx) };
                            let is_null = unsafe { *(*tuple_table_slot).tts_isnull.add(col_idx) };

                            if is_null {
                                vec.push(None);
                            } else {
                                vec.push(unsafe { u32::from_datum(*datum, false) });
                            }
                        }

                        Arc::new(UInt32Array::from(vec))
                    },
                ),
                index_datum: Box::new(
                    |array: &Arc<dyn Array>, index: usize| -> Result<Datum, String> {
                        Ok(array
                            .as_primitive::<UInt32Type>()
                            .value(index)
                            .into_datum()
                            .ok_or("Could not convert UInt32 into datum")?)
                    },
                ),
            }),
            DataType::Float32 => f(DatafusionMap {
                literal: Box::new(|datum: *mut pg_sys::Datum, is_null: bool| -> Expr {
                    if is_null {
                        Expr::Literal(ScalarValue::Float32(None))
                    } else {
                        unsafe {
                            Expr::Literal(ScalarValue::Float32(f32::from_datum(*datum, false)))
                        }
                    }
                }),
                array: Box::new(
                    |slots: *mut *mut pg_sys::TupleTableSlot,
                     nslots: usize,
                     col_idx: usize|
                     -> ArrayRef {
                        let mut vec = Vec::with_capacity(nslots);

                        for row_idx in 0..nslots {
                            let tuple_table_slot = unsafe { *slots.add(row_idx) };
                            let datum = unsafe { (*tuple_table_slot).tts_values.add(col_idx) };
                            let is_null = unsafe { *(*tuple_table_slot).tts_isnull.add(col_idx) };

                            if is_null {
                                vec.push(None);
                            } else {
                                vec.push(unsafe { f32::from_datum(*datum, false) });
                            }
                        }

                        Arc::new(Float32Array::from(vec))
                    },
                ),
                index_datum: Box::new(
                    |array: &Arc<dyn Array>, index: usize| -> Result<Datum, String> {
                        Ok(array
                            .as_primitive::<Float32Type>()
                            .value(index)
                            .into_datum()
                            .ok_or("Could not convert Float32 into datum")?)
                    },
                ),
            }),
            DataType::Float64 => f(DatafusionMap {
                literal: Box::new(|datum: *mut pg_sys::Datum, is_null: bool| -> Expr {
                    if is_null {
                        Expr::Literal(ScalarValue::Float64(None))
                    } else {
                        unsafe {
                            Expr::Literal(ScalarValue::Float64(f64::from_datum(*datum, false)))
                        }
                    }
                }),
                array: Box::new(
                    |slots: *mut *mut pg_sys::TupleTableSlot,
                     nslots: usize,
                     col_idx: usize|
                     -> ArrayRef {
                        let mut vec = Vec::with_capacity(nslots);

                        for row_idx in 0..nslots {
                            let tuple_table_slot = unsafe { *slots.add(row_idx) };
                            let datum = unsafe { (*tuple_table_slot).tts_values.add(col_idx) };
                            let is_null = unsafe { *(*tuple_table_slot).tts_isnull.add(col_idx) };

                            if is_null {
                                vec.push(None);
                            } else {
                                vec.push(unsafe { f64::from_datum(*datum, false) });
                            }
                        }

                        Arc::new(Float64Array::from(vec))
                    },
                ),
                index_datum: Box::new(
                    |array: &Arc<dyn Array>, index: usize| -> Result<Datum, String> {
                        Ok(array
                            .as_primitive::<Float64Type>()
                            .value(index)
                            .into_datum()
                            .ok_or("Could not convert Float64 into datum")?)
                    },
                ),
            }),
            DataType::Time64(TimeUnit::Microsecond) => f(DatafusionMap {
                literal: Box::new(|datum: *mut pg_sys::Datum, is_null: bool| -> Expr {
                    if is_null {
                        Expr::Literal(ScalarValue::Time64Microsecond(None))
                    } else {
                        unsafe {
                            Expr::Literal(ScalarValue::Time64Microsecond(i64::from_datum(
                                *datum, false,
                            )))
                        }
                    }
                }),
                array: Box::new(
                    |slots: *mut *mut pg_sys::TupleTableSlot,
                     nslots: usize,
                     col_idx: usize|
                     -> ArrayRef {
                        let mut vec = Vec::with_capacity(nslots);

                        for row_idx in 0..nslots {
                            let tuple_table_slot = unsafe { *slots.add(row_idx) };
                            let datum = unsafe { (*tuple_table_slot).tts_values.add(col_idx) };
                            let is_null = unsafe { *(*tuple_table_slot).tts_isnull.add(col_idx) };

                            if is_null {
                                vec.push(None);
                            } else {
                                vec.push(unsafe { i64::from_datum(*datum, false) });
                            }
                        }

                        Arc::new(Time64MicrosecondArray::from(vec))
                    },
                ),
                index_datum: Box::new(
                    |array: &Arc<dyn Array>, index: usize| -> Result<Datum, String> {
                        Ok(array
                            .as_primitive::<Time64MicrosecondType>()
                            .value(index)
                            .into_datum()
                            .ok_or("Could not convert Time64 into datum")?)
                    },
                ),
            }),
            DataType::Timestamp(TimeUnit::Microsecond, None) => f(DatafusionMap {
                literal: Box::new(|datum: *mut pg_sys::Datum, is_null: bool| -> Expr {
                    if is_null {
                        Expr::Literal(ScalarValue::TimestampMicrosecond(None, None))
                    } else {
                        unsafe {
                            Expr::Literal(ScalarValue::TimestampMicrosecond(
                                i64::from_datum(*datum, false),
                                None,
                            ))
                        }
                    }
                }),
                array: Box::new(
                    |slots: *mut *mut pg_sys::TupleTableSlot,
                     nslots: usize,
                     col_idx: usize|
                     -> ArrayRef {
                        let mut vec = Vec::with_capacity(nslots);

                        for row_idx in 0..nslots {
                            let tuple_table_slot = unsafe { *slots.add(row_idx) };
                            let datum = unsafe { (*tuple_table_slot).tts_values.add(col_idx) };
                            let is_null = unsafe { *(*tuple_table_slot).tts_isnull.add(col_idx) };

                            if is_null {
                                vec.push(None);
                            } else {
                                vec.push(unsafe { i64::from_datum(*datum, false) });
                            }
                        }

                        Arc::new(TimestampMicrosecondArray::from(vec))
                    },
                ),
                index_datum: Box::new(
                    |array: &Arc<dyn Array>, index: usize| -> Result<Datum, String> {
                        Ok(array
                            .as_primitive::<TimestampMicrosecondType>()
                            .value(index)
                            .into_datum()
                            .ok_or("Could not convert Timestamp into datum")?)
                    },
                ),
            }),
            DataType::Date32 => f(DatafusionMap {
                literal: Box::new(|datum: *mut pg_sys::Datum, is_null: bool| -> Expr {
                    if is_null {
                        Expr::Literal(ScalarValue::Date32(None))
                    } else {
                        unsafe {
                            Expr::Literal(ScalarValue::Date32(i32::from_datum(*datum, false)))
                        }
                    }
                }),
                array: Box::new(
                    |slots: *mut *mut pg_sys::TupleTableSlot,
                     nslots: usize,
                     col_idx: usize|
                     -> ArrayRef {
                        let mut vec = Vec::with_capacity(nslots);

                        for row_idx in 0..nslots {
                            let tuple_table_slot = unsafe { *slots.add(row_idx) };
                            let datum = unsafe { (*tuple_table_slot).tts_values.add(col_idx) };
                            let is_null = unsafe { *(*tuple_table_slot).tts_isnull.add(col_idx) };

                            if is_null {
                                vec.push(None);
                            } else {
                                vec.push(unsafe { i32::from_datum(*datum, false) });
                            }
                        }

                        Arc::new(Date32Array::from(vec))
                    },
                ),
                index_datum: Box::new(
                    |array: &Arc<dyn Array>, index: usize| -> Result<Datum, String> {
                        Ok(array
                            .as_primitive::<Date32Type>()
                            .value(index)
                            .into_datum()
                            .ok_or("Could not convert Date32 into datum")?)
                    },
                ),
            }),
            _ => todo!(),
        };

        Ok(result)
    }
}
