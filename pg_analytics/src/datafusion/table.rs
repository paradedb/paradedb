use async_std::task;
use deltalake::datafusion::arrow::datatypes::{DataType, Field, Schema as ArrowSchema};
use deltalake::datafusion::catalog::schema::SchemaProvider;
use deltalake::datafusion::common::DFSchema;
use deltalake::datafusion::datasource::provider_as_source;
use deltalake::datafusion::datasource::TableProvider;
use deltalake::datafusion::sql::TableReference;
use pgrx::*;
use std::sync::Arc;

use crate::datafusion::context::DatafusionContext;
use crate::datafusion::datatype::{DatafusionTypeTranslator, PostgresTypeTranslator};
use crate::errors::{NotFound, ParadeError};

pub trait DeltaTableProvider {
    fn fields(&self) -> Result<Vec<Field>, ParadeError>;
    fn arrow_schema(&self) -> Result<Arc<ArrowSchema>, ParadeError>;
}

impl DeltaTableProvider for PgRelation {
    fn fields(&self) -> Result<Vec<Field>, ParadeError> {
        let tupdesc = self.tuple_desc();
        let mut fields = Vec::with_capacity(tupdesc.len());

        for attribute in tupdesc.iter() {
            if attribute.is_dropped() {
                continue;
            }

            let attname = attribute.name();
            let attribute_type_oid = attribute.type_oid();
            let nullability = !attribute.attnotnull;

            let array_type = unsafe { pg_sys::get_element_type(attribute_type_oid.value()) };
            let (base_oid, is_array) = if array_type != pg_sys::InvalidOid {
                (PgOid::from(array_type), true)
            } else {
                (attribute_type_oid, false)
            };
            // Note: even if you have an int[][], the attribute-type is INT4ARRAYOID and the base is INT4OID

            let field = if is_array {
                Field::new_list(
                    attname,
                    Field::new_list_field(
                        DataType::from_sql_data_type(
                            base_oid.to_sql_data_type(attribute.type_mod())?,
                        )?,
                        true, // TODO: i think postgres always allows array constants to be null
                    ),
                    nullability,
                )
            } else {
                Field::new(
                    attname,
                    DataType::from_sql_data_type(base_oid.to_sql_data_type(attribute.type_mod())?)?,
                    nullability,
                )
            };

            fields.push(field);
        }

        Ok(fields)
    }

    fn arrow_schema(&self) -> Result<Arc<ArrowSchema>, ParadeError> {
        let table_name = self.name();
        let schema_name = self.namespace();
        let postgres_catalog_name = DatafusionContext::postgres_catalog_name()?;

        let provider =
            DatafusionContext::with_pg_permanent_schema_provider(schema_name, |provider| {
                let delta_table = task::block_on(provider.get_delta_table(table_name))?;
                Ok(provider.register_table(
                    table_name.to_string(),
                    Arc::new(delta_table) as Arc<dyn TableProvider>,
                )?)
            })?;

        let source = provider_as_source(provider.ok_or(NotFound::Table(table_name.to_string()))?);
        let reference = TableReference::full(postgres_catalog_name, schema_name, table_name);
        let df_schema = DFSchema::try_from_qualified_schema(reference, source.schema().as_ref())?;

        Ok(Arc::new(df_schema.into()))
    }
}
