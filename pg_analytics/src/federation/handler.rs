use datafusion_federation::{FederatedQueryPlanner, FederationAnalyzerRule};
use datafusion_federation_sql::{MultiSchemaProvider, SQLFederationProvider, SQLSchemaProvider};
use deltalake::datafusion::catalog::schema::SchemaProvider;
use deltalake::datafusion::common::DataFusionError;
use deltalake::datafusion::dataframe::DataFrame;
use deltalake::datafusion::execution::config::SessionConfig;
use deltalake::datafusion::execution::context::SessionState;
use deltalake::datafusion::execution::runtime_env::{RuntimeConfig, RuntimeEnv};
use deltalake::datafusion::prelude::SessionContext;
use pgrx::*;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

use crate::federation::executor::{ColumnExecutor, RowExecutor};
use crate::federation::{COLUMN_FEDERATION_KEY, ROW_FEDERATION_KEY};

pub async fn get_federated_dataframe(
    query: String,
    classified_tables: HashMap<&'static str, Vec<PgRelation>>,
) -> Result<DataFrame, FederatedHandlerError> {
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
        .ok_or(FederatedHandlerError::CatalogNotFound(
            options.default_catalog.clone(),
        ))?;

    // Map schema names to maps of table type to vectors of table names
    let mut schema_map = HashMap::<String, HashMap<&'static str, Vec<String>>>::new();
    for (table_type, table_vec) in classified_tables.iter() {
        for table_relation in table_vec {
            let table_name = table_relation.name().to_string();
            let schema_name = table_relation.namespace().to_string();
            schema_map
                .entry(schema_name)
                .or_default()
                .entry(table_type)
                .or_default()
                .push(table_name)
        }
    }

    // Register a MultiSchemaProvider for each schema with SQLSchemaProviders for each table type
    for (schema_name, table_map) in schema_map.iter() {
        let mut federation_providers: HashMap<&str, Arc<SQLFederationProvider>> = HashMap::new();
        federation_providers.insert(
            ROW_FEDERATION_KEY,
            Arc::new(SQLFederationProvider::new(Arc::new(RowExecutor::new(
                schema_name.clone(),
            )?))),
        );
        federation_providers.insert(
            COLUMN_FEDERATION_KEY,
            Arc::new(SQLFederationProvider::new(Arc::new(ColumnExecutor::new(
                schema_name.clone(),
            )?))),
        );

        let mut schema_providers: Vec<Arc<dyn SchemaProvider>> = vec![];
        for (table_type, table_vec) in table_map.iter() {
            schema_providers.push(Arc::new(
                SQLSchemaProvider::new_with_tables(
                    federation_providers
                        .get(table_type)
                        .ok_or(FederatedHandlerError::FederationProviderNotFound(
                            table_type.to_string(),
                        ))?
                        .clone(),
                    table_vec.clone(),
                )
                .await?,
            ))
        }
        let federation_schema_provider = MultiSchemaProvider::new(schema_providers);
        catalog.register_schema(schema_name, Arc::new(federation_schema_provider))?;
    }

    let ctx = SessionContext::new_with_state(state);
    let df = ctx.sql(query.as_str()).await?;

    Ok(df)
}

#[derive(Error, Debug)]
pub enum FederatedHandlerError {
    #[error(transparent)]
    DataFusionError(#[from] DataFusionError),

    #[error("Catalog {0} not found")]
    CatalogNotFound(String),

    #[error("Federation provider not found for table type {0}")]
    FederationProviderNotFound(String),
}
