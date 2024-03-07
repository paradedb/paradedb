use async_std::task;
use std::sync::Arc;

use datafusion_federation::{FederatedQueryPlanner, FederationAnalyzerRule};
use datafusion_federation_sql::{MultiSchemaProvider, SQLFederationProvider, SQLSchemaProvider};
use deltalake::datafusion::arrow::record_batch::RecordBatch;
use deltalake::datafusion::execution::config::SessionConfig;
use deltalake::datafusion::execution::context::SessionState;
use deltalake::datafusion::execution::runtime_env::{RuntimeConfig, RuntimeEnv};
use deltalake::datafusion::prelude::SessionContext;

use crate::errors::{NotFound, ParadeError};
use crate::federation::executor::{ColumnExecutor, RowExecutor};

pub async fn execute_federated_query(
    query: String,
    row_tables: Vec<String>,
    col_tables: Vec<String>,
) -> Result<Vec<RecordBatch>, ParadeError> {
    let row_executor = RowExecutor::new()?;
    let col_executor = ColumnExecutor::new()?;

    let row_federation_provider = Arc::new(SQLFederationProvider::new(Arc::new(row_executor)));
    let col_federation_provider = Arc::new(SQLFederationProvider::new(Arc::new(col_executor)));

    let row_schema_provider = Arc::new(task::block_on(SQLSchemaProvider::new_with_tables(
        row_federation_provider,
        row_tables,
    ))?);
    let col_schema_provider = Arc::new(task::block_on(SQLSchemaProvider::new_with_tables(
        col_federation_provider,
        col_tables,
    ))?);

    let federation_schema_provider =
        MultiSchemaProvider::new(vec![row_schema_provider, col_schema_provider]);

    // Create a separate session context to process the federated query
    // Can only use one partition because pgrx cannot work on multiple threads
    let config = SessionConfig::new().with_target_partitions(1);
    let rn_config = RuntimeConfig::new();
    let runtime_env = RuntimeEnv::new(rn_config)?;
    let state = SessionState::new_with_config_rt(config, Arc::new(runtime_env))
        .add_analyzer_rule(Arc::new(FederationAnalyzerRule::new()))
        .with_query_planner(Arc::new(FederatedQueryPlanner::new()));
    let options = &state.config().options().catalog;
    let catalog = state
        .catalog_list()
        .catalog(options.default_catalog.as_str())
        .ok_or(ParadeError::NotFound(NotFound::Catalog(
            options.default_catalog.clone(),
        )))?;
    catalog.register_schema(
        options.default_schema.as_str(),
        Arc::new(federation_schema_provider),
    )?;
    let ctx = SessionContext::new_with_state(state);

    let df = ctx.sql(query.as_str()).await?;

    Ok(df.collect().await?)
}
