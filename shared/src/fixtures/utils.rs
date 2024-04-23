use super::db::*;

use sqlx::PgConnection;
use std::path::PathBuf;

pub fn current_lsn(conn: &mut PgConnection) -> String {
    "SELECT pg_current_wal_lsn()::TEXT"
        .fetch_one::<(String,)>(conn)
        .0
}

pub fn pg_version(conn: &mut PgConnection) -> i32 {
    let version = "SELECT current_setting('server_version_num')::int;"
        .fetch_one::<(i32,)>(conn)
        .0;
    version / 10000
}

pub fn database_oid(conn: &mut PgConnection) -> String {
    let db_name = "SELECT current_database()".fetch_one::<(String,)>(conn).0;

    format!("SELECT oid FROM pg_database WHERE datname='{db_name}'")
        .fetch_one::<(sqlx::postgres::types::Oid,)>(conn)
        .0
         .0
        .to_string()
}

pub fn schema_oid(conn: &mut PgConnection, schema_name: &str) -> String {
    format!("SELECT oid FROM pg_namespace WHERE nspname='{schema_name}'")
        .to_string()
        .fetch_one::<(sqlx::postgres::types::Oid,)>(conn)
        .0
         .0
        .to_string()
}

pub fn table_oid(conn: &mut PgConnection, schema_name: &str, table_name: &str) -> String {
    format!("SELECT oid FROM pg_class WHERE relname='{table_name}' AND relnamespace=(SELECT oid FROM pg_namespace WHERE nspname='{schema_name}')")
        .to_string()
        .fetch_one::<(sqlx::postgres::types::Oid,)>(conn)
        .0
        .0
        .to_string()
}

pub fn default_database_path(conn: &mut PgConnection) -> PathBuf {
    let data_dir = "SHOW data_directory".fetch_one::<(String,)>(conn).0;
    let deltalake_dir = "deltalake";
    let database_oid = database_oid(conn);

    PathBuf::from(&data_dir)
        .join(deltalake_dir)
        .join(database_oid)
}

pub fn default_schema_path(conn: &mut PgConnection, schema_name: &str) -> PathBuf {
    let schema_oid = schema_oid(conn, schema_name);
    default_database_path(conn).join(schema_oid)
}

pub fn default_table_path(conn: &mut PgConnection, schema_name: &str, table_name: &str) -> PathBuf {
    let table_oid = table_oid(conn, schema_name, table_name);
    default_schema_path(conn, schema_name).join(table_oid)
}
