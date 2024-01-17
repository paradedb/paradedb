use async_std::task;

use deltalake::datafusion::arrow::datatypes::Schema as ArrowSchema;
use deltalake::datafusion::common::DFSchema;
use deltalake::datafusion::datasource::provider_as_source;
use deltalake::datafusion::logical_expr::TableSource;
use deltalake::datafusion::sql::TableReference;
use pgrx::*;
use std::sync::Arc;

use crate::datafusion::context::DatafusionContext;
use crate::errors::ParadeError;

pub struct ParadeTable {
    table_name: String,
    schema_name: String,
}

impl ParadeTable {
    pub fn from_pg(pg_relation: &PgRelation) -> Result<Self, ParadeError> {
        let table_name = pg_relation.name().to_string();
        let schema_name = pg_relation.namespace().to_string();

        Ok(Self {
            table_name,
            schema_name,
        })
    }

    pub fn table_name(&self) -> Result<String, ParadeError> {
        Ok(self.table_name.clone())
    }

    pub fn schema_name(&self) -> Result<String, ParadeError> {
        Ok(self.schema_name.clone())
    }

    pub fn arrow_schema(&self) -> Result<Arc<ArrowSchema>, ParadeError> {
        let df_schema = self.df_schema()?;
        Ok(Arc::new(df_schema.into()))
    }

    fn df_schema(&self) -> Result<DFSchema, ParadeError> {
        let source = Self::source(self)?;
        let reference = TableReference::partial(self.schema_name.clone(), self.table_name.clone());

        Ok(DFSchema::try_from_qualified_schema(
            reference,
            source.schema().as_ref(),
        )?)
    }

    fn source(&self) -> Result<Arc<dyn TableSource>, ParadeError> {
        DatafusionContext::with_session_context(|context| {
            let reference =
                TableReference::partial(self.schema_name.clone(), self.table_name.clone());

            match context.table_exist(&reference) {
                Ok(true) => {
                    let table_provider = task::block_on(context.table_provider(reference))?;
                    Ok(provider_as_source(table_provider))
                }
                Ok(false) => Err(ParadeError::ContextNotInitialized(format!(
                    "Table {}.{} not found. Please run `CALL paradedb.init();`.",
                    self.schema_name, self.table_name
                ))),
                Err(err) => Err(ParadeError::DataFusion(err)),
            }
        })
    }
}
