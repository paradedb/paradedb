use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::{
    testing::{TestArgs, TestContext, TestSupport},
    ConnectOptions, PgConnection, Postgres,
};

pub struct Db {
    context: TestContext<Postgres>,
}

impl Db {
    pub async fn new() -> Self {
        // Use a timestamp as a unique identifier.
        let path = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros()
            .to_string();

        let args = TestArgs::new(Box::leak(path.into_boxed_str()));
        let context = Postgres::test_context(&args)
            .await
            .unwrap_or_else(|err| panic!("could not create test database: {err:#?}"));

        Self { context }
    }

    pub async fn connection(&self) -> PgConnection {
        let mut conn = self
            .context
            .connect_opts
            .connect()
            .await
            .unwrap_or_else(|err| panic!("failed to connect to test database: {err:#?}"));

        // Create pg_bm25 extension
        sqlx::query(DB_SETUP_SQL)
            .execute(&mut conn)
            .await
            .unwrap_or_else(|err| panic!("could not create extension pg_bm25: {err:#?}"));

        conn
    }
}

impl Drop for Db {
    fn drop(&mut self) {
        let db_name = self.context.db_name.to_string();
        async_std::task::spawn(async move {
            Postgres::cleanup_test(db_name.as_str()).await.unwrap();
        });
    }
}

static DB_SETUP_SQL: &str = r#"
    CREATE EXTENSION pg_analytics;
"#;
