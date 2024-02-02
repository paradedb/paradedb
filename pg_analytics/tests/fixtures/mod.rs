mod db;
mod tables;

use async_std::task::block_on;
pub use db::*;
use rstest::*;
use sqlx::PgConnection;
pub use tables::*;

#[fixture]
pub fn database() -> Db {
    block_on(async { Db::new().await })
}

#[fixture]
pub fn conn(database: Db) -> PgConnection {
    block_on(async { database.connection().await })
}
