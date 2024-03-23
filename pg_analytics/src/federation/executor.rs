use async_trait::async_trait;
use async_std::task;
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
use crate::federation::handler::set_active_query;
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
        let ret = task::block_on(set_active_query(sql, schema, false));
        ret
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
        let ret = task::block_on(set_active_query(sql, schema, true));
        ret
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
