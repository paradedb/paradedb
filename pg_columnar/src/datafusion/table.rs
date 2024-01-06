use async_std::task;
use datafusion::arrow::datatypes::{Field, Schema};
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::common::DFSchema;
use datafusion::dataframe::DataFrameWriteOptions;
use datafusion::datasource::provider_as_source;
use datafusion::logical_expr::TableSource;
use datafusion::sql::TableReference;
use pgrx::*;
use std::sync::Arc;

use crate::datafusion::context::CONTEXT;
use crate::datafusion::directory::ParquetDirectory;
use crate::datafusion::error::datafusion_err_to_string;
use crate::datafusion::registry::{PARADE_CATALOG, PARADE_SCHEMA};
use crate::datafusion::schema::ParadeSchemaProvider;
use crate::datafusion::substrait::SubstraitTranslator;

pub struct DatafusionTable {
    name: String,
    source: Option<Arc<dyn TableSource>>,
}

impl DatafusionTable {
    pub fn new(pg_relation: &PgRelation) -> Result<Self, String> {
        let name = Self::get_name_from_pg(pg_relation)?;

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

    pub fn create(pg_relation: &PgRelation) -> Result<Self, String> {
        let name = Self::get_name_from_pg(pg_relation)?;
        let fields = Self::fields(pg_relation)?;
        let schema = Schema::new(fields);
        let batch = RecordBatch::new_empty(Arc::new(schema));

        let context_lock = CONTEXT.read();
        let context = (*context_lock)
            .as_ref()
            .ok_or("No columnar context found. Run SELECT paradedb.init(); first.")?;
        let df = context
            .read_batch(batch)
            .map_err(datafusion_err_to_string())?;

        let _ = task::block_on(df.write_parquet(
            &ParquetDirectory::table_path(&name)?,
            DataFrameWriteOptions::new(),
            None,
        ));

        let schema_provider = context
            .catalog(PARADE_CATALOG)
            .expect("Catalog not found")
            .schema(PARADE_SCHEMA)
            .expect("Schema not found");

        let lister = schema_provider
            .as_any()
            .downcast_ref::<ParadeSchemaProvider>()
            .expect("Failed to downcast schema provider");

        task::block_on(lister.refresh(&context.state()))
            .expect("Failed to refresh schema provider");

        let table = task::block_on(schema_provider.table(&name)).expect("Failed to get table");

        Ok(Self {
            name,
            source: Some(provider_as_source(table)),
        })
    }

    fn fields(pg_relation: &PgRelation) -> Result<Vec<Field>, String> {
        let tupdesc = pg_relation.tuple_desc();
        let mut fields = Vec::with_capacity(tupdesc.len());

        for (_, attribute) in tupdesc.iter().enumerate() {
            if attribute.is_dropped() {
                continue;
            }

            let attname = attribute.name();
            let attribute_type_oid = attribute.type_oid();
            // Setting it to true because of a likely bug in Datafusion where inserts
            // fail on nullability = false fields
            let nullability = true;

            let array_type = unsafe { pg_sys::get_element_type(attribute_type_oid.value()) };
            let (base_oid, is_array) = if array_type != pg_sys::InvalidOid {
                (PgOid::from(array_type), true)
            } else {
                (attribute_type_oid, false)
            };

            if is_array {
                return Err("Array data types are not supported".to_string());
            }

            let field = Field::new(
                attname,
                SubstraitTranslator::from_substrait(base_oid.to_substrait()?)?,
                nullability,
            );

            fields.push(field);
        }

        Ok(fields)
    }

    fn get_name_from_pg(pg_relation: &PgRelation) -> Result<String, String> {
        let name: String = format!("{}", pg_relation.oid())
            .chars()
            .filter(|c| c.is_ascii_digit())
            .collect();

        Ok(name)
    }
}
