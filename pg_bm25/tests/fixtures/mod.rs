mod db;
mod tables;

pub use db::*;
use rstest::*;
use sqlx::PgConnection;
pub use tables::*;

#[fixture]
pub async fn database() -> Db {
    Db::new().await
}

#[fixture]
pub fn conn(#[future] database: Db) -> PgConnection {
    async_std::task::block_on(async { database.await.connection().await })
}
