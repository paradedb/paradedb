use async_std::task::block_on;
use rstest::*;
use sqlx::PgConnection;

pub use shared::fixtures::db::*;
#[allow(unused_imports)]
pub use shared::fixtures::tables::*;
#[allow(unused_imports)]
pub use shared::fixtures::utils::*;

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
pub fn conn_with_pg_search(database: Db) -> PgConnection {
    block_on(async {
        let mut conn = database.connection().await;
        sqlx::query("CREATE EXTENSION pg_analytics;")
            .execute(&mut conn)
            .await
            .expect("could not create extension pg_analytics");
        sqlx::query("CREATE EXTENSION pg_search;")
            .execute(&mut conn)
            .await
            .expect("could not create extension pg_search");
        conn
    })
}
