use async_std::task;

use deltalake::datafusion::common::DFSchema;

use deltalake::datafusion::datasource::provider_as_source;
use deltalake::datafusion::logical_expr::TableSource;
use deltalake::datafusion::sql::TableReference;

use pgrx::*;
use std::sync::Arc;

use crate::datafusion::context::DatafusionContext;

use crate::datafusion::error::datafusion_err_to_string;

pub struct ParadeTable {
    name: String,
}

impl ParadeTable {
    pub fn from_pg(pg_relation: &PgRelation) -> Result<Self, String> {
        let name = pg_relation.name().to_string();
        Ok(Self { name })
    }

    pub fn name(&self) -> Result<String, String> {
        Ok(self.name.clone())
    }

    pub fn schema(&self) -> Result<DFSchema, String> {
        let source = Self::source(self)?;
        DFSchema::try_from_qualified_schema(&self.name, source.schema().as_ref())
            .map_err(datafusion_err_to_string())
    }

    pub fn source(&self) -> Result<Arc<dyn TableSource>, String> {
        DatafusionContext::with_provider_context(|_, context| {
            let reference = TableReference::from(self.name.clone());

            let source = match context.table_exist(&reference) {
                Ok(true) => {
                    let provider = task::block_on(context.table_provider(reference))
                        .map_err(datafusion_err_to_string())?;
                    Some(provider_as_source(provider))
                }
                Ok(false) => None,
                Err(e) => return Err(datafusion_err_to_string()(e)),
            };

            source.ok_or("Table not found. Run CALL paradedb.init(); first.".to_string())
        })
    }
}
