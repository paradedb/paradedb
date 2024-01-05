use datafusion::arrow::datatypes::DataType;
use datafusion::common::arrow::array::{
    ArrayRef, BooleanArray, Date32Array, Float32Array, Float64Array, Int16Array, Int32Array,
    Int64Array, StringArray, Time64MicrosecondArray, TimestampMicrosecondArray, UInt32Array,
};
use datafusion::common::ScalarValue;
use datafusion::logical_expr::Expr;
use substrait::proto::r#type as substrait_type_mod;
use substrait::proto::Type as SubstraitType;
use pgrx::*;
use std::sync::Arc;

pub struct DatafusionMap {
    pub literal: Box<dyn Fn(*mut pg_sys::Datum, bool) -> Expr>,
    pub array: Box<dyn Fn(Vec<*mut pg_sys::Datum>, Vec<bool>) -> ArrayRef>,
}

const SUBSTRAIT_USER_DEFINED_U32: u32 = 1;

fn substrait_user_defined_type_from_reference(
    type_reference: u32, 
) -> substrait_type_mod::Kind {
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
                                nullability: substrait_type_mod::Nullability::Required.into()
                            }
                        ))
                    }
                ))
            }];
        },
        _ => todo!()
    };

    substrait_type_mod::Kind::UserDefined(result)
}

pub trait SubstraitTranslator {
    fn to_substrait(&self) -> Result<SubstraitType, String>;
    fn from_substrait(substrait_type: SubstraitType) -> Result<Self, String> where Self: Sized;
}

impl SubstraitTranslator for datafusion::arrow::datatypes::DataType {
    fn to_substrait(&self) -> Result<SubstraitType, String> {
        let result = SubstraitType { kind: match self {
            DataType::Boolean => Some(
                substrait_type_mod::Kind::Bool(
                    substrait_type_mod::Boolean::default()
                )
            ),
            DataType::Utf8 => Some(
                substrait_type_mod::Kind::String(
                    substrait_type_mod::String::default()
                )
            ),
            DataType::Int16 => Some(
                substrait_type_mod::Kind::I16(
                    substrait_type_mod::I16::default()
                )
            ),
            DataType::Int32 => Some(
                substrait_type_mod::Kind::I32(
                    substrait_type_mod::I32::default()
                )
            ),
            DataType::Int64 => Some(
                substrait_type_mod::Kind::I64(
                    substrait_type_mod::I64::default()
                )
            ),
            DataType::UInt32 => Some(
                substrait_user_defined_type_from_reference(
                    SUBSTRAIT_USER_DEFINED_U32,
                )
            ),
            DataType::Float32 => Some(
                substrait_type_mod::Kind::Fp32(
                    substrait_type_mod::Fp32::default()
                )
            ),
            DataType::Float64 => Some(
                substrait_type_mod::Kind::Fp64(
                    substrait_type_mod::Fp64::default()
                )
            ),
            DataType::Time64(datafusion::arrow::datatypes::TimeUnit::Microsecond) => Some(
                substrait_type_mod::Kind::Time(
                    substrait_type_mod::Time::default()
                )
            ),
            DataType::Timestamp(datafusion::arrow::datatypes::TimeUnit::Microsecond, None) => Some(
                substrait_type_mod::Kind::Timestamp(
                    substrait_type_mod::Timestamp::default()
                )
            ),
            DataType::Date32 => Some(
                substrait_type_mod::Kind::Date(
                    substrait_type_mod::Date::default()
                )
            ),
            _ => todo!()
        }};

        Ok(result)
    }

    fn from_substrait(substrait_type: SubstraitType) -> Result<datafusion::arrow::datatypes::DataType, String> {
        let result = match substrait_type.kind {
            Some(kind) => match kind {
                substrait_type_mod::Kind::Bool(_) => DataType::Boolean,
                substrait_type_mod::Kind::String(_) => DataType::Utf8,
                substrait_type_mod::Kind::I16(_) => DataType::Int16,
                substrait_type_mod::Kind::I32(_) => DataType::Int32,
                substrait_type_mod::Kind::I64(_) => DataType::Int64,
                substrait_type_mod::Kind::Fp32(_) => DataType::Float32,
                substrait_type_mod::Kind::Fp64(_) => DataType::Float64,
                substrait_type_mod::Kind::Time(_) => DataType::Time64(datafusion::arrow::datatypes::TimeUnit::Microsecond),
                substrait_type_mod::Kind::Timestamp(_) => DataType::Timestamp(datafusion::arrow::datatypes::TimeUnit::Microsecond, None),
                substrait_type_mod::Kind::Date(_) => DataType::Date32,
                substrait_type_mod::Kind::UserDefined(user_defined) => {
                    match user_defined.type_reference {
                        SUBSTRAIT_USER_DEFINED_U32 => DataType::UInt32,
                        _ => todo!()
                    }
                },
                _ => todo!()
            },
            None => todo!()
        };

        Ok(result)
    }
}

impl SubstraitTranslator for PgOid {
    fn to_substrait(&self) -> Result<SubstraitType, String> {
        let result = SubstraitType { kind: match self {
            PgOid::BuiltIn(builtin) => match builtin {
                PgBuiltInOids::BOOLOID => Some(
                    substrait_type_mod::Kind::Bool(
                        substrait_type_mod::Boolean::default()
                    )
                ),
                PgBuiltInOids::BPCHAROID | PgBuiltInOids::TEXTOID | PgBuiltInOids::VARCHAROID => Some(
                    substrait_type_mod::Kind::String(
                        substrait_type_mod::String::default()
                    )
                ),
                PgBuiltInOids::INT2OID => Some(
                    substrait_type_mod::Kind::I16(
                        substrait_type_mod::I16::default()
                    )
                ),
                PgBuiltInOids::INT4OID => Some(
                    substrait_type_mod::Kind::I32(
                        substrait_type_mod::I32::default()
                    )
                ),
                PgBuiltInOids::INT8OID => Some(
                    substrait_type_mod::Kind::I64(
                        substrait_type_mod::I64::default()
                    )
                ),
                PgBuiltInOids::OIDOID | PgBuiltInOids::XIDOID => Some(
                    substrait_user_defined_type_from_reference(
                        SUBSTRAIT_USER_DEFINED_U32,
                    )
                ),
                PgBuiltInOids::FLOAT4OID => Some(
                    substrait_type_mod::Kind::Fp32(
                        substrait_type_mod::Fp32::default()
                    )
                ),
                PgBuiltInOids::FLOAT8OID => Some(
                    substrait_type_mod::Kind::Fp64(
                        substrait_type_mod::Fp64::default()
                    )
                ),
                PgBuiltInOids::TIMEOID => Some(
                    substrait_type_mod::Kind::Time(
                        substrait_type_mod::Time::default()
                    )
                ),
                PgBuiltInOids::TIMESTAMPOID => Some(
                    substrait_type_mod::Kind::Timestamp(
                        substrait_type_mod::Timestamp::default()
                    )
                ),
                PgBuiltInOids::DATEOID => Some(
                    substrait_type_mod::Kind::Date(
                        substrait_type_mod::Date::default()
                    )
                ),
                _ => todo!()
            },
            _ => todo!()
        }};

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
                        _ => todo!()
                    }
                },
                _ => todo!()
            },
            None => todo!()
        };

        Ok(pgrx::PgOid::BuiltIn(result))
    }
}

pub struct DatafusionProducer;
impl DatafusionProducer {
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
                    |datums: Vec<*mut pg_sys::Datum>, is_nulls: Vec<bool>| -> ArrayRef {
                        let vec: Vec<Option<bool>> = (0..datums.len())
                            .map(|idx| {
                                if is_nulls[idx] {
                                    None
                                } else {
                                    unsafe { bool::from_datum(*datums[idx], false) }
                                }
                            })
                            .collect();

                        Arc::new(BooleanArray::from(vec))
                    },
                ),
            }),
            DataType::Utf8 => {
                f(DatafusionMap {
                    literal: Box::new(|datum: *mut pg_sys::Datum, is_null: bool| -> Expr {
                        if is_null {
                            Expr::Literal(ScalarValue::Utf8(None))
                        } else {
                            unsafe {
                                Expr::Literal(ScalarValue::Utf8(String::from_datum(
                                    *datum, false,
                                )))
                            }
                        }
                    }),
                    array: Box::new(
                        |datums: Vec<*mut pg_sys::Datum>, is_nulls: Vec<bool>| -> ArrayRef {
                            let vec: Vec<Option<String>> = (0..datums.len())
                                .map(|idx| {
                                    if is_nulls[idx] {
                                        None
                                    } else {
                                        unsafe { String::from_datum(*datums[idx], false) }
                                    }
                                })
                                .collect();

                            Arc::new(StringArray::from(vec))
                        },
                    ),
                })
            }
            DataType::Int16 => f(DatafusionMap {
                literal: Box::new(|datum: *mut pg_sys::Datum, is_null: bool| -> Expr {
                    if is_null {
                        Expr::Literal(ScalarValue::Int16(None))
                    } else {
                        unsafe {
                            Expr::Literal(ScalarValue::Int16(i16::from_datum(*datum, false)))
                        }
                    }
                }),
                array: Box::new(
                    |datums: Vec<*mut pg_sys::Datum>, is_nulls: Vec<bool>| -> ArrayRef {
                        let vec: Vec<Option<i16>> = (0..datums.len())
                            .map(|idx| {
                                if is_nulls[idx] {
                                    None
                                } else {
                                    unsafe { i16::from_datum(*datums[idx], false) }
                                }
                            })
                            .collect();

                        Arc::new(Int16Array::from(vec))
                    },
                ),
            }),
            DataType::Int32 => f(DatafusionMap {
                literal: Box::new(|datum: *mut pg_sys::Datum, is_null: bool| -> Expr {
                    if is_null {
                        Expr::Literal(ScalarValue::Int32(None))
                    } else {
                        unsafe {
                            Expr::Literal(ScalarValue::Int32(i32::from_datum(*datum, false)))
                        }
                    }
                }),
                array: Box::new(
                    |datums: Vec<*mut pg_sys::Datum>, is_nulls: Vec<bool>| -> ArrayRef {
                        let vec: Vec<Option<i32>> = (0..datums.len())
                            .map(|idx| {
                                if is_nulls[idx] {
                                    None
                                } else {
                                    unsafe { i32::from_datum(*datums[idx], false) }
                                }
                            })
                            .collect();

                        Arc::new(Int32Array::from(vec))
                    },
                ),
            }),
            DataType::Int64 => f(DatafusionMap {
                literal: Box::new(|datum: *mut pg_sys::Datum, is_null: bool| -> Expr {
                    if is_null {
                        Expr::Literal(ScalarValue::Int64(None))
                    } else {
                        unsafe {
                            Expr::Literal(ScalarValue::Int64(i64::from_datum(*datum, false)))
                        }
                    }
                }),
                array: Box::new(
                    |datums: Vec<*mut pg_sys::Datum>, is_nulls: Vec<bool>| -> ArrayRef {
                        let vec: Vec<Option<i64>> = (0..datums.len())
                            .map(|idx| {
                                if is_nulls[idx] {
                                    None
                                } else {
                                    unsafe { i64::from_datum(*datums[idx], false) }
                                }
                            })
                            .collect();

                        Arc::new(Int64Array::from(vec))
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
                    |datums: Vec<*mut pg_sys::Datum>, is_nulls: Vec<bool>| -> ArrayRef {
                        let vec: Vec<Option<u32>> = (0..datums.len())
                            .map(|idx| {
                                if is_nulls[idx] {
                                    None
                                } else {
                                    unsafe { u32::from_datum(*datums[idx], false) }
                                }
                            })
                            .collect();

                        Arc::new(UInt32Array::from(vec))
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
                    |datums: Vec<*mut pg_sys::Datum>, is_nulls: Vec<bool>| -> ArrayRef {
                        let vec: Vec<Option<f32>> = (0..datums.len())
                            .map(|idx| {
                                if is_nulls[idx] {
                                    None
                                } else {
                                    unsafe { f32::from_datum(*datums[idx], false) }
                                }
                            })
                            .collect();

                        Arc::new(Float32Array::from(vec))
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
                    |datums: Vec<*mut pg_sys::Datum>, is_nulls: Vec<bool>| -> ArrayRef {
                        let vec: Vec<Option<f64>> = (0..datums.len())
                            .map(|idx| {
                                if is_nulls[idx] {
                                    None
                                } else {
                                    unsafe { f64::from_datum(*datums[idx], false) }
                                }
                            })
                            .collect();

                        Arc::new(Float64Array::from(vec))
                    },
                ),
            }),
            DataType::Time64(datafusion::arrow::datatypes::TimeUnit::Microsecond) => f(DatafusionMap {
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
                    |datums: Vec<*mut pg_sys::Datum>, is_nulls: Vec<bool>| -> ArrayRef {
                        let vec: Vec<Option<i64>> = (0..datums.len())
                            .map(|idx| {
                                if is_nulls[idx] {
                                    None
                                } else {
                                    unsafe { i64::from_datum(*datums[idx], false) }
                                }
                            })
                            .collect();

                        Arc::new(Time64MicrosecondArray::from(vec))
                    },
                ),
            }),
            DataType::Timestamp(
                datafusion::arrow::datatypes::TimeUnit::Microsecond,
                None,
            ) => f(DatafusionMap {
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
                    |datums: Vec<*mut pg_sys::Datum>, is_nulls: Vec<bool>| -> ArrayRef {
                        let vec: Vec<Option<i64>> = (0..datums.len())
                            .map(|idx| {
                                if is_nulls[idx] {
                                    None
                                } else {
                                    unsafe { i64::from_datum(*datums[idx], false) }
                                }
                            })
                            .collect();

                        Arc::new(TimestampMicrosecondArray::from(vec))
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
                    |datums: Vec<*mut pg_sys::Datum>, is_nulls: Vec<bool>| -> ArrayRef {
                        let vec: Vec<Option<i32>> = (0..datums.len())
                            .map(|idx| {
                                if is_nulls[idx] {
                                    None
                                } else {
                                    unsafe { i32::from_datum(*datums[idx], false) }
                                }
                            })
                            .collect();

                        Arc::new(Date32Array::from(vec))
                    },
                ),
            }),
            _ => todo!(),
        };

        Ok(result)
    }
}
