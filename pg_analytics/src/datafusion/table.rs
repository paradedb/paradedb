use async_trait::async_trait;
use deltalake::datafusion::arrow::datatypes::{DataType, Field, Schema as ArrowSchema};
use deltalake::datafusion::arrow::record_batch::RecordBatch;
use deltalake::datafusion::common::{Result as DataFusionResult, Statistics};
use deltalake::datafusion::datasource::provider::TableProvider;
use deltalake::datafusion::error::Result;
use deltalake::datafusion::execution::context::SessionState;
use deltalake::datafusion::logical_expr::utils::conjunction;
use deltalake::datafusion::logical_expr::{
    Expr, LogicalPlan, TableProviderFilterPushDown, TableType,
};
use deltalake::datafusion::physical_plan::ExecutionPlan;
use deltalake::delta_datafusion::DeltaScanBuilder;
use deltalake::kernel::Schema as DeltaSchema;
use deltalake::operations::create::CreateBuilder;
use deltalake::operations::delete::{DeleteBuilder, DeleteMetrics};
use deltalake::operations::optimize::OptimizeBuilder;
use deltalake::operations::vacuum::VacuumBuilder;
use deltalake::table::state::DeltaTableState;
use deltalake::writer::{DeltaWriter as DeltaWriterTrait, RecordBatchWriter, WriteMode};
use deltalake::DeltaTable;
use pgrx::*;
use std::any::{type_name, Any};
use std::collections::{
    hash_map::Entry::{Occupied, Vacant},
    HashMap,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use url::Url;

use crate::datafusion::directory::ParadeDirectory;
use crate::datafusion::session::Session;
use crate::errors::{NotFound, ParadeError};
use crate::guc::PARADE_GUC;
use crate::types::datatype::{ArrowDataType, PgAttribute, PgTypeMod};

pub static RESERVED_TID_FIELD: &str = "parade_ctid";
pub static RESERVED_XMIN_FIELD: &str = "parade_xmin";

const BYTES_IN_MB: i64 = 1_048_576;

pub trait DatafusionTable {
    fn arrow_schema(&self) -> Result<ArrowSchema, ParadeError>;
    fn arrow_schema_with_reserved_fields(&self) -> Result<ArrowSchema, ParadeError>;
    fn table_path(&self) -> Result<PathBuf, ParadeError>;
}

impl DatafusionTable for PgRelation {
    fn arrow_schema(&self) -> Result<ArrowSchema, ParadeError> {
        let tupdesc = self.tuple_desc();
        let mut fields = Vec::with_capacity(tupdesc.len());

        for attribute in tupdesc.iter() {
            if attribute.is_dropped() {
                continue;
            }

            let attname = attribute.name();

            if attname == RESERVED_TID_FIELD || attname == RESERVED_XMIN_FIELD {
                return Err(ParadeError::ReservedFieldName(attname.to_string()));
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

    fn arrow_schema_with_reserved_fields(&self) -> Result<ArrowSchema, ParadeError> {
        Ok(ArrowSchema::try_merge(vec![
            self.arrow_schema()?,
            ArrowSchema::new(vec![
                Field::new(RESERVED_TID_FIELD, DataType::Int64, false),
                Field::new(RESERVED_XMIN_FIELD, DataType::Int64, false),
            ]),
        ])?)
    }

    fn table_path(&self) -> Result<PathBuf, ParadeError> {
        ParadeDirectory::table_path(Session::catalog_oid()?, self.namespace_oid(), self.oid())
    }
}

pub struct Tables {
    tables: HashMap<PathBuf, DeltaTable>,
}

impl Tables {
    pub fn new() -> Result<Self, ParadeError> {
        Ok(Self {
            tables: HashMap::new(),
        })
    }

    pub async fn alter_schema(
        &mut self,
        table_path: &Path,
        batch: RecordBatch,
    ) -> Result<DeltaTable, ParadeError> {
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
    ) -> Result<DeltaTable, ParadeError> {
        let delta_schema = DeltaSchema::try_from(arrow_schema.as_ref())?;

        let delta_table = CreateBuilder::new()
            .with_location(table_path.to_string_lossy())
            .with_columns(delta_schema.fields().to_vec())
            .await?;

        Ok(delta_table)
    }

    pub async fn delete(
        &mut self,
        table_path: &Path,
        predicate: Option<Expr>,
    ) -> Result<(DeltaTable, DeleteMetrics), ParadeError> {
        let old_table = Self::get_owned(self, table_path).await?;

        let mut delete_builder = DeleteBuilder::new(
            old_table.log_store(),
            old_table
                .state
                .ok_or(NotFound::Value(type_name::<DeltaTableState>().to_string()))?,
        );

        if let Some(predicate) = predicate {
            delete_builder = delete_builder.with_predicate(predicate);
        }

        Ok(delete_builder.await?)
    }

    pub fn deregister(&mut self, table_path: &Path) -> Result<(), ParadeError> {
        self.tables.remove(table_path);
        Ok(())
    }

    pub async fn get_owned(&mut self, table_path: &Path) -> Result<DeltaTable, ParadeError> {
        let table = match self.tables.entry(table_path.to_path_buf()) {
            Occupied(entry) => entry.remove(),
            Vacant(_) => deltalake::open_table(table_path.to_string_lossy()).await?,
        };

        Ok(table)
    }

    pub async fn get_ref(&mut self, table_path: &Path) -> Result<&mut DeltaTable, ParadeError> {
        let table = match self.tables.entry(table_path.to_path_buf()) {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => {
                entry.insert(deltalake::open_table(table_path.to_string_lossy()).await?)
            }
        };

        Ok(table)
    }

    pub fn register(&mut self, table_path: &Path, table: DeltaTable) -> Result<(), ParadeError> {
        self.tables.insert(table_path.to_path_buf(), table);
        Ok(())
    }

    pub async fn vacuum(
        &mut self,
        table_path: &Path,
        optimize: bool,
    ) -> Result<DeltaTable, ParadeError> {
        let mut old_table = Self::get_owned(self, table_path).await?;

        if optimize {
            let optimized_table = OptimizeBuilder::new(
                old_table.log_store(),
                old_table
                    .state
                    .ok_or(NotFound::Value(type_name::<DeltaTableState>().to_string()))?,
            )
            .with_target_size(PARADE_GUC.optimize_file_size_mb.get() as i64 * BYTES_IN_MB)
            .await?
            .0;

            old_table = optimized_table;
        }

        let vacuumed_table = VacuumBuilder::new(
            old_table.log_store(),
            old_table
                .state
                .ok_or(NotFound::Value(type_name::<DeltaTableState>().to_string()))?,
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
}

impl PgTableProvider {
    #[allow(dead_code)]
    pub fn new(table: DeltaTable) -> Self {
        Self { table }
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
        self.table.get_logical_plan()
    }

    async fn scan(
        &self,
        session: &SessionState,
        projection: Option<&Vec<usize>>,
        filters: &[Expr],
        limit: Option<usize>,
    ) -> DataFusionResult<Arc<dyn ExecutionPlan>> {
        let store = self.table.log_store();
        let env = session.runtime_env();
        let object_store_url = store.object_store_url();
        let url: &Url = object_store_url.as_ref();
        env.register_object_store(url, store.object_store());

        let filter_expr = conjunction(filters.iter().cloned());

        let scan = DeltaScanBuilder::new(self.table.snapshot()?, store, session)
            .with_projection(projection)
            .with_limit(limit)
            .with_filter(filter_expr)
            .build()
            .await?;

        Ok(Arc::new(scan))
    }

    #[allow(deprecated)]
    fn supports_filter_pushdown(
        &self,
        filter: &Expr,
    ) -> DataFusionResult<TableProviderFilterPushDown> {
        self.table.supports_filter_pushdown(filter)
    }

    fn statistics(&self) -> Option<Statistics> {
        self.table.statistics()
    }
}
