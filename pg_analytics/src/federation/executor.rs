use async_trait::async_trait;
use datafusion_federation_sql::SQLExecutor;
use deltalake::datafusion::arrow::datatypes::SchemaRef;
use deltalake::datafusion::arrow::record_batch::RecordBatch;
use deltalake::datafusion::error::{DataFusionError, Result};
use deltalake::datafusion::logical_expr::LogicalPlan;
use deltalake::datafusion::physical_plan::{
    stream::RecordBatchStreamAdapter, SendableRecordBatchStream,
};
use pgrx::*;

use crate::datafusion::query::QueryString;
use crate::datafusion::session::Session;
use crate::datafusion::table::DatafusionTable;
use crate::errors::ParadeError;
use crate::types::array::IntoArrowArray;
use crate::types::datatype::PgTypeMod;

pub struct ColumnExecutor {
    schema_name: String,
}

impl ColumnExecutor {
    pub fn new(schema_name: String) -> Result<Self> {
        Ok(Self { schema_name })
    }
}
#[async_trait]
impl SQLExecutor for ColumnExecutor {
    fn name(&self) -> &str {
        "column_executor"
    }

    fn compute_context(&self) -> Option<String> {
        Some("col".to_string())
    }

    fn execute(
        &self,
        sql: &str,
        schema: SchemaRef,
    ) -> Result<SendableRecordBatchStream, DataFusionError> {
        let logical_plan = LogicalPlan::try_from(QueryString(sql))?;
        let batch_stream = Session::with_session_context(|context| {
            Box::pin(async move {
                let dataframe = context.execute_logical_plan(logical_plan.clone()).await?;
                Ok(dataframe.execute_stream().await?)
            })
        })?;
        Ok(Box::pin(RecordBatchStreamAdapter::new(
            schema,
            batch_stream,
        )))
    }

    async fn table_names(&self) -> Result<Vec<String>> {
        Err(DataFusionError::NotImplemented(
            "column source: table inference not implemented".to_string(),
        ))
    }

    async fn get_table_schema(&self, table_name: &str) -> Result<SchemaRef, DataFusionError> {
        let pg_relation = unsafe {
            PgRelation::open_with_name(format!("{}.{}", self.schema_name, table_name).as_str())
                .map_err(|err| DataFusionError::External(err.into()))?
        };
        let schema = pg_relation.arrow_schema()?;
        Ok(schema)
    }

    fn dialect(&self) -> &str {
        "postgres"
    }
}

pub struct RowExecutor {
    schema_name: String,
}

impl RowExecutor {
    pub fn new(schema_name: String) -> Result<Self> {
        Ok(Self { schema_name })
    }
}

#[async_trait]
impl SQLExecutor for RowExecutor {
    fn name(&self) -> &str {
        "row_executor"
    }

    fn compute_context(&self) -> Option<String> {
        Some("row".to_string())
    }

    fn execute(
        &self,
        sql: &str,
        schema: SchemaRef,
    ) -> Result<SendableRecordBatchStream, DataFusionError> {
        let mut col_arrays = vec![];
        Spi::connect(|client| {
            let mut cursor = client.open_cursor(sql, None);
            let schema_tuple_table = cursor.fetch(0)?;

            let num_cols = schema_tuple_table.columns()?;
            let mut col_datums: Vec<Vec<Option<pg_sys::Datum>>> =
                (0..num_cols).map(|_| vec![]).collect();

            // We can only get the typmod from the raw tuptable
            let raw_schema_tuple_table = unsafe { pg_sys::SPI_tuptable };
            let tuple_attrs = unsafe { (*(*raw_schema_tuple_table).tupdesc).attrs.as_mut_ptr() };

            // Fill all columns with the appropriate datums
            let mut tuple_table;
            loop {
                tuple_table = cursor.fetch(1)?;
                if tuple_table.is_empty() {
                    break;
                }
                tuple_table = tuple_table.first();
                for (col_idx, col) in col_datums.iter_mut().enumerate().take(num_cols) {
                    col.push(tuple_table.get_datum_by_ordinal(col_idx + 1)?);
                }
            }

            // Convert datum columns to arrow arrays
            for col_idx in 0..num_cols {
                let oid = tuple_table.column_type_oid(col_idx + 1)?;
                let typmod = unsafe { (*tuple_attrs.add(col_idx)).atttypmod };

                col_arrays.push(
                    col_datums[col_idx]
                        .clone()
                        .into_iter()
                        .into_arrow_array(oid, PgTypeMod(typmod))?,
                );
            }

            Ok::<(), ParadeError>(())
        })?;

        let record_batch = RecordBatch::try_new(schema.clone(), col_arrays)?;
        let stream = futures::stream::once(async move { Ok(record_batch) });
        Ok(Box::pin(RecordBatchStreamAdapter::new(schema, stream)))
    }

    async fn table_names(&self) -> Result<Vec<String>> {
        Err(DataFusionError::NotImplemented(
            "row source: table inference not implemented".to_string(),
        ))
    }

    async fn get_table_schema(&self, table_name: &str) -> Result<SchemaRef, DataFusionError> {
        let pg_relation = unsafe {
            PgRelation::open_with_name(format!("{}.{}", self.schema_name, table_name).as_str())
                .map_err(|err| DataFusionError::External(err.into()))?
        };
        let schema = pg_relation.arrow_schema()?;
        Ok(schema)
    }

    fn dialect(&self) -> &str {
        "postgres"
    }
}
