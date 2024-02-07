use async_std::task::block_on;
use rstest::*;
use shared::sqlx::{self, PgConnection};

pub use shared::fixtures::db::*;
pub use shared::fixtures::tables::*;

#[fixture]
pub fn database() -> Db {
    block_on(async { Db::new().await })
}

#[fixture]
pub fn conn(database: Db) -> PgConnection {
    block_on(async {
        let mut conn = database.connection().await;
        sqlx::query("CREATE EXTENSION pg_analytics;")
            .execute(&mut conn)
            .await
            .expect("could not create extension pg_analytics");
        conn
    })
}

#[fixture]
pub fn user_session_log_table(conn: PgConnection) -> TableConnection<UserSessionLogsTable> {
    TableConnection::setup_new(conn)
}

#[fixture]
pub fn research_project_arrays_table(
    conn: PgConnection,
) -> TableConnection<ResearchProjectArraysTable> {
    TableConnection::setup_new(conn)
}
