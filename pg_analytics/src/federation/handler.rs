use async_std::task;
use std::collections::HashMap;
use std::sync::Arc;

use datafusion_federation::{FederatedQueryPlanner, FederationAnalyzerRule};
use datafusion_federation_sql::{MultiSchemaProvider, SQLFederationProvider, SQLSchemaProvider};
use deltalake::datafusion::arrow::record_batch::RecordBatch;
use deltalake::datafusion::catalog::schema::SchemaProvider;
use deltalake::datafusion::execution::config::SessionConfig;
use deltalake::datafusion::execution::context::SessionState;
use deltalake::datafusion::execution::runtime_env::{RuntimeConfig, RuntimeEnv};
use deltalake::datafusion::prelude::SessionContext;

use crate::errors::{NotFound, ParadeError};
use crate::federation::executor::{ColumnExecutor, RowExecutor};
use crate::federation::TableDetails;

use pgrx::*;

pub async fn execute_federated_query(
    query: String,
    row_tables: Vec<TableDetails>,
    col_tables: Vec<TableDetails>,
) -> Result<Vec<RecordBatch>, ParadeError> {
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

    // schema_name: map of (type: vec[table_name])
    let mut schema_map = HashMap::<String, HashMap<String, Vec<String>>>::new();
    for (table_type, table_vec) in vec![
        ("row".to_string(), row_tables),
        ("col".to_string(), col_tables),
    ] {
        for table_details in table_vec {
            let table_name = table_details.table;
            let schema_name = table_details.schema;
            match schema_map.get(&schema_name) {
                Some(type_map) => {
                    let mut type_map = type_map.clone();
                    match type_map.get(&table_type) {
                        Some(table_type_vec) => {
                            let mut table_type_vec = table_type_vec.clone();
                            table_type_vec.push(table_name);
                            type_map.insert(table_type.clone(), table_type_vec);
                        }
                        None => {
                            type_map.insert(table_type.clone(), vec![table_name]);
                        }
                    }
                    schema_map.insert(schema_name, type_map);
                }
                None => {
                    let mut type_map = HashMap::<String, Vec<String>>::new();
                    type_map.insert(table_type.clone(), vec![table_name]);
                    schema_map.insert(schema_name, type_map);
                }
            }
        }
    }

    for (schema_name, table_map) in schema_map.iter() {
        let mut schema_providers: Vec<Arc<dyn SchemaProvider>> = vec![];
        let row_executor = RowExecutor::new(schema_name.to_string())?;
        let col_executor = ColumnExecutor::new(schema_name.to_string())?;
        let row_federation_provider = Arc::new(SQLFederationProvider::new(Arc::new(row_executor)));
        let col_federation_provider = Arc::new(SQLFederationProvider::new(Arc::new(col_executor)));
        for (table_type, table_vec) in table_map.iter() {
            match table_type.as_str() {
                "row" => schema_providers.push(Arc::new(task::block_on(
                    SQLSchemaProvider::new_with_tables(
                        row_federation_provider.clone(),
                        table_vec.to_vec(),
                    ),
                )?)),
                "col" => schema_providers.push(Arc::new(task::block_on(
                    SQLSchemaProvider::new_with_tables(
                        col_federation_provider.clone(),
                        table_vec.to_vec(),
                    ),
                )?)),
                _ => {
                    return Err(ParadeError::Generic(
                        "Only row and col table types can be federated".to_string(),
                    ))
                }
            }
        }
        let federation_schema_provider = MultiSchemaProvider::new(schema_providers);
        catalog.register_schema(schema_name, Arc::new(federation_schema_provider))?;
    }

    let ctx = SessionContext::new_with_state(state);
    let df = ctx.sql(query.as_str()).await?;

    Ok(df.collect().await?)
}
