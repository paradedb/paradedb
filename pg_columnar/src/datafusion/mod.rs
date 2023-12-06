use async_std::task;
use core::ffi::c_char;
use datafusion::arrow::array::{Array, ArrayIter, AsArray, Int32Array, PrimitiveArray, Scalar};
use datafusion::arrow::datatypes::{DataType, Field, Int32Type, Schema, SchemaRef, TimeUnit};
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::datasource::{MemTable, TableProvider};
use datafusion::prelude::SessionContext;
use datafusion::sql::TableReference;
use lazy_static::lazy_static;
use pgrx::pg_sys::*;
use pgrx::*;
use serde::Serialize;
use std::fmt::format;
use std::sync::Arc;

// Let's try adding the session context globally for now so we can retain info about our tables
lazy_static! {
    pub static ref CONTEXT: SessionContext = SessionContext::new();
}

#[derive(Clone, Serialize)]
pub struct DFTable;
impl DFTable {
    pub fn get_from_pg(pgrel: &PgRelation) -> Arc<dyn TableProvider> {
        let table_ref = TableReference::from(format!("{}", (*pgrel).oid()));
        task::block_on(CONTEXT.table_provider(table_ref)).expect("Could not get table")
    }

    pub fn create_from_pg(pgrel: &PgRelation, persistence: u8) {
        let schema = Self::schema_from_pg(&pgrel);
        if persistence == RELPERSISTENCE_PERMANENT {
            panic!("Persisted tables are not yet implemented. For now, try CREATE TEMP TABLE.");
        }

        if persistence == RELPERSISTENCE_UNLOGGED {
            panic!("Unlogged tables are not yet implemented. For now, try CREATE TEMP TABLE.");
        }

        match MemTable::try_new(schema, vec![Vec::<RecordBatch>::new()]).ok() {
            Some(mem_table) => {
                CONTEXT.register_table(format!("{}", pgrel.oid()), Arc::new(mem_table));
            }
            None => panic!("An unexpected error occured creating the table"),
        };
    }

    pub fn schema_from_pg(pgrel: &PgRelation) -> Arc<Schema> {
        let tupdesc = pgrel.tuple_desc();
        let mut fields = Vec::with_capacity(tupdesc.len());

        for (attno, attribute) in tupdesc.iter().enumerate() {
            if attribute.is_dropped() {
                continue;
            }
            let attname = attribute.name();
            let attribute_type_oid = attribute.type_oid();

            let field = {
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
                        PgBuiltInOids::BOOLOID => Field::new(attname, DataType::Boolean, true),
                        PgBuiltInOids::INT2OID => Field::new(attname, DataType::Int16, true),
                        PgBuiltInOids::INT4OID => Field::new(attname, DataType::Int32, true),
                        PgBuiltInOids::INT8OID => Field::new(attname, DataType::Int64, true),
                        PgBuiltInOids::OIDOID | PgBuiltInOids::XIDOID => {
                            Field::new(attname, DataType::UInt32, true)
                        }
                        PgBuiltInOids::FLOAT4OID => Field::new(attname, DataType::Float32, true),
                        PgBuiltInOids::FLOAT8OID | PgBuiltInOids::NUMERICOID => {
                            Field::new(attname, DataType::Float64, true)
                        }
                        PgBuiltInOids::TEXTOID | PgBuiltInOids::VARCHAROID => {
                            Field::new(attname, DataType::Utf8, true)
                        }
                        PgBuiltInOids::TIMEOID => {
                            Field::new(attname, DataType::Time32(TimeUnit::Second), true)
                        }
                        PgBuiltInOids::TIMESTAMPOID => {
                            Field::new(attname, DataType::Timestamp(TimeUnit::Second, None), true)
                        }
                        PgBuiltInOids::DATEOID => Field::new(attname, DataType::Date32, true),
                        PgBuiltInOids::TIMESTAMPTZOID => {
                            panic!("Timestamp with time zone data type not supported")
                        }
                        PgBuiltInOids::TIMETZOID => {
                            panic!("Time with time zone data type not supported")
                        }
                        PgBuiltInOids::JSONOID | PgBuiltInOids::JSONBOID => {
                            panic!("JSON data type not supported")
                        }
                        _ => panic!("Unsupported PostgreSQL type: {:?}", builtin),
                    },
                    PgOid::Custom(_custom) => panic!("Custom data types are not supported"),
                    PgOid::Invalid => panic!("{} has a type oid of InvalidOid", attname),
                    _ => panic!("Unsupported PostgreSQL type oid: {}", base_oid.value()),
                }
            };

            fields.push(field);
        }

        let schema = SchemaRef::new(Schema::new(fields));

        schema
    }
}
