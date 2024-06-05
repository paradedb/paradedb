mod fixtures;

use anyhow::Result;
use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
async fn test_sigint(database: Db) -> Result<()> {
    let mut first_conn = database.connection().await;
    let mut second_conn = database.connection().await;

    sqlx::query("CREATE EXTENSION pg_lakehouse;")
        .execute(&mut first_conn)
        .await
        .expect("could not create extension pg_lakehouse");
}
