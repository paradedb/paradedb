use async_std::task;
use datafusion::common::DFSchema;
use datafusion::datasource::provider_as_source;
use datafusion::logical_expr::TableSource;
use datafusion::sql::TableReference;
use pgrx::*;
use std::sync::Arc;

use crate::datafusion::error::datafusion_err_to_string;
use crate::datafusion::registry::CONTEXT;

pub struct DatafusionTable {
    name: String,
    source: Option<Arc<dyn TableSource>>,
}

impl DatafusionTable {
    pub fn new(pg_relation: &PgRelation) -> Result<Self, String> {
        // Strip non-numbers from the relation OID
        let name: String = format!("{}", pg_relation.oid())
            .chars()
            .filter(|c| c.is_ascii_digit())
            .collect();

        // Get TableSource
        let context_lock = CONTEXT.read();
        let context = (*context_lock)
            .as_ref()
            .ok_or("No columnar context found. Run SELECT paradedb.init(); first.")?;

        let reference = TableReference::from(name.clone());

        let source = match context.table_exist(&reference) {
            Ok(true) => {
                let provider = task::block_on(context.table_provider(reference))
                    .map_err(datafusion_err_to_string())?;
                Some(provider_as_source(provider))
            }
            Ok(false) => None,
            Err(e) => return Err(datafusion_err_to_string()(e)),
        };

        Ok(Self { name, source })
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
        self.source
            .as_ref()
            .cloned()
            .ok_or("Table not found. Run SELECT paradedb.init(); first.".to_string())
    }
}
