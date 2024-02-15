use async_std::task;
use deltalake::datafusion::catalog::schema::SchemaProvider;
use deltalake::datafusion::catalog::CatalogProvider;
use pgrx::*;
use std::ffi::CStr;
use std::sync::Arc;

use crate::datafusion::context::DatafusionContext;
use crate::datafusion::schema::TempSchemaProvider;
use crate::errors::{NotFound, ParadeError};

const DUMMY_TABLE_NAME: &str = "paradedb_dummy_foreign_parquet_table";

extension_sql!(
    r#"
    CREATE OR REPLACE PROCEDURE create_foreign_parquet_table(
        table_name TEXT,
        foreign_table_name TEXT,
        foreign_nickname TEXT
    ) 
    LANGUAGE C AS 'MODULE_PATHNAME', 'create_foreign_parquet_table';
    "#,
    name = "create_foreign_parquet_table"
);
#[pg_guard]
#[no_mangle]
pub extern "C" fn create_foreign_parquet_table(fcinfo: pg_sys::FunctionCallInfo) {
    create_foreign_parquet_table_impl(fcinfo).unwrap_or_else(|err| {
        panic!("{}", err);
    });
}

fn create_foreign_parquet_table_impl(fcinfo: pg_sys::FunctionCallInfo) -> Result<(), ParadeError> {
    let table_name: String = unsafe { fcinfo::pg_getarg(fcinfo, 0).unwrap() };
    let foreign_table_name: String = unsafe { fcinfo::pg_getarg(fcinfo, 1).unwrap() };
    let foreign_nickname: String = unsafe { fcinfo::pg_getarg(fcinfo, 2).unwrap() };

    let temp_schema_oid = unsafe {
        match direct_function_call::<pg_sys::Oid>(pg_sys::pg_my_temp_schema, &[]) {
            Some(pg_sys::InvalidOid) => {
                spi::Spi::run(&format!("CREATE TEMP TABLE {} (a int)", DUMMY_TABLE_NAME))?;

                match direct_function_call::<pg_sys::Oid>(pg_sys::pg_my_temp_schema, &[]) {
                    Some(pg_sys::InvalidOid) => return Err(NotFound::TempSchemaOid.into()),
                    Some(oid) => oid,
                    _ => return Err(NotFound::TempSchemaOid.into()),
                }
            }
            Some(oid) => oid,
            _ => return Err(NotFound::TempSchemaOid.into()),
        }
    };

    let temp_schema_name =
        unsafe { CStr::from_ptr(pg_sys::get_namespace_name(temp_schema_oid)).to_str()? };

    let listing_table = DatafusionContext::with_object_store_catalog(|catalog| {
        let schema_provider = catalog
            .schema(&foreign_nickname)
            .ok_or(NotFound::Schema(foreign_nickname.to_string()))?;

        task::block_on(schema_provider.table(&foreign_table_name))
            .ok_or(NotFound::Table(foreign_table_name).into())
    })?;

    DatafusionContext::with_postgres_catalog(|catalog| {
        if catalog.schema(&temp_schema_name).is_none() {
            let schema_provider = Arc::new(TempSchemaProvider::new()?);
            catalog.register_schema(&temp_schema_name, schema_provider)?;
        }
        Ok(())
    })?;

    let _ = DatafusionContext::with_temp_schema_provider(temp_schema_name, |provider| {
        Ok(provider.register_table(table_name.clone(), listing_table))
    })?;

    spi::Spi::run(&format!("CREATE TEMP TABLE {} (WatchID BIGINT NOT NULL, JavaEnable SMALLINT NOT NULL, Title TEXT NOT NULL, GoodEvent SMALLINT NOT NULL, EventTime TIMESTAMP NOT NULL, EventDate Date NOT NULL, CounterID INTEGER NOT NULL, ClientIP INTEGER NOT NULL, RegionID INTEGER NOT NULL, UserID BIGINT NOT NULL, CounterClass SMALLINT NOT NULL, OS SMALLINT NOT NULL, UserAgent SMALLINT NOT NULL, URL TEXT NOT NULL, Referer TEXT NOT NULL, IsRefresh SMALLINT NOT NULL, RefererCategoryID SMALLINT NOT NULL, RefererRegionID INTEGER NOT NULL, URLCategoryID SMALLINT NOT NULL, URLRegionID INTEGER NOT NULL, ResolutionWidth SMALLINT NOT NULL, ResolutionHeight SMALLINT NOT NULL, ResolutionDepth SMALLINT NOT NULL, FlashMajor SMALLINT NOT NULL, FlashMinor SMALLINT NOT NULL, FlashMinor2 TEXT NOT NULL, NetMajor SMALLINT NOT NULL, NetMinor SMALLINT NOT NULL, UserAgentMajor SMALLINT NOT NULL, UserAgentMinor VARCHAR(255) NOT NULL, CookieEnable SMALLINT NOT NULL, JavascriptEnable SMALLINT NOT NULL, IsMobile SMALLINT NOT NULL, MobilePhone SMALLINT NOT NULL, MobilePhoneModel TEXT NOT NULL, Params TEXT NOT NULL, IPNetworkID INTEGER NOT NULL, TraficSourceID SMALLINT NOT NULL, SearchEngineID SMALLINT NOT NULL, SearchPhrase TEXT NOT NULL, AdvEngineID SMALLINT NOT NULL, IsArtifical SMALLINT NOT NULL, WindowClientWidth SMALLINT NOT NULL, WindowClientHeight SMALLINT NOT NULL, ClientTimeZone SMALLINT NOT NULL, ClientEventTime TIMESTAMP NOT NULL, SilverlightVersion1 SMALLINT NOT NULL, SilverlightVersion2 SMALLINT NOT NULL, SilverlightVersion3 INTEGER NOT NULL, SilverlightVersion4 SMALLINT NOT NULL, PageCharset TEXT NOT NULL, CodeVersion INTEGER NOT NULL, IsLink SMALLINT NOT NULL, IsDownload SMALLINT NOT NULL, IsNotBounce SMALLINT NOT NULL, FUniqID BIGINT NOT NULL, OriginalURL TEXT NOT NULL, HID INTEGER NOT NULL, IsOldCounter SMALLINT NOT NULL, IsEvent SMALLINT NOT NULL, IsParameter SMALLINT NOT NULL, DontCountHits SMALLINT NOT NULL, WithHash SMALLINT NOT NULL, HitColor CHAR NOT NULL, LocalEventTime TIMESTAMP NOT NULL, Age SMALLINT NOT NULL, Sex SMALLINT NOT NULL, Income SMALLINT NOT NULL, Interests SMALLINT NOT NULL, Robotness SMALLINT NOT NULL, RemoteIP INTEGER NOT NULL, WindowName INTEGER NOT NULL, OpenerName INTEGER NOT NULL, HistoryLength SMALLINT NOT NULL, BrowserLanguage TEXT NOT NULL, BrowserCountry TEXT NOT NULL, SocialNetwork TEXT NOT NULL, SocialAction TEXT NOT NULL, HTTPError SMALLINT NOT NULL, SendTiming INTEGER NOT NULL, DNSTiming INTEGER NOT NULL, ConnectTiming INTEGER NOT NULL, ResponseStartTiming INTEGER NOT NULL, ResponseEndTiming INTEGER NOT NULL, FetchTiming INTEGER NOT NULL, SocialSourceNetworkID SMALLINT NOT NULL, SocialSourcePage TEXT NOT NULL, ParamPrice BIGINT NOT NULL, ParamOrderID TEXT NOT NULL, ParamCurrency TEXT NOT NULL, ParamCurrencyID SMALLINT NOT NULL, OpenstatServiceName TEXT NOT NULL, OpenstatCampaignID TEXT NOT NULL, OpenstatAdID TEXT NOT NULL, OpenstatSourceID TEXT NOT NULL, UTMSource TEXT NOT NULL, UTMMedium TEXT NOT NULL, UTMCampaign TEXT NOT NULL, UTMContent TEXT NOT NULL, UTMTerm TEXT NOT NULL, FromTag TEXT NOT NULL, HasGCLID SMALLINT NOT NULL, RefererHash BIGINT NOT NULL, URLHash BIGINT NOT NULL, CLID INTEGER NOT NULL, PRIMARY KEY (CounterID, EventDate, UserID, EventTime, WatchID)) USING parquet;", table_name))?;
    spi::Spi::run(&format!("DROP TABLE {}", DUMMY_TABLE_NAME))?;
    Ok(())
}
