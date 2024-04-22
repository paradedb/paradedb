use async_std::sync::Mutex;
use async_trait::async_trait;
use deltalake::arrow::error::ArrowError;
use deltalake::datafusion::arrow::datatypes::{DataType, Field, Schema as ArrowSchema};
use deltalake::datafusion::arrow::record_batch::RecordBatch;
use deltalake::datafusion::common::{Constraints, Result as DataFusionResult, Statistics};
use deltalake::datafusion::dataframe::DataFrame;
use deltalake::datafusion::datasource::file_format::parquet::ParquetFormat;
use deltalake::datafusion::datasource::listing::{
    ListingOptions, ListingTable, ListingTableConfig, ListingTableUrl,
};
use deltalake::datafusion::datasource::{provider_as_source, TableProvider};
use deltalake::datafusion::error::Result;
use deltalake::datafusion::execution::context::SessionState;
use deltalake::datafusion::logical_expr::{
    col, lit, Expr, LogicalPlan, LogicalPlanBuilder, TableProviderFilterPushDown, TableType,
};
use deltalake::datafusion::physical_plan::ExecutionPlan;
use deltalake::datafusion::sql::TableReference;
use deltalake::errors::DeltaTableError;
use deltalake::kernel::Schema as DeltaSchema;
use deltalake::operations::create::CreateBuilder;
use deltalake::operations::optimize::OptimizeBuilder;
use deltalake::operations::update::{UpdateBuilder, UpdateMetrics};
use deltalake::operations::vacuum::VacuumBuilder;
use deltalake::writer::{DeltaWriter as DeltaWriterTrait, RecordBatchWriter, WriteMode};
use deltalake::DeltaTable;
use once_cell::sync::Lazy;
use pgrx::*;
use std::any::Any;
use std::collections::{
    hash_map::Entry::{Occupied, Vacant},
    HashMap,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;

use super::catalog::CatalogError;
use super::directory::{DirectoryError, ParadeDirectory};
use super::session::Session;
use crate::guc::PARADE_GUC;
use crate::types::datatype::{ArrowDataType, DataTypeError, PgAttribute, PgTypeMod};

pub static RESERVED_TID_FIELD: &str = "parade_ctid";
pub static RESERVED_XMIN_FIELD: &str = "parade_xmin";
pub static RESERVED_XMAX_FIELD: &str = "parade_xmax";

const BYTES_IN_MB: i64 = 1_048_576;

pub static DELETE_ON_PRECOMMIT_CACHE: Lazy<Arc<Mutex<HashMap<PathBuf, String>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

pub static DROP_ON_PRECOMMIT_CACHE: Lazy<Arc<Mutex<HashMap<PathBuf, String>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

pub static DROP_ON_ABORT_CACHE: Lazy<Arc<Mutex<HashMap<PathBuf, String>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

pub trait DatafusionTable {
    fn arrow_schema(&self) -> Result<ArrowSchema, DataFusionTableError>;
    fn arrow_schema_with_reserved_fields(&self) -> Result<ArrowSchema, DataFusionTableError>;
    fn table_path(&self) -> Result<PathBuf, DataFusionTableError>;
}

impl DatafusionTable for PgRelation {
    fn arrow_schema(&self) -> Result<ArrowSchema, DataFusionTableError> {
        let tupdesc = self.tuple_desc();
        let mut fields = Vec::with_capacity(tupdesc.len());

        for attribute in tupdesc.iter() {
            if attribute.is_dropped() {
                continue;
            }

            let attname = attribute.name();

            if attname == RESERVED_TID_FIELD
                || attname == RESERVED_XMIN_FIELD
                || attname == RESERVED_XMAX_FIELD
            {
                return Err(DataFusionTableError::ReservedFieldName(attname.to_string()));
            }

            let attribute_type_oid = attribute.type_oid();
            let nullability = !attribute.attnotnull;

            let array_type = unsafe { pg_sys::get_element_type(attribute_type_oid.value()) };
            let (base_oid, is_array) = if array_type != pg_sys::InvalidOid {
                (PgOid::from(array_type), true)
            } else {
                (attribute_type_oid, false)
            };

            // Note: even if you have an int[][], the attribute-type is INT4ARRAYOID and the base is INT4OID
            let ArrowDataType(datatype) =
                PgAttribute(base_oid, PgTypeMod(attribute.type_mod())).try_into()?;
            let field = if is_array {
                Field::new_list(
                    attname,
                    Field::new_list_field(
                        datatype,
                        true, // TODO: i think postgres always allows array constants to be null
                    ),
                    nullability,
                )
            } else {
                Field::new(attname, datatype, nullability)
            };

            fields.push(field);
        }

        Ok(ArrowSchema::new(fields))
    }

    fn arrow_schema_with_reserved_fields(&self) -> Result<ArrowSchema, DataFusionTableError> {
        Ok(ArrowSchema::try_merge(vec![
            self.arrow_schema()?,
            ArrowSchema::new(vec![
                Field::new(RESERVED_TID_FIELD, DataType::Int64, false),
                Field::new(RESERVED_XMIN_FIELD, DataType::Int64, false),
                Field::new(RESERVED_XMAX_FIELD, DataType::Int64, false),
            ]),
        ])?)
    }

    fn table_path(&self) -> Result<PathBuf, DataFusionTableError> {
        Ok(ParadeDirectory::table_path_from_name(
            self.namespace(),
            self.name(),
        )?)
    }
}

pub struct Tables {
    tables: HashMap<PathBuf, DeltaTable>,
    schema_name: String,
}

impl Tables {
    pub fn new(schema_name: &str) -> Result<Self, DataFusionTableError> {
        Ok(Self {
            tables: HashMap::new(),
            schema_name: schema_name.to_string(),
        })
    }

    pub async fn alter_schema(
        &mut self,
        table_path: &Path,
        batch: RecordBatch,
    ) -> Result<DeltaTable, DataFusionTableError> {
        let mut delta_table = Self::get_owned(self, table_path).await?;

        // Write the RecordBatch to the DeltaTable
        let mut writer = RecordBatchWriter::for_table(&delta_table)?;
        writer
            .write_with_mode(batch, WriteMode::MergeSchema)
            .await?;
        writer.flush_and_commit(&mut delta_table).await?;

        Ok(delta_table)
    }

    pub async fn create(
        &self,
        table_path: &Path,
        arrow_schema: Arc<ArrowSchema>,
    ) -> Result<DeltaTable, DataFusionTableError> {
        let delta_schema = DeltaSchema::try_from(arrow_schema.as_ref())?;

        let delta_table = CreateBuilder::new()
            .with_location(table_path.to_string_lossy())
            .with_columns(delta_schema.fields().to_vec())
            .await?;

        let mut drop_cache = DROP_ON_ABORT_CACHE.lock().await;
        drop_cache.insert(table_path.to_path_buf(), self.schema_name.clone());

        Ok(delta_table)
    }

    pub async fn logical_delete(
        &mut self,
        table_path: &Path,
        predicate: Option<Expr>,
    ) -> Result<(DeltaTable, UpdateMetrics), DataFusionTableError> {
        let delta_table = Self::get_owned(self, table_path).await?;

        let mut update_builder = UpdateBuilder::new(
            delta_table.log_store(),
            delta_table
                .state
                .ok_or(DataFusionTableError::DeltaTableStateNotFound)?,
        );

        if let Some(predicate) = predicate {
            update_builder = update_builder.with_predicate(predicate);
        }

        update_builder = update_builder.with_update(
            RESERVED_XMAX_FIELD,
            lit(unsafe { pg_sys::GetCurrentTransactionId() }),
        );

        let mut delete_cache = DELETE_ON_PRECOMMIT_CACHE.lock().await;
        delete_cache.insert(table_path.to_path_buf(), self.schema_name.clone());

        Ok(update_builder.await?)
    }

    pub async fn logical_drop(&mut self, table_path: &Path) -> Result<(), DataFusionTableError> {
        self.tables.remove(table_path);

        let mut drop_cache = DROP_ON_PRECOMMIT_CACHE.lock().await;
        drop_cache.insert(table_path.to_path_buf(), self.schema_name.clone());

        Ok(())
    }

    pub async fn get_owned(
        &mut self,
        table_path: &Path,
    ) -> Result<DeltaTable, DataFusionTableError> {
        let mut table = match self.tables.entry(table_path.to_path_buf()) {
            Occupied(entry) => entry.remove(),
            Vacant(_) => deltalake::open_table(table_path.to_string_lossy()).await?,
        };

        table.update().await?;
        Ok(table)
    }

    pub async fn get_ref(
        &mut self,
        table_path: &Path,
    ) -> Result<&mut DeltaTable, DataFusionTableError> {
        let table = match self.tables.entry(table_path.to_path_buf()) {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => {
                entry.insert(deltalake::open_table(table_path.to_string_lossy()).await?)
            }
        };

        table.update().await?;
        Ok(table)
    }

    pub async fn vacuum(
        &mut self,
        table_path: &Path,
        optimize: bool,
    ) -> Result<DeltaTable, DataFusionTableError> {
        let mut delta_table = Self::get_owned(self, table_path).await?;

        if optimize {
            let optimized_table = OptimizeBuilder::new(
                delta_table.log_store(),
                delta_table
                    .state
                    .ok_or(DataFusionTableError::DeltaTableStateNotFound)?,
            )
            .with_target_size(PARADE_GUC.optimize_file_size_mb.get() as i64 * BYTES_IN_MB)
            .await?
            .0;

            delta_table = optimized_table;
        }

        let vacuumed_table = VacuumBuilder::new(
            delta_table.log_store(),
            delta_table
                .state
                .ok_or(DataFusionTableError::DeltaTableStateNotFound)?,
        )
        .with_retention_period(chrono::Duration::days(
            PARADE_GUC.vacuum_retention_days.get() as i64,
        ))
        .with_enforce_retention_duration(PARADE_GUC.vacuum_enforce_retention.get())
        .await?
        .0;

        Ok(vacuumed_table)
    }
}

pub struct PgTableProvider {
    table: DeltaTable,
    dataframe: DataFrame,
    plan: Option<LogicalPlan>,
}

impl PgTableProvider {
    pub async fn new(
        table: DeltaTable,
        schema_name: &str,
        table_name: &str,
    ) -> Result<Self, CatalogError> {
        let table_path = ParadeDirectory::table_path_from_name(schema_name, table_name)?;
        let listing_table_url = ListingTableUrl::parse(table_path.to_str().ok_or(
            DataFusionTableError::ListingTableUrlParseError(table_path.clone()),
        )?)?;
        let file_format = ParquetFormat::new();
        let listing_options =
            ListingOptions::new(Arc::new(file_format)).with_file_extension(".parquet");

        let state =
            Session::with_session_context(|context| Box::pin(async move { Ok(context.state()) }))?;

        let resolved_schema = listing_options
            .infer_schema(&state, &listing_table_url)
            .await?;

        let config = ListingTableConfig::new(listing_table_url)
            .with_listing_options(listing_options)
            .with_schema(resolved_schema);

        let listing_provider = Arc::new(ListingTable::try_new(config)?);

        let all_tuples = Session::with_session_context(|context| {
            Box::pin(async move { Ok(context.read_table(listing_provider.clone())?) })
        })?;

        let uncommitted_inserts = all_tuples.filter(
            col(RESERVED_XMIN_FIELD).eq(lit(unsafe { pg_sys::GetCurrentTransactionId() })),
        )?;

        let reference = TableReference::full(
            Session::catalog_name()?,
            schema_name.to_string(),
            table_name.to_string(),
        );

        let table_scan = LogicalPlanBuilder::scan(
            reference,
            provider_as_source(Arc::new(table.clone()) as Arc<dyn TableProvider>),
            None,
        )?
        .build()?;

        let committed_inserts = DataFrame::new(state, table_scan);
        let dataframe = committed_inserts
            .filter(
                col(RESERVED_XMAX_FIELD).not_eq(lit(unsafe { pg_sys::GetCurrentTransactionId() })),
            )?
            .union(uncommitted_inserts)?;

        Ok(Self {
            table,
            dataframe: dataframe.clone(),
            plan: Some(dataframe.logical_plan().clone()),
        })
    }

    pub fn dataframe(&self) -> DataFrame {
        self.dataframe.clone()
    }
}

#[async_trait]
impl TableProvider for PgTableProvider {
    fn as_any(&self) -> &dyn Any {
        self.table.as_any()
    }

    fn schema(&self) -> Arc<ArrowSchema> {
        self.table.snapshot().unwrap().arrow_schema().unwrap()
    }

    fn table_type(&self) -> TableType {
        self.table.table_type()
    }

    fn get_table_definition(&self) -> Option<&str> {
        self.table.get_table_definition()
    }

    fn get_logical_plan(&self) -> Option<&LogicalPlan> {
        self.plan.as_ref()
    }

    async fn scan(
        &self,
        session: &SessionState,
        projection: Option<&Vec<usize>>,
        filters: &[Expr],
        limit: Option<usize>,
    ) -> DataFusionResult<Arc<dyn ExecutionPlan>> {
        self.table.scan(session, projection, filters, limit).await
    }

    #[allow(deprecated)]
    fn supports_filters_pushdown(
        &self,
        filters: &[&Expr],
    ) -> DataFusionResult<Vec<TableProviderFilterPushDown>> {
        filters
            .iter()
            .map(|filter| self.table.supports_filter_pushdown(filter))
            .collect()
    }

    fn statistics(&self) -> Option<Statistics> {
        self.table.statistics()
    }
}

#[derive(Error, Debug)]
pub enum DataFusionTableError {
    #[error(transparent)]
    ArrowError(#[from] ArrowError),

    #[error(transparent)]
    DataTypeError(#[from] DataTypeError),

    #[error(transparent)]
    DeltaTableError(#[from] DeltaTableError),

    #[error(transparent)]
    DirectoryError(#[from] DirectoryError),

    #[error("Delta table state not found")]
    DeltaTableStateNotFound,

    #[error("Could not convert {0:?} to ListingTableUrl")]
    ListingTableUrlParseError(PathBuf),

    #[error("Column name {0} is reserved by pg_analytics")]
    ReservedFieldName(String),
}
