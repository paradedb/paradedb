use async_std::task;
use memoffset::offset_of;
use pgrx::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Condvar;
use once_cell::sync::Lazy;

use deltalake::datafusion::arrow::datatypes::SchemaRef;
use datafusion_federation::{FederatedQueryPlanner, FederationAnalyzerRule};
use datafusion_federation_sql::{MultiSchemaProvider, SQLFederationProvider, SQLSchemaProvider};
use deltalake::datafusion::arrow::record_batch::RecordBatch;
use deltalake::datafusion::catalog::schema::SchemaProvider;
use deltalake::datafusion::error::DataFusionError;
use deltalake::datafusion::execution::config::SessionConfig;
use deltalake::datafusion::execution::context::SessionState;
use deltalake::datafusion::execution::runtime_env::{RuntimeConfig, RuntimeEnv};
use deltalake::datafusion::physical_plan::{
    stream::RecordBatchStreamAdapter, SendableRecordBatchStream,
};
use deltalake::datafusion::prelude::SessionContext;

use crate::errors::{NotFound, ParadeError};
use crate::federation::executor::{ColumnExecutor, RowExecutor};
use crate::federation::{COLUMN_FEDERATION_KEY, ROW_FEDERATION_KEY};
use crate::types::datatype::PgTypeMod;
use crate::types::array::IntoArrowArray;
use deltalake::datafusion::logical_expr::LogicalPlan;
use crate::datafusion::session::Session;
use crate::datafusion::query::QueryString;


struct RowQueryStatus {
    query: String,
    schema: Option::<SchemaRef>,
    complete: bool,
    row: bool,
}

static SQL_QUERY: Lazy<Arc<(Mutex<RowQueryStatus>, Condvar)>> = Lazy::new(|| Arc::new((Mutex::new(RowQueryStatus { query: String::new(), schema: None, complete: false, row: false}), Condvar::new())));
static RB_STREAM: Lazy<Arc<(Mutex<Option<Result<SendableRecordBatchStream, DataFusionError>>>, Condvar)>> = Lazy::new(|| Arc::new((Mutex::new(Option::<Result<SendableRecordBatchStream, DataFusionError>>::None), Condvar::new())));

pub async fn set_active_query(sql: &str, schema: SchemaRef, row: bool) -> Result<SendableRecordBatchStream, DataFusionError> {
    // Signal time for query execution w/ semaphore?
    let (query_lock, query_cvar) = &**SQL_QUERY;
    let mut query_details = query_lock.lock().unwrap();
    (*query_details).query = sql.to_string();
    (*query_details).schema = Some(schema);
    (*query_details).row = row;
    query_cvar.notify_one();

    drop(query_details);

    // Wait for query results
    let (rb_lock, rb_cvar) = &**RB_STREAM;
    let mut rb_stream = rb_lock.lock().unwrap();
    if (*rb_stream).is_none() {
        rb_stream = rb_cvar.wait(rb_stream).unwrap();
    }

    // Save result rb_stream and set shared to None
    let ret_rb_stream = rb_stream.take();
    *rb_stream = None;

    // Return query results
    ret_rb_stream.unwrap()
}

async fn execute_col_sql_query(sql: &str, schema: SchemaRef) -> Result<SendableRecordBatchStream, DataFusionError> {
    let logical_plan = LogicalPlan::try_from(QueryString(sql))?;
    let now = std::time::Instant::now();
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

async fn execute_row_sql_query(sql: &str, schema: SchemaRef) -> Result<SendableRecordBatchStream, DataFusionError> {
    let mut col_arrays = vec![];
    let now = std::time::Instant::now();
    Spi::connect(|client| {
        let mut cursor = client.open_cursor(sql, None);
        let schema_tuple_table = cursor.fetch(0)?;

        let num_cols = schema_tuple_table.columns()?;
        let mut col_datums: Vec<Vec<Option<pg_sys::Datum>>> =
            (0..num_cols).map(|_| vec![]).collect();

        // We can only get the typmod from the raw tuptable
        let raw_schema_tuple_table = unsafe { pg_sys::SPI_tuptable };
        let tuple_attrs = unsafe { (*(*raw_schema_tuple_table).tupdesc).attrs.as_mut_ptr() };

        // Fill all columns with the appropriate datums
        let mut tuple_table;
        // Calculate MAX_TUPLES_PER_PAGE and fetch that many tuples at a time
        let max_tuples = unsafe {
            (pg_sys::BLCKSZ as usize - offset_of!(pg_sys::PageHeaderData, pd_linp))
                / (pg_sys::MAXALIGN(offset_of!(pg_sys::HeapTupleHeaderData, t_bits))
                    + std::mem::size_of::<pg_sys::ItemIdData>())
        };

        // let now = std::time::Instant::now();
        // let mut elapsed_time = now.elapsed();

        loop {
            tuple_table = cursor.fetch(max_tuples as i64)?;

            // elapsed_time = now.elapsed();
            // info!("fetch: {:?} ms", elapsed_time.as_millis());

            tuple_table = tuple_table.first();
            if tuple_table.is_empty() {
                break;
            }
            let mut num_tuple = 1;
            while tuple_table.get_heap_tuple()?.is_some() {
                // elapsed_time = now.elapsed() - elapsed_time;
                // info!("get_heap_tuple: {:?} ms ({:?})", elapsed_time.as_millis(), now.elapsed());
                for (col_idx, col) in col_datums.iter_mut().enumerate().take(num_cols) {
                    col.push(tuple_table.get_datum_by_ordinal(col_idx + 1)?);
                }

                // elapsed_time = now.elapsed() - elapsed_time;
                // info!("push entry datums for tuple {:?}: {:?} ms ({:?})", num_tuple, elapsed_time.as_millis(), now.elapsed());
                num_tuple += 1;

                if tuple_table.next().is_none() {
                    break;
                }
                // elapsed_time = now.elapsed() - elapsed_time;
                // info!("get next tuple: {:?} ms ({:?})", elapsed_time.as_millis(), now.elapsed());
            }
        }
        // let mut elapsed_time = now.elapsed();
        // info!("finished loop: {:?}", now.elapsed());

        // Convert datum columns to arrow arrays
        for (col_idx, col_datum_vec) in col_datums.iter().enumerate().take(num_cols) {
            let oid = tuple_table.column_type_oid(col_idx + 1)?;
            let typmod = unsafe { (*tuple_attrs.add(col_idx)).atttypmod };

            col_arrays.push(
                col_datum_vec
                    .clone()
                    .into_iter()
                    .into_arrow_array(oid, PgTypeMod(typmod))?,
            );
        }
        // elapsed_time = now.elapsed() - elapsed_time;
        // info!("convert datum cols to arrow arrays: {:?} ms", now.elapsed());

        Ok::<(), ParadeError>(())
    })?;

    let record_batch = RecordBatch::try_new(schema.clone(), col_arrays)?;
    let stream = futures::stream::once(async move { Ok(record_batch) });
    info!("finished spi and stream: {:?}", now.elapsed());
    Ok(Box::pin(RecordBatchStreamAdapter::new(schema, stream)))
}

pub async fn get_federated_batches(
    query: String,
    classified_tables: HashMap<&'static str, Vec<PgRelation>>,
) -> Result<Vec<RecordBatch>, ParadeError> {
    // Create a separate session context to process the federated query
    // Can only use one partition because pgrx cannot work on multiple threads
    // let config = SessionConfig::new().with_repartition_joins(false);
    let config = SessionConfig::new();
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
                        .ok_or(ParadeError::Generic(format!(
                            "No federation provider for table type {:?}",
                            table_type
                        )))?
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
    let now = std::time::Instant::now();
    let df = ctx.sql(query.as_str()).await?;

    // TODO: wait on main thread for pgrx sql queries - other threads can pass sql queries to held mutex
    //       run df.collect on another thread. when it is complete, release the main thread and continue.

    /*
    1. Start df.collect
        on each execute:
        a. grab mutex
        b. push query to shared resource
        c. signal main thread to execute query
        d. main thread executes query and pushes recordbatchstream to shared resource
        e. main thread signals calling thread that recordbatchstream is ready
        f. calling thread returns recordbatchstream to caller
        g. release mutex
        when df.collect returns, set indicator to complete
    2. Block until all row sql queries are called
        when indicator is set to complete, exit loop
    */

    // Reset mutexes
    let (query_lock, query_cvar) = &**SQL_QUERY;
    let mut query_details = query_lock.lock().unwrap();
    (*query_details).complete = false;
    (*query_details).schema = None;
    (*query_details).query.clear();
    (*query_details).row = false;
    drop(query_details);

    let (rb_lock, rb_cvar) = &**RB_STREAM;
    let mut rb_stream = rb_lock.lock().unwrap();
    (*rb_stream) = None;
    drop(rb_stream);

    let res = task::spawn(async move {
        let ret = df.collect().await;

        let (query_lock, query_cvar) = &**SQL_QUERY;
        let mut query_details = query_lock.lock().unwrap();
        (*query_details).complete = true;
        query_cvar.notify_one();

        ret
    });


    task::block_on(async move {
        let (query_lock, query_cvar) = &**SQL_QUERY;
        let mut query_details = query_lock.lock().unwrap();

        while !(*query_details).complete {
            if !(*query_details).query.is_empty() {
                let res_rb_stream = if (*query_details).row {
                    execute_row_sql_query((*query_details).query.as_str(), (*query_details).schema.as_ref().unwrap().clone()).await
                } else {
                    execute_col_sql_query((*query_details).query.as_str(), (*query_details).schema.as_ref().unwrap().clone()).await
                };
                let (rb_lock, rb_cvar) = &**RB_STREAM;
                let mut rb_stream = rb_lock.lock().unwrap();
                *rb_stream = Some(res_rb_stream);
                rb_cvar.notify_one();
                drop(rb_stream);
            }
            query_details = query_cvar.wait(query_details).unwrap();
        }
    });

    Ok(res.await?)
}
