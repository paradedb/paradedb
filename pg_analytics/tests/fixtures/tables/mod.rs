mod research_project_arrays;
mod user_session_log;

use std::marker::PhantomData;

use async_std::task::block_on;
use sqlx::{Executor, FromRow, PgConnection, Postgres};

pub use research_project_arrays::*;
pub use user_session_log::*;

/// A consistent interace for setting up a Table.
/// with() should return a SQL string for creating the table and inserting rows.
pub trait Table<T>
where
    T: for<'r> FromRow<'r, <Postgres as sqlx::Database>::Row> + Send + Unpin + 'static,
{
    fn with() -> &'static str;
}

// A generic struct that owns a connection and performs queries for a table.
pub struct TableConnection<T: Table<T>>
where
    T: for<'r> FromRow<'r, <Postgres as sqlx::Database>::Row> + Send + Unpin + 'static,
{
    connection: PgConnection,
    marker: PhantomData<T>,
}

#[allow(dead_code)]
impl<T> TableConnection<T>
where
    T: Table<T> + for<'r> FromRow<'r, <Postgres as sqlx::Database>::Row> + Send + Unpin + 'static,
{
    pub fn new(connection: PgConnection) -> Self {
        Self {
            connection,
            marker: PhantomData,
        }
    }

    pub fn setup_new(connection: PgConnection) -> Self {
        let mut new_self = Self::new(connection);
        new_self.setup();
        new_self
    }

    pub fn setup(&mut self) {
        block_on(async {
            let setup_query = T::with();
            self.connection.execute(setup_query).await.unwrap();
        })
    }

    pub fn execute(&mut self, query_str: &str) {
        block_on(async {
            sqlx::query(query_str)
                .execute(self.connection.as_mut())
                .await
                .unwrap();
        })
    }

    pub fn fetch_all(&mut self, query_str: &str) -> Vec<T> {
        block_on(async {
            sqlx::query_as::<_, T>(query_str)
                .fetch_all(self.connection.as_mut())
                .await
                .unwrap()
        })
    }

    pub fn fetch_collect<B>(&mut self, query_str: &str) -> B
    where
        B: FromIterator<T>,
    {
        self.fetch_all(query_str).into_iter().collect::<B>()
    }
}
