use async_std::task;
use datafusion::arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::common::{DFSchema, ScalarValue};
use datafusion::dataframe::DataFrameWriteOptions;
use datafusion::logical_expr::Expr;
use lazy_static::lazy_static;
use parking_lot::RwLock;
use pgrx::*;
use std::sync::Arc;

use crate::datafusion::directory::ParquetDirectory;
use crate::datafusion::error::datafusion_err_to_string;
use crate::datafusion::registry::{CONTEXT, PARADE_CATALOG, PARADE_SCHEMA};
use crate::datafusion::schema::ParadeSchemaProvider;
use crate::datafusion::table::DatafusionTable;

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
    pub static ref BULK_INSERT_STATE: RwLock<BulkInsertState> = RwLock::new(BulkInsertState::new());
}

pub unsafe fn create_from_pg(pgrel: &PgRelation, persistence: u8) -> Result<(), String> {
    let table = DatafusionTable::new(pgrel)?;
    let table_name = table.name()?;
    let fields = get_datafusion_fields_from_pg(pgrel)?;
    let schema = Schema::new(fields);

    let binding = CONTEXT.read();
    let context = (*binding).as_ref().ok_or("Context not initialized")?;

    match persistence {
        pg_sys::RELPERSISTENCE_UNLOGGED => {
            return Err("Unlogged tables are not yet supported".to_string());
        }
        pg_sys::RELPERSISTENCE_TEMP => {
            return Err("Temp tables are not yet supported".to_string());
        }
        pg_sys::RELPERSISTENCE_PERMANENT => {
            let batch = RecordBatch::new_empty(Arc::new(schema));
            let df = context
                .read_batch(batch)
                .map_err(datafusion_err_to_string())?;

            let _ = task::block_on(df.write_parquet(
                &ParquetDirectory::table_path(&table_name)?,
                DataFrameWriteOptions::new(),
                None,
            ));

            let schema_provider = context
                .catalog(PARADE_CATALOG)
                .ok_or("Catalog not found")
                .unwrap()
                .schema(PARADE_SCHEMA)
                .ok_or("Schema not found")
                .unwrap();
            let lister = schema_provider
                .as_any()
                .downcast_ref::<ParadeSchemaProvider>();
            if let Some(lister) = lister {
                task::block_on(lister.refresh(&context.state())).unwrap();
            }
        }
        _ => return Err("Unsupported persistence type".to_string()),
    };

    Ok(())
}

pub unsafe fn get_pg_relation(rte: *mut pg_sys::RangeTblEntry) -> Result<PgRelation, String> {
    let relation = pg_sys::RelationIdGetRelation((*rte).relid);
    Ok(PgRelation::from_pg_owned(relation))
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
