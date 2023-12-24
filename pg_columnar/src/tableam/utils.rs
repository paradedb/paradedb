use async_std::task;
use datafusion::arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use datafusion::arrow::record_batch::RecordBatch;

use datafusion::dataframe::DataFrameWriteOptions;
use datafusion::datasource::MemTable;

use datafusion::prelude::{ParquetReadOptions, SessionContext};
use lazy_static::lazy_static;
use pgrx::*;
use std::ffi::{CStr, CString};
use std::sync::Arc;

// Let's try adding the session context globally for now so we can retain info about our tables
lazy_static! {
    pub static ref CONTEXT: SessionContext = SessionContext::new();
}

pub unsafe fn create_from_pg(pgrel: &PgRelation, persistence: u8) -> Result<(), String> {
    let table_name = name_from_pg(pgrel);
    let fields = fields_from_pg(pgrel)?;
    let schema = Schema::new(fields);

    match persistence {
        pg_sys::RELPERSISTENCE_UNLOGGED => {
            return Err("Unlogged tables are not yet supported".to_string());
        }
        pg_sys::RELPERSISTENCE_TEMP => {
            match MemTable::try_new(schema.clone().into(), vec![Vec::<RecordBatch>::new()]).ok() {
                Some(mem_table) => {
                    let _ = CONTEXT.register_table(table_name.clone(), Arc::new(mem_table));
                }
                None => return Err("An unexpected error occured creating the table".to_string()),
            };
        }
        pg_sys::RELPERSISTENCE_PERMANENT => {
            let batch = RecordBatch::new_empty(Arc::new(schema.clone()));
            let df = CONTEXT
                .read_batch(batch)
                .expect("Could not create dataframe");

            let _ = task::block_on(df.write_parquet(
                get_parquet_directory(&table_name).as_str(),
                DataFrameWriteOptions::new(),
                None,
            ));

            let _ = task::block_on(CONTEXT.register_parquet(
                &table_name.clone(),
                get_parquet_directory(&table_name).as_str(),
                ParquetReadOptions::default(),
            ));
        }
        _ => return Err("Unsupported persistence type".to_string()),
    };

    Ok(())
}

pub fn name_from_pg(pgrel: &PgRelation) -> String {
    format!("{}", pgrel.oid()).replace("oid=#", "")
}

pub unsafe fn get_parquet_directory(table_name: &str) -> String {
    let option_name_cstr = CString::new("data_directory").expect("failed to create CString");
    let data_dir_str = String::from_utf8(
        CStr::from_ptr(pg_sys::GetConfigOptionByName(
            option_name_cstr.as_ptr(),
            std::ptr::null_mut(),
            true,
        ))
        .to_bytes()
        .to_vec(),
    )
    .expect("Failed to convert C string to Rust string");
    format!("{}/{}/{}", data_dir_str, "paradedb", table_name)
}

fn fields_from_pg(pgrel: &PgRelation) -> Result<Vec<Field>, String> {
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
    let nullability = !attribute.attnotnull;

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
            PgBuiltInOids::BPCHAROID => Ok(Field::new(attname, DataType::Utf8, nullability)),
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
            PgBuiltInOids::TEXTOID | PgBuiltInOids::VARCHAROID => {
                Ok(Field::new(attname, DataType::Utf8, nullability))
            }
            PgBuiltInOids::TIMEOID => Ok(Field::new(
                attname,
                DataType::Time32(TimeUnit::Second),
                nullability,
            )),
            PgBuiltInOids::TIMESTAMPOID => Ok(Field::new(
                attname,
                DataType::Timestamp(TimeUnit::Second, None),
                nullability,
            )),
            PgBuiltInOids::DATEOID => Ok(Field::new(attname, DataType::Date32, nullability)),
            PgBuiltInOids::TIMESTAMPTZOID => {
                Err("Timestamp with time zone data type not supported".to_string())
            }
            PgBuiltInOids::TIMETZOID => {
                Err("Time with time zone data type not supported".to_string())
            }
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
