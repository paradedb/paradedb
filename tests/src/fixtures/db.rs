// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use anyhow::Result;
use async_std::prelude::Stream;
use async_std::stream::StreamExt;
use async_std::task::block_on;
use bytes::Bytes;
use rand::RngExt;
use sqlx::{
    AssertSqlSafe, ConnectOptions, Connection, Decode, Error, Executor, FromRow, PgConnection,
    Postgres, Type,
    postgres::PgRow,
    testing::{TestArgs, TestContext, TestSupport},
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn lost_connection(e: &Error) -> bool {
    use crate::fixtures::fault_grace::{TransientKind, classify_transient};
    classify_transient(e) == Some(TransientKind::ConnectionLost)
}

/// Awaits `$op`, retrying lost connections per [`crate::fixtures::fault_grace::FaultRetry`].
/// Panics on any other error. The async twin of `fault_grace::retry_transient`, without a pool.
macro_rules! tolerate_transient_setup {
    ($what:literal, $op:expr) => {{
        let mut retry = crate::fixtures::fault_grace::FaultRetry::default();
        loop {
            match $op.await {
                Ok(value) => break value,
                Err(err) => {
                    if !lost_connection(&err) {
                        panic!(concat!($what, ": {:#?}"), err);
                    }
                    if let Err(reason) = retry.record($what, &err) {
                        panic!("{}", reason);
                    }
                }
            }
        }
    }};
}

pub struct Db {
    context: TestContext<Postgres>,
}

impl Db {
    pub async fn new() -> Self {
        let path =
            // timestamp
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("current time should be retrievable")
                .as_micros()
                .to_string()

                // plus the current thread name, which is typically going to be the test name
                + &std::thread::current()
                .name()
                .map(String::from)
                .unwrap_or_else(|| {
                    // or a random 7-letter "word"
                    rand::rng()
                        .sample_iter(&rand::distr::Alphanumeric)
                        .take(7)
                        .map(char::from)
                        .collect()
                });

        let args = TestArgs::new(Box::leak(path.into_boxed_str()));
        let context = tolerate_transient_setup!(
            "could not create test database",
            Postgres::test_context(&args)
        );

        Self { context }
    }

    pub async fn connection(&self) -> PgConnection {
        tolerate_transient_setup!(
            "failed to connect to test database",
            self.context.connect_opts.connect()
        )
    }
}

impl Drop for Db {
    fn drop(&mut self) {
        let db_name = self.context.db_name.to_string();
        async_std::task::spawn(async move {
            Postgres::cleanup_test(db_name.as_str()).await.ok(); // ignore errors as there's nothing we can do about it
        });
    }
}

pub trait ConnExt {
    fn deallocate_all(&mut self) -> Result<(), sqlx::Error>;
}

impl ConnExt for PgConnection {
    /// Deallocate all cached prepared statements.  Akin to Postgres' `DEALLOCATE ALL` command
    /// but also does the right thing for the sql [`PgConnection`] internals.
    fn deallocate_all(&mut self) -> Result<(), Error> {
        async_std::task::block_on(async { self.clear_cached_statements().await })
    }
}

#[allow(dead_code)]
pub trait Query
where
    Self: AsRef<str> + Sized,
{
    fn execute(self, connection: &mut PgConnection) {
        block_on(async { self.execute_async(connection).await })
    }

    #[allow(async_fn_in_trait)]
    async fn execute_async(self, connection: &mut PgConnection) {
        connection
            .execute(AssertSqlSafe(self.as_ref()))
            .await
            .expect("query execution should succeed");
    }

    fn execute_result(self, connection: &mut PgConnection) -> Result<(), sqlx::Error> {
        block_on(async { connection.execute(AssertSqlSafe(self.as_ref())).await })?;
        Ok(())
    }

    fn fetch<T>(self, connection: &mut PgConnection) -> Vec<T>
    where
        T: for<'r> FromRow<'r, <Postgres as sqlx::Database>::Row> + Send + Unpin,
    {
        block_on(async {
            sqlx::query_as::<_, T>(AssertSqlSafe(self.as_ref()))
                .fetch_all(connection)
                .await
                .unwrap_or_else(|e| panic!("{e}:  error in query '{}'", self.as_ref()))
        })
    }

    fn fetch_retry<T>(
        self,
        connection: &mut PgConnection,
        retries: u32,
        delay_ms: u64,
        validate: fn(&[T]) -> bool,
    ) -> Vec<T>
    where
        T: for<'r> FromRow<'r, <Postgres as sqlx::Database>::Row> + Send + Unpin,
    {
        for attempt in 0..retries {
            match block_on(async {
                sqlx::query_as::<_, T>(AssertSqlSafe(self.as_ref()))
                    .fetch_all(&mut *connection)
                    .await
                    .map_err(anyhow::Error::from)
            }) {
                Ok(result) => {
                    if validate(&result) {
                        return result;
                    } else if attempt < retries - 1 {
                        block_on(async_std::task::sleep(Duration::from_millis(delay_ms)));
                    }
                }
                Err(_) if attempt < retries - 1 => {
                    block_on(async_std::task::sleep(Duration::from_millis(delay_ms)));
                }
                Err(e) => panic!("Fetch attempt {}/{} failed: {}", attempt + 1, retries, e),
            }
        }
        panic!("Exhausted retries for query '{}'", self.as_ref());
    }

    fn fetch_dynamic(self, connection: &mut PgConnection) -> Vec<PgRow> {
        block_on(async {
            sqlx::query(AssertSqlSafe(self.as_ref()))
                .fetch_all(connection)
                .await
                .unwrap_or_else(|e| panic!("{e}:  error in query '{}'", self.as_ref()))
        })
    }

    /// Like [`Query::fetch_dynamic`], but surfaces the `sqlx::Error` instead of panicking.
    fn fetch_dynamic_result(
        self,
        connection: &mut PgConnection,
    ) -> Result<Vec<PgRow>, sqlx::Error> {
        block_on(async {
            sqlx::query(AssertSqlSafe(self.as_ref()))
                .fetch_all(connection)
                .await
        })
    }

    fn fetch_scalar<T>(self, connection: &mut PgConnection) -> Vec<T>
    where
        T: Type<Postgres> + for<'a> Decode<'a, sqlx::Postgres> + Send + Unpin,
    {
        block_on(async {
            sqlx::query_scalar(AssertSqlSafe(self.as_ref()))
                .fetch_all(connection)
                .await
                .unwrap_or_else(|e| panic!("{e}:  error in query '{}'", self.as_ref()))
        })
    }

    fn fetch_one<T>(self, connection: &mut PgConnection) -> T
    where
        T: for<'r> FromRow<'r, <Postgres as sqlx::Database>::Row> + Send + Unpin,
    {
        block_on(async {
            sqlx::query_as::<_, T>(AssertSqlSafe(self.as_ref()))
                .fetch_one(connection)
                .await
                .unwrap_or_else(|e| panic!("{e}:  error in query '{}'", self.as_ref()))
        })
    }

    /// Like [`Query::fetch_one`], but surfaces the `sqlx::Error` instead of panicking.
    fn fetch_one_result<T>(self, connection: &mut PgConnection) -> Result<T, sqlx::Error>
    where
        T: for<'r> FromRow<'r, <Postgres as sqlx::Database>::Row> + Send + Unpin,
    {
        block_on(async {
            sqlx::query_as::<_, T>(AssertSqlSafe(self.as_ref()))
                .fetch_one(connection)
                .await
        })
    }

    fn fetch_result<T>(self, connection: &mut PgConnection) -> Result<Vec<T>, sqlx::Error>
    where
        T: for<'r> FromRow<'r, <Postgres as sqlx::Database>::Row> + Send + Unpin,
    {
        block_on(async {
            sqlx::query_as::<_, T>(AssertSqlSafe(self.as_ref()))
                .fetch_all(connection)
                .await
        })
    }

    fn fetch_collect<T, B>(self, connection: &mut PgConnection) -> B
    where
        T: for<'r> FromRow<'r, <Postgres as sqlx::Database>::Row> + Send + Unpin,
        B: FromIterator<T>,
    {
        self.fetch(connection).into_iter().collect::<B>()
    }
}

impl Query for String {}
impl Query for &String {}
impl Query for &str {}

pub trait DisplayAsync: Stream<Item = Result<Bytes, sqlx::Error>> + Sized {
    fn to_csv(self) -> String {
        let mut csv_str = String::new();
        let mut stream = Box::pin(self);

        while let Some(chunk) = block_on(stream.as_mut().next()) {
            let chunk = chunk.expect("chunk should be valid for DisplayAsync");
            csv_str.push_str(&String::from_utf8_lossy(&chunk));
        }

        csv_str
    }
}

impl<T> DisplayAsync for T where T: Stream<Item = Result<Bytes, sqlx::Error>> + Send + Sized {}
