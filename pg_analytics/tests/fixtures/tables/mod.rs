mod user_session_log;

use async_trait::async_trait;
use sqlx::{Executor, FromRow, PgConnection, Postgres};

pub use user_session_log::*;

#[async_trait]
pub trait Table<T>
where
    T: for<'r> FromRow<'r, <Postgres as sqlx::Database>::Row> + Send + Unpin + 'static,
{
    fn with() -> &'static str;

    async fn setup(conn: &mut PgConnection) -> Result<(), sqlx::Error> {
        let setup_query = Self::with();
        conn.execute(setup_query).await.unwrap();
        Ok(())
    }

    async fn execute(conn: &mut PgConnection, query_str: &str) -> Result<(), sqlx::Error> {
        sqlx::query(query_str).execute(conn).await.unwrap();
        Ok(())
    }

    async fn fetch_all(conn: &mut PgConnection, query_str: &str) -> Result<Vec<T>, sqlx::Error> {
        Ok(sqlx::query_as::<_, T>(query_str)
            .fetch_all(conn)
            .await
            .unwrap())
    }
}
