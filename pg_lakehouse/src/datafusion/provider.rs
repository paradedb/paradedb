use arrow::datatypes::Schema;
use datafusion::catalog::CatalogProvider;
use datafusion::common::DataFusionError;
use datafusion::datasource::listing::{
    ListingOptions, ListingTable, ListingTableConfig, ListingTableUrl,
};
use datafusion::datasource::TableProvider;
use datafusion::execution::context::SessionState;
use deltalake::DeltaTableError;
use derive_builder::Builder;
use iceberg::arrow::arrow_schema_to_schema;
use iceberg::io::{S3_ACCESS_KEY_ID, S3_REGION, S3_SECRET_ACCESS_KEY};
use iceberg::{Catalog, TableIdent};
use iceberg_catalog_glue::{
    GlueCatalog, GlueCatalogConfig, AWS_ACCESS_KEY_ID, AWS_REGION_NAME, AWS_SECRET_ACCESS_KEY,
};
use iceberg_datafusion::IcebergCatalogProvider;
use pgrx::PgOid;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

use crate::datafusion::format::*;
use crate::schema::attribute::*;

pub async fn create_listing_provider(
    path: &str,
    extension: &str,
    state: &SessionState,
) -> Result<Arc<dyn TableProvider>, TableProviderError> {
    let listing_url = ListingTableUrl::parse(path)?;
    let listing_options = ListingOptions::try_from(FileExtension(extension.to_string()))?;
    let schema = listing_options.infer_schema(state, &listing_url).await?;
    let listing_config = ListingTableConfig::new(listing_url)
        .with_listing_options(listing_options)
        .with_schema(schema);
    let listing_table = ListingTable::try_new(listing_config)?;

    Ok(Arc::new(listing_table) as Arc<dyn TableProvider>)
}

pub async fn create_delta_provider(
    path: &str,
    extension: &str,
) -> Result<Arc<dyn TableProvider>, TableProviderError> {
    if extension != "parquet" {
        return Err(TableProviderError::FileNotParquet(
            extension.to_string(),
            "delta".to_string(),
        ));
    }

    Ok(Arc::new(deltalake::open_table(path).await?) as Arc<dyn TableProvider>)
}

#[derive(Builder, Debug)]
#[builder(setter(into))]
pub struct IcebergTableConfig {
    pub warehouse: String,
    pub database: String,
    pub namespace: String,
    pub table: String,
    #[builder(setter(custom))]
    pub schema: iceberg::spec::Schema,
}

impl IcebergTableConfigBuilder {
    pub fn arrow_schema(&mut self, schema: &Schema) -> Result<&mut Self, iceberg::Error> {
        // self.schema = Some(arrow_schema_to_schema(&schema)?);
        self.schema = Some(
            iceberg::spec::Schema::builder()
                .with_schema_id(0)
                .with_fields(vec![iceberg::spec::NestedField::required(
                    1,
                    "trip_id",
                    iceberg::spec::Type::Primitive(iceberg::spec::PrimitiveType::Int),
                )
                .into()])
                .build()?,
        );

        Ok(self)
    }

    pub fn columns(
        &mut self,
        columns: &[supabase_wrappers::interface::Column],
    ) -> Result<&mut Self, Box<dyn std::error::Error>> {
        let mut arrow_fields = vec![];

        for column in columns {
            let pg_attribute = crate::types::datatype::PgAttribute(
                PgOid::from_untagged(column.type_oid),
                crate::types::datatype::PgTypeMod(column.type_mod),
            );
            let arrow_data_type =
                TryInto::<crate::types::datatype::ArrowDataType>::try_into(pg_attribute)?;
            let arrow_field =
                arrow::datatypes::Field::new(column.name.clone(), arrow_data_type.0, false);
            arrow_fields.push(arrow_field)
        }

        let arrow_schema = arrow::datatypes::Schema::new(arrow_fields);

        Ok(self.arrow_schema(&arrow_schema)?)
    }
}

#[derive(Builder, Debug)]
#[builder(setter(into))]
pub struct IcebergCatalogConfig {
    aws_access_key: String,
    aws_secret_access_key: String,
    aws_region: String,
    #[builder(default = "self.default_s3_access_key()?")]
    s3_access_key: String,
    #[builder(default = "self.default_s3_secret_access_key()?")]
    s3_secret_access_key: String,
    #[builder(default = "self.default_s3_region()?")]
    s3_region: String,
}

impl IcebergCatalogConfig {
    pub async fn table_provider(
        &self,
        table_config: &IcebergTableConfig,
    ) -> Result<Arc<dyn TableProvider>, TableProviderError> {
        let catalog_provider =
            IcebergCatalogProvider::try_new(Arc::new(self.catalog(table_config).await?)).await?;
        let schema_provider = catalog_provider
            .schema(&table_config.namespace)
            .ok_or_else(|| {
                TableProviderError::IcebergSchemaNotFound(
                    table_config.database.to_string(),
                    table_config.namespace.to_string(),
                )
            })?;
        let table = schema_provider
            .table(&table_config.table)
            .await?
            .ok_or_else(|| {
                TableProviderError::IcebergTableNotFound(
                    table_config.database.to_string(),
                    table_config.namespace.to_string(),
                    table_config.table.to_string(),
                )
            })?;

        Ok(table)
    }

    pub async fn catalog(
        &self,
        table_config: &IcebergTableConfig,
    ) -> Result<GlueCatalog, iceberg::Error> {
        // You need to pass both the "AWS_" and "S3_" variants of the config data here.
        // The "AWS_" ones seem to allow you to create a namespace.
        // The "S3_" ones seem to allow you to create a table.
        let props = HashMap::from([
            (AWS_ACCESS_KEY_ID.into(), self.aws_access_key.clone()),
            (
                AWS_SECRET_ACCESS_KEY.into(),
                self.aws_secret_access_key.clone(),
            ),
            (AWS_REGION_NAME.into(), self.aws_region.clone()),
            (S3_ACCESS_KEY_ID.into(), self.s3_access_key.clone()),
            (
                S3_SECRET_ACCESS_KEY.into(),
                self.s3_secret_access_key.clone(),
            ),
            (S3_REGION.into(), self.s3_region.clone()),
        ]);

        // The builder can accept a .uri() input to point to a non-AWS source,
        // but we'll default to AWS for now.
        let config = GlueCatalogConfig::builder()
            .warehouse(table_config.warehouse.to_string())
            .props(props.clone())
            .build();

        GlueCatalog::new(config).await
    }

    pub async fn ensure_namespace(
        &self,
        catalog: &GlueCatalog,
        table_config: &IcebergTableConfig,
    ) -> Result<(), iceberg::Error> {
        let namespace_ident = iceberg::NamespaceIdent::new(table_config.namespace.clone());
        let namespace_exists = catalog.namespace_exists(&namespace_ident).await?;

        if !namespace_exists {
            catalog
                .create_namespace(&namespace_ident, HashMap::new())
                .await?;
        }

        Ok(())
    }

    pub async fn ensure_table(
        &self,
        catalog: &GlueCatalog,
        table_config: &IcebergTableConfig,
    ) -> Result<(), iceberg::Error> {
        let namespace_ident = iceberg::NamespaceIdent::new(table_config.namespace.clone());
        let table_ident = TableIdent::new(namespace_ident.clone(), table_config.table.to_string());
        let table_exists = catalog.table_exists(&table_ident).await?;

        if !table_exists {
            let table_creation = iceberg::TableCreation::builder()
                .location(table_config.warehouse.clone())
                .name(table_config.table.clone())
                .properties(HashMap::new())
                .schema(table_config.schema.clone())
                .build();

            catalog
                .create_table(&namespace_ident, table_creation)
                .await?;
        }

        Ok(())
    }
}

impl IcebergCatalogConfigBuilder {
    pub fn aws_access_key_opt<T: Into<String>>(&mut self, aws_access_key: Option<T>) -> &mut Self {
        if let Some(aws_access_key) = aws_access_key {
            self.aws_access_key = Some(aws_access_key.into());
        }
        self
    }
    pub fn aws_secret_access_key_opt<T: Into<String>>(
        &mut self,
        aws_secret_access_key: Option<T>,
    ) -> &mut Self {
        if let Some(aws_secret_access_key) = aws_secret_access_key {
            self.aws_secret_access_key = Some(aws_secret_access_key.into());
        }
        self
    }
    fn default_s3_access_key(&self) -> Result<String, String> {
        self.aws_access_key
            .clone()
            .ok_or("no aws_access_key set on iceberg catalog config builder".into())
    }
    fn default_s3_secret_access_key(&self) -> Result<String, String> {
        self.aws_secret_access_key
            .clone()
            .ok_or("no aws_secret_access_key set on iceberg catalog config builder".into())
    }
    fn default_s3_region(&self) -> Result<String, String> {
        self.aws_region
            .clone()
            .ok_or("no aws_region set on iceberg catalog config builder".into())
    }
}

#[derive(Error, Debug)]
pub enum TableProviderError {
    #[error(transparent)]
    DataFusion(#[from] DataFusionError),

    #[error(transparent)]
    DeltaTable(#[from] DeltaTableError),

    #[error(transparent)]
    Iceberg(#[from] iceberg::Error),

    #[error("no iceberg schema found for database: {0}, namespace: {1}")]
    IcebergSchemaNotFound(String, String),

    #[error("no iceberg schema found for database: {0}, namespace: {1}, table: {2}")]
    IcebergTableNotFound(String, String, String),

    #[error(transparent)]
    Format(#[from] FormatError),

    #[error(transparent)]
    Options(#[from] supabase_wrappers::prelude::OptionsError),

    #[error(transparent)]
    Schema(#[from] SchemaError),

    #[error(
        "File extension '{0}' is not supported for table format '{1}', extension must be 'parquet'"
    )]
    FileNotParquet(String, String),
}
