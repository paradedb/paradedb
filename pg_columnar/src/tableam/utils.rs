use async_std::task;
use datafusion::arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::common::{DFSchema, ScalarValue};
use datafusion::dataframe::DataFrameWriteOptions;

use datafusion::datasource::MemTable;
use datafusion::logical_expr::Expr;
use datafusion::prelude::SessionContext;
use lazy_static::lazy_static;
use pgrx::*;
use std::ffi::{CStr, CString, NulError};
use std::string::FromUtf8Error;
use std::sync::Arc;
use std::sync::Mutex;

use crate::nodes::utils::{datafusion_err_to_string, get_datafusion_table_name, register_listing_table};

pub struct BulkInsertState {
    pub batches: Vec<RecordBatch>,
    pub schema: Option<DFSchema>,
    pub nslots: usize,
}

impl BulkInsertState {
    pub const fn new() -> Self {
        BulkInsertState {
            batches: vec![],
            schema: None,
            nslots: 0,
        }
    }
}

lazy_static! {
    pub static ref CONTEXT: SessionContext = SessionContext::new();
    pub static ref BULK_INSERT_STATE: Mutex<BulkInsertState> = Mutex::new(BulkInsertState::new());
}

pub unsafe fn create_from_pg(pgrel: &PgRelation, persistence: u8) -> Result<(), String> {
    let table_name = get_datafusion_table_name(pgrel)?;
    let fields = get_datafusion_fields_from_pg(pgrel)?;
    let schema = Schema::new(fields);

    match persistence {
        pg_sys::RELPERSISTENCE_UNLOGGED => {
            return Err("Unlogged tables are not yet supported".to_string());
        }
        pg_sys::RELPERSISTENCE_TEMP => {
            match MemTable::try_new(schema.clone().into(), vec![Vec::<RecordBatch>::new()]).ok() {
                Some(mem_table) => {
                    CONTEXT
                        .register_table(table_name.clone(), Arc::new(mem_table))
                        .map_err(datafusion_err_to_string("Could not register table"))?;
                }
                None => return Err("An unexpected error occured creating the table".to_string()),
            };
        }
        pg_sys::RELPERSISTENCE_PERMANENT => {
            let batch = RecordBatch::new_empty(Arc::new(schema.clone()));
            let df = CONTEXT
                .read_batch(batch)
                .map_err(datafusion_err_to_string("Could not create dataframe"))?;

            let _ = task::block_on(df.write_parquet(
                get_parquet_directory(&table_name)?.as_str(),
                DataFrameWriteOptions::new(),
                None,
            ));

            register_listing_table(&table_name, &schema)?;
        }
        _ => return Err("Unsupported persistence type".to_string()),
    };

    Ok(())
}

pub unsafe fn get_pg_relation(rte: *mut pg_sys::RangeTblEntry) -> Result<PgRelation, String> {
    let relation = pg_sys::RelationIdGetRelation((*rte).relid);
    Ok(PgRelation::from_pg_owned(relation))
}

pub unsafe fn get_parquet_directory(table_name: &str) -> Result<String, String> {
    let option_name_cstr = CString::new("data_directory")
        .map_err(|e: NulError| format!("Failed to create CString: {}", e))?;
    let data_dir_str = String::from_utf8(
        CStr::from_ptr(pg_sys::GetConfigOptionByName(
            option_name_cstr.as_ptr(),
            std::ptr::null_mut(),
            true,
        ))
        .to_bytes()
        .to_vec(),
    )
    .map_err(|e: FromUtf8Error| format!("Failed to convert C string to Rust string: {}", e))?;

    Ok(format!("{}/{}/{}", data_dir_str, "paradedb", table_name))
}

pub unsafe fn datum_to_expr(
    datum: *mut pg_sys::Datum,
    oid: PgOid,
    is_null: bool,
) -> Result<Expr, String> {
    let scalar_value_from_oid = |oid: &PgOid, is_null: bool| -> Result<ScalarValue, String> {
        match oid {
            PgOid::BuiltIn(builtin) => match builtin {
                PgBuiltInOids::BOOLOID => Ok(if is_null {
                    ScalarValue::Boolean(None)
                } else {
                    ScalarValue::Boolean(bool::from_datum(*datum, false))
                }),
                PgBuiltInOids::BPCHAROID | PgBuiltInOids::TEXTOID | PgBuiltInOids::VARCHAROID => {
                    Ok(if is_null {
                        ScalarValue::Utf8(None)
                    } else {
                        ScalarValue::Utf8(String::from_datum(*datum, false))
                    })
                }
                PgBuiltInOids::INT2OID => Ok(if is_null {
                    ScalarValue::Int16(None)
                } else {
                    ScalarValue::Int16(i16::from_datum(*datum, false))
                }),
                PgBuiltInOids::INT4OID => Ok(if is_null {
                    ScalarValue::Int32(None)
                } else {
                    ScalarValue::Int32(i32::from_datum(*datum, false))
                }),
                PgBuiltInOids::INT8OID => Ok(if is_null {
                    ScalarValue::Int64(None)
                } else {
                    ScalarValue::Int64(i64::from_datum(*datum, false))
                }),
                PgBuiltInOids::OIDOID | PgBuiltInOids::XIDOID => Ok(if is_null {
                    ScalarValue::UInt32(None)
                } else {
                    ScalarValue::UInt32(u32::from_datum(*datum, false))
                }),
                PgBuiltInOids::FLOAT4OID => Ok(if is_null {
                    ScalarValue::Float32(None)
                } else {
                    ScalarValue::Float32(f32::from_datum(*datum, false))
                }),
                PgBuiltInOids::FLOAT8OID | PgBuiltInOids::NUMERICOID => Ok(if is_null {
                    ScalarValue::Float64(None)
                } else {
                    ScalarValue::Float64(f64::from_datum(*datum, false))
                }),
                PgBuiltInOids::TIMEOID => Ok(if is_null {
                    ScalarValue::Time32Second(None)
                } else {
                    ScalarValue::Time32Second(i32::from_datum(*datum, false))
                }),
                PgBuiltInOids::TIMESTAMPOID => Ok(if is_null {
                    ScalarValue::TimestampSecond(None, None)
                } else {
                    ScalarValue::TimestampSecond(i64::from_datum(*datum, false), None)
                }),
                PgBuiltInOids::DATEOID => Ok(if is_null {
                    ScalarValue::Date32(None)
                } else {
                    ScalarValue::Date32(i32::from_datum(*datum, false))
                }),
                _ => Err(format!("Unsupported built-in Postgres type: {:?}", builtin)),
            },
            _ => Err("Custom or Invalid data types are not supported".to_string()),
        }
    };

    scalar_value_from_oid(&oid, is_null).map(Expr::Literal)
}

pub fn get_datafusion_fields_from_pg(pgrel: &PgRelation) -> Result<Vec<Field>, String> {
    let tupdesc = pgrel.tuple_desc();
    let mut fields = Vec::with_capacity(tupdesc.len());

    for (_, attribute) in tupdesc.iter().enumerate() {
        if attribute.is_dropped() {
            continue;
        }

        let field = field_from_pg_attribute(*attribute)?;
        fields.push(field);
    }

    Ok(fields)
}

fn field_from_pg_attribute(attribute: pg_sys::FormData_pg_attribute) -> Result<Field, String> {
    let attname = attribute.name();
    let attribute_type_oid = attribute.type_oid();
    // Setting it to true because of a likely bug in Datafusion where inserts
    // fail on nullability = false fields
    let nullability = true;

    let array_type = unsafe { pg_sys::get_element_type(attribute_type_oid.value()) };
    let (base_oid, is_array) = if array_type != pg_sys::InvalidOid {
        (PgOid::from(array_type), true)
    } else {
        (attribute_type_oid, false)
    };

    if is_array {
        panic!("Array data types are not supported");
    }

    match &base_oid {
        PgOid::BuiltIn(builtin) => match builtin {
            PgBuiltInOids::BOOLOID => Ok(Field::new(attname, DataType::Boolean, nullability)),
            PgBuiltInOids::INT2OID => Ok(Field::new(attname, DataType::Int16, nullability)),
            PgBuiltInOids::INT4OID => Ok(Field::new(attname, DataType::Int32, nullability)),
            PgBuiltInOids::INT8OID => Ok(Field::new(attname, DataType::Int64, nullability)),
            PgBuiltInOids::OIDOID | PgBuiltInOids::XIDOID => {
                Ok(Field::new(attname, DataType::UInt32, nullability))
            }
            PgBuiltInOids::FLOAT4OID => Ok(Field::new(attname, DataType::Float32, nullability)),
            PgBuiltInOids::FLOAT8OID | PgBuiltInOids::NUMERICOID => {
                Ok(Field::new(attname, DataType::Float64, nullability))
            }
            PgBuiltInOids::BPCHAROID | PgBuiltInOids::TEXTOID | PgBuiltInOids::VARCHAROID => {
                Ok(Field::new(attname, DataType::Utf8, nullability))
            }
            PgBuiltInOids::TIMEOID => Ok(Field::new(
                attname,
                DataType::Time64(TimeUnit::Microsecond),
                nullability,
            )),
            PgBuiltInOids::TIMETZOID => {
                Err("Time with time zone data type not supported".to_string())
            }
            PgBuiltInOids::DATEOID => Ok(Field::new(attname, DataType::Date32, nullability)),
            PgBuiltInOids::TIMESTAMPOID => Ok(Field::new(
                attname,
                DataType::Timestamp(TimeUnit::Millisecond, None),
                nullability,
            )),
            PgBuiltInOids::TIMESTAMPTZOID => Ok(Field::new(
                attname,
                // Postgres stores all timestamps internally as UTC
                DataType::Timestamp(TimeUnit::Millisecond, Some(Arc::from("UTC"))),
                nullability,
            )),
            PgBuiltInOids::JSONOID | PgBuiltInOids::JSONBOID => {
                Err("JSON data type not supported".to_string())
            }
            _ => Err(format!(
                "schema_from_pg: Unsupported built-in Postgres type: {:?}",
                builtin
            )),
        },
        PgOid::Custom(_custom) => Err("Custom data types are not supported".to_string()),
        PgOid::Invalid => Err(format!("{} has a type oid of InvalidOid", attname)),
    }
}
