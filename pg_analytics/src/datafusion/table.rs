use async_std::task;
use deltalake::datafusion::arrow::datatypes::Schema as ArrowSchema;
use deltalake::datafusion::catalog::schema::SchemaProvider;
use deltalake::datafusion::common::DFSchema;
use deltalake::datafusion::datasource::provider_as_source;
use deltalake::datafusion::datasource::TableProvider;
use deltalake::datafusion::sql::TableReference;
use pgrx::*;
use std::sync::Arc;

use crate::datafusion::context::DatafusionContext;
use crate::errors::ParadeError;

pub trait DeltaTableProvider {
    fn arrow_schema(&self) -> Result<Arc<ArrowSchema>, ParadeError>;
}

impl DeltaTableProvider for PgRelation {
    fn arrow_schema(&self) -> Result<Arc<ArrowSchema>, ParadeError> {
        let table_name = self.name();
        let schema_name = self.namespace();

        let provider = DatafusionContext::with_schema_provider(schema_name, |provider| {
            let delta_table = task::block_on(provider.get_delta_table(table_name))?;
            Ok(provider.register_table(
                table_name.to_string(),
                Arc::new(delta_table) as Arc<dyn TableProvider>,
            )?)
        })?;

        let source = provider_as_source(provider.ok_or(ParadeError::NotFound)?);
        let reference = TableReference::partial(schema_name, table_name);
        let df_schema = DFSchema::try_from_qualified_schema(reference, source.schema().as_ref())?;

        Ok(Arc::new(df_schema.into()))
    }
}
