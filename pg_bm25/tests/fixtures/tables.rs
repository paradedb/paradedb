use async_trait::async_trait;
use sqlx::{Executor, FromRow, PgConnection, Postgres};

#[async_trait]
pub trait Table<T>
where
    T: for<'r> FromRow<'r, <Postgres as sqlx::Database>::Row> + Send + Unpin + 'static,
{
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

#[derive(Debug, PartialEq, FromRow)]
pub struct TestTable {
    pub id: i32,
    pub description: String,
    pub category: String,
    pub rating: i32,
    pub in_stock: bool,
    pub metadata: serde_json::Value,
}

impl TestTable {
    pub async fn setup(conn: &mut PgConnection) {
        conn.execute(include_str!("sql/create_bm25_test_table_default.sql"))
            .await
            .expect("could not setup TestTable");
    }

    pub async fn setup_no_index(conn: &mut PgConnection) {
        conn.execute(include_str!("sql/create_bm25_test_table.sql"))
            .await
            .expect("could not setup TestTable");
    }
}

impl Table<TestTable> for TestTable {}
