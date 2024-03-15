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

            let mut cols: Vec<Vec<pg_sys::Datum>> = vec![];
            let mut col_oids: Vec<pg_sys::PgOid> = vec![];
            let num_cols = schema_tuple_table.columns()?;

            // Get the result schema
            for i in 0..num_cols {
                // TODO: typmod?
                let type_oid = schema_tuple_table.column_type_oid(i + 1)?;
                col_oids.push(type_oid);
                cols.push(vec![]);
            }

            loop {
                let mut tuple_table = cursor.fetch(1)?;
                if tuple_table.is_empty() {
                    break;
                }
                tuple_table = tuple_table.first();
                for (col_idx, col) in cols.iter_mut().enumerate().take(num_cols) {
                    col.push(tuple_table.get_datum_by_ordinal(col_idx + 1)?.ok_or(
                        ParadeError::Generic(format!("Cannot get datum from {:?}", sql)),
                    )?);
                }
            }

            for col_idx in 0..num_cols {
                col_arrays.push(
                    cols[col_idx]
                        .clone()
                        .into_iter()
                        .map(move |datum| (datum, false))
                        .into_arrow_array(col_oids[col_idx], PgTypeMod(-1))?,
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
