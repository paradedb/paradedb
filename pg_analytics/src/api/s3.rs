use async_std::task;
use deltalake::datafusion::catalog::listing_schema::ListingSchemaProvider;
use deltalake::datafusion::catalog::CatalogProvider;
use object_store::aws::{AmazonS3Builder, Checksum};
use pgrx::*;
use serde::Serialize;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use url::Url;

use crate::datafusion::context::DatafusionContext;
use crate::errors::{NotFound, ParadeError};

#[derive(PostgresEnum, Serialize)]
pub enum FileFormat {
    Avro,
    Csv,
    Json,
    NdJson,
    Parquet,
}

impl Display for FileFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            FileFormat::Avro => write!(f, "AVRO"),
            FileFormat::Csv => write!(f, "CSV"),
            FileFormat::Json => write!(f, "JSON"),
            FileFormat::NdJson => write!(f, "NDJSON"),
            FileFormat::Parquet => write!(f, "PARQUET"),
        }
    }
}

extension_sql!(
    r#"
    CREATE OR REPLACE PROCEDURE register_s3(
        nickname TEXT,
        file_format FileFormat,
        url TEXT,
        region TEXT,
        access_key_id TEXT DEFAULT NULL,
        secret_access_key TEXT DEFAULT NULL,
        token TEXT DEFAULT NULL,
        endpoint TEXT DEFAULT NULL,
        allow_http BOOLEAN DEFAULT FALSE,
        s3_express BOOLEAN DEFAULT FALSE,
        imdsv1_fallback BOOLEAN DEFAULT FALSE,
        unsigned_payload BOOLEAN DEFAULT FALSE,
        skip_signature BOOLEAN DEFAULT FALSE,
        checksum_algorithm BOOLEAN DEFAULT FALSE,
        metadata_endpoint TEXT DEFAULT NULL,
        proxy_url TEXT DEFAULT NULL,
        proxy_ca_certificate TEXT DEFAULT NULL,
        proxy_excludes TEXT DEFAULT NULL,
        disable_tagging BOOLEAN DEFAULT FALSE,
        has_header BOOLEAN DEFAULT FALSE
    ) 
    LANGUAGE C AS 'MODULE_PATHNAME', 'register_s3';
    "#,
    name = "register_s3"
);
#[pg_guard]
#[no_mangle]
pub extern "C" fn register_s3(fcinfo: pg_sys::FunctionCallInfo) {
    register_s3_impl(fcinfo).unwrap_or_else(|err| {
        panic!("{}", err);
    });
}

fn register_s3_impl(fcinfo: pg_sys::FunctionCallInfo) -> Result<(), ParadeError> {
    let nickname: String = unsafe { fcinfo::pg_getarg(fcinfo, 0).unwrap() };
    let file_format: FileFormat = unsafe { fcinfo::pg_getarg(fcinfo, 1).unwrap() };
    let url: String = unsafe { fcinfo::pg_getarg(fcinfo, 2).unwrap() };
    let region: String = unsafe { fcinfo::pg_getarg(fcinfo, 3).unwrap() };
    let access_key_id: Option<String> = unsafe { fcinfo::pg_getarg(fcinfo, 4) };
    let secret_access_key: Option<String> = unsafe { fcinfo::pg_getarg(fcinfo, 5) };
    let token: Option<String> = unsafe { fcinfo::pg_getarg(fcinfo, 6) };
    let endpoint: Option<String> = unsafe { fcinfo::pg_getarg(fcinfo, 7) };
    let allow_http: Option<bool> = unsafe { fcinfo::pg_getarg(fcinfo, 8) };
    let s3_express: Option<bool> = unsafe { fcinfo::pg_getarg(fcinfo, 9) };
    let imdsv1_fallback: Option<bool> = unsafe { fcinfo::pg_getarg(fcinfo, 10) };
    let unsigned_payload: Option<bool> = unsafe { fcinfo::pg_getarg(fcinfo, 11) };
    let skip_signature: Option<bool> = unsafe { fcinfo::pg_getarg(fcinfo, 12) };
    let checksum_algorithm: Option<bool> = unsafe { fcinfo::pg_getarg(fcinfo, 13) };
    let metadata_endpoint: Option<String> = unsafe { fcinfo::pg_getarg(fcinfo, 14) };
    let proxy_url: Option<String> = unsafe { fcinfo::pg_getarg(fcinfo, 15) };
    let proxy_ca_certificate: Option<String> = unsafe { fcinfo::pg_getarg(fcinfo, 16) };
    let proxy_excludes: Option<String> = unsafe { fcinfo::pg_getarg(fcinfo, 17) };
    let disable_tagging: Option<bool> = unsafe { fcinfo::pg_getarg(fcinfo, 18) };
    let has_header: Option<bool> = unsafe { fcinfo::pg_getarg(fcinfo, 19) };

    let builder = AmazonS3Builder::new()
        .with_url(url.as_str())
        .with_region(region.as_str());

    if let Some(access_key_id) = access_key_id {
        builder.clone().with_access_key_id(access_key_id.as_str());
    }

    if let Some(secret_access_key) = secret_access_key {
        builder
            .clone()
            .with_secret_access_key(secret_access_key.as_str());
    }

    if let Some(token) = token {
        builder.clone().with_token(token.as_str());
    }

    if let Some(endpoint) = endpoint {
        builder.clone().with_endpoint(endpoint.as_str());
    }

    if let Some(allow_http) = allow_http {
        builder.clone().with_allow_http(allow_http);
    }

    if let Some(s3_express) = s3_express {
        builder.clone().with_s3_express(s3_express);
    }

    if let Some(true) = imdsv1_fallback {
        builder.clone().with_imdsv1_fallback();
    }

    if let Some(unsigned_payload) = unsigned_payload {
        builder.clone().with_unsigned_payload(unsigned_payload);
    }

    if let Some(skip_signature) = skip_signature {
        builder.clone().with_skip_signature(skip_signature);
    }

    if let Some(true) = checksum_algorithm {
        builder.clone().with_checksum_algorithm(Checksum::SHA256);
    }

    if let Some(metadata_endpoint) = metadata_endpoint {
        builder
            .clone()
            .with_metadata_endpoint(metadata_endpoint.as_str());
    }

    if let Some(proxy_url) = proxy_url {
        builder.clone().with_proxy_url(proxy_url.as_str());
    }

    if let Some(proxy_ca_certificate) = proxy_ca_certificate {
        builder
            .clone()
            .with_proxy_ca_certificate(proxy_ca_certificate.as_str());
    }

    if let Some(proxy_excludes) = proxy_excludes {
        builder.clone().with_proxy_excludes(proxy_excludes.as_str());
    }

    if let Some(disable_tagging) = disable_tagging {
        builder.clone().with_disable_tagging(disable_tagging);
    }

    let listing_schema_provider = DatafusionContext::with_session_context(|context| {
        let object_store = Arc::new(builder.build()?);

        context
            .runtime_env()
            .register_object_store(&Url::parse(&url)?, object_store.clone());

        let schema_provider = ListingSchemaProvider::new(
            url,
            "".into(),
            context
                .state()
                .table_factories()
                .get(&format!("{}", file_format))
                .ok_or(NotFound::FileFormat(format!("{}", file_format)))?
                .clone(),
            object_store,
            format!("{}", file_format),
            has_header.unwrap_or(false),
        );

        task::block_on(schema_provider.refresh(&context.state()))?;

        Ok(schema_provider)
    })?;

    let _ = DatafusionContext::with_object_store_catalog(|catalog| {
        let _ = catalog.register_schema(&nickname, Arc::new(listing_schema_provider));
        Ok(())
    });

    Ok(())
}
