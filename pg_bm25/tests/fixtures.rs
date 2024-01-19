pub use rstest::*;
use sqlx::{
    testing::{TestArgs, TestContext, TestSupport},
    ConnectOptions, PgConnection, Postgres,
};

struct Db {
    context: TestContext<Postgres>,
}

impl Drop for Db {
    fn drop(&mut self) {
        Postgres::cleanup_test(&self.context.db_name);
    }
}

#[fixture]
pub async fn db() -> Db {
    let path = format!("{}::{}", file!(), line!());
    let args = TestArgs::new(Box::leak(path.into_boxed_str()));
    let context = Postgres::test_context(&args)
        .await
        .expect("could not create test database");

    Db { context }
}

#[fixture]
pub async fn conn(#[future] db: Db) -> PgConnection {
    db.await
        .context
        .connect_opts
        .connect()
        .await
        .expect("failed to connect to test database")
}
