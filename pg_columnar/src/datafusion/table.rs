use async_std::task;

use deltalake::datafusion::common::DFSchema;

use deltalake::datafusion::datasource::provider_as_source;
use deltalake::datafusion::logical_expr::TableSource;
use deltalake::datafusion::sql::TableReference;

use pgrx::*;
use std::sync::Arc;

use crate::datafusion::context::DatafusionContext;
use crate::errors::ParadeError;

pub struct ParadeTable {
    name: String,
}

impl ParadeTable {
    pub fn from_pg(pg_relation: &PgRelation) -> Result<Self, ParadeError> {
        let name = pg_relation.name().to_string();
        Ok(Self { name })
    }

    pub fn name(&self) -> Result<String, ParadeError> {
        Ok(self.name.clone())
    }

    pub fn schema(&self) -> Result<DFSchema, ParadeError> {
        let source = Self::source(self)?;
        DFSchema::try_from_qualified_schema(&self.name, source.schema().as_ref())
            .map_err(ParadeError::DataFusion)
    }

    fn source(&self) -> Result<Arc<dyn TableSource>, ParadeError> {
        DatafusionContext::with_provider_context(|_, context| {
            let reference = TableReference::from(self.name.clone());

            let source = match context.table_exist(&reference) {
                Ok(true) => {
                    let provider = task::block_on(context.table_provider(reference))
                        .map_err(ParadeError::DataFusion)?;
                    Some(provider_as_source(provider))
                }
                Ok(false) => None,
                Err(err) => return Err(ParadeError::DataFusion(err)),
            };

            source.ok_or_else(|| ParadeError::ContextNotInitialized)
        })?
    }
}
