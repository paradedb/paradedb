use async_std::task;
use deltalake::datafusion::arrow::datatypes::{DataType, Field, Schema as ArrowSchema};
use deltalake::datafusion::arrow::record_batch::RecordBatch;
use deltalake::datafusion::common::DFSchema;

use deltalake::datafusion::datasource::provider_as_source;
use deltalake::datafusion::logical_expr::TableSource;
use deltalake::datafusion::sql::TableReference;
use deltalake::errors::DeltaTableError;
use deltalake::operations::DeltaOps;
use deltalake::parquet::file::properties::{WriterProperties, WriterVersion};
use deltalake::schema::{Schema as DeltaSchema, SchemaField};
use deltalake::table::DeltaTable;
use deltalake::writer::{DeltaWriter, RecordBatchWriter};
use pgrx::*;
use std::sync::Arc;

use crate::datafusion::context::DatafusionContext;
use crate::datafusion::directory::ParquetDirectory;
use crate::datafusion::error::{datafusion_err_to_string, delta_err_to_string};
use crate::datafusion::registry::{PARADE_CATALOG, PARADE_SCHEMA};
use crate::datafusion::schema::ParadeSchemaProvider;
use crate::datafusion::substrait::SubstraitTranslator;

pub struct ParadeTable {
    name: String,
    oid: u32,
}

impl ParadeTable {
    pub fn from_pg(pg_relation: &PgRelation) -> Result<Self, String> {
        let name = pg_relation.name().to_string();
        let oid = pg_relation.oid().as_u32();

        Ok(Self { name, oid })
    }

    pub fn name(&self) -> Result<String, String> {
        Ok(self.name.clone())
    }

    pub fn oid(&self) -> Result<u32, String> {
        Ok(self.oid)
    }

    pub fn schema(&self) -> Result<DFSchema, String> {
        let source = Self::source(self)?;
        DFSchema::try_from_qualified_schema(&self.name, source.schema().as_ref())
            .map_err(datafusion_err_to_string())
    }

    pub fn source(&self) -> Result<Arc<dyn TableSource>, String> {
        DatafusionContext::with_read(|context| {
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

    pub fn create(pg_relation: &PgRelation) -> Result<Self, String> {
        let table_oid = pg_relation.oid().as_u32().to_string();
        let table_name = pg_relation.name();
        let fields = Self::fields(pg_relation)?;
        let arrow_schema = ArrowSchema::new(fields);
        let delta_schema =
            DeltaSchema::try_from(&arrow_schema).expect("Could not convert Arrow to Delta schema");
        let batch = RecordBatch::new_empty(Arc::new(arrow_schema));

        let mut table = task::block_on(Self::create_delta_table(
            &ParquetDirectory::table_path(&table_oid)?,
            delta_schema.get_fields().to_vec(),
        ))?;

        let writer_properties = WriterProperties::builder()
            .set_writer_version(WriterVersion::PARQUET_2_0)
            .build();

        let mut writer = RecordBatchWriter::for_table(&table)
            .map_err(delta_err_to_string())?
            .with_writer_properties(writer_properties);

        task::block_on(writer.write(batch)).map_err(delta_err_to_string())?;
        task::block_on(writer.flush_and_commit(&mut table)).map_err(delta_err_to_string())?;

        DatafusionContext::with_read(|context| {
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

            Ok(Self {
                name: table_name.to_string(),
                oid: pg_relation.oid().as_u32(),
            })
        })
    }

    fn fields(pg_relation: &PgRelation) -> Result<Vec<Field>, String> {
        let tupdesc = pg_relation.tuple_desc();
        let mut fields = Vec::with_capacity(tupdesc.len());

        for attribute in tupdesc.iter() {
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
                DataType::from_substrait(base_oid.to_substrait()?)?,
                nullability,
            );

            fields.push(field);
        }

        Ok(fields)
    }

    async fn create_delta_table(
        table_path: &str,
        fields: Vec<SchemaField>,
    ) -> Result<DeltaTable, String> {
        let maybe_table = deltalake::open_table(table_path).await;

        match maybe_table {
            Err(DeltaTableError::NotATable(_)) => Ok(DeltaOps::try_from_uri(table_path)
                .await
                .map_err(delta_err_to_string())?
                .create()
                .with_columns(fields)
                .await
                .map_err(delta_err_to_string())?),
            Err(err) => Err(err.to_string()),
            Ok(_) => Err(format!("Table at {:?} already exists", table_path)),
        }
    }
}
