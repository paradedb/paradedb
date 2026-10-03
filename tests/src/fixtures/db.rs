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

use anyhow::Context;
use async_std::prelude::Stream;
use async_std::stream::StreamExt;
use async_std::task::block_on;
use bytes::Bytes;
use rand::RngExt;
use sqlx::Error;
use sqlx::{
    AssertSqlSafe, ConnectOptions, Connection, Decode, Executor, FromRow, PgConnection, Postgres,
    Type,
    postgres::PgRow,
    testing::{TestArgs, TestContext, TestSupport},
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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

/// Dedicated session-level advisory lock key for test database cleanup serialization.
/// This key is derived from a hash of "pg_test_cln" and does not collide with pg_search merge locks
/// which use 0x5047534D ("PGSM").
/// Value: first 8 bytes of SHA256("pg_test_cleanup") = 0x70675F746573745F -> truncated to i64
const CLEANUP_ADVISORY_LOCK_KEY: i64 = 0x70675F746573745F;

/// Maximum time to wait for target database sessions to terminate after sending
/// pg_terminate_backend.
const SESSION_TERMINATION_TIMEOUT: Duration = Duration::from_secs(30);

/// Polling interval while waiting for sessions to disappear.
const SESSION_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Performs cleanup of a single test database.
///
/// This function runs in a dedicated thread with an isolated Tokio runtime.
/// It establishes its own PostgreSQL connection (not from the pool) to ensure
/// the session-level advisory lock is held for the entire cleanup sequence.
async fn cleanup_test_database(db_name: String) -> Result<(), Error> {
    eprintln!("test db cleanup: START for {:?}", db_name);
    // Get the master DATABASE_URL to connect to the postgres database
    let url = std::env::var("DATABASE_URL").map_err(|_| {
        Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "DATABASE_URL not set",
        ))
    })?;

    // Connect directly to the postgres database (not the test database)
    // We need a connection that is NOT to the target database so we can DROP it.
    let mut conn = PgConnection::connect(&url).await?;

    // Acquire session-level advisory lock to serialize cleanup across all test binaries.
    // This lock is held for the duration of this connection.
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(CLEANUP_ADVISORY_LOCK_KEY)
        .execute(&mut conn)
        .await?;

    // Get our own backend PID to avoid terminating ourselves
    let our_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut conn)
        .await?;

    // Terminate all sessions connected to the target database (except our own connection).
    let pids: Vec<i32> =
        sqlx::query_scalar("SELECT pid FROM pg_stat_activity WHERE datname = $1 AND pid <> $2")
            .bind(&db_name)
            .bind(our_pid)
            .fetch_all(&mut conn)
            .await?;

    if !pids.is_empty() {
        eprintln!(
            "test db cleanup: terminating {} sessions for database {:?}",
            pids.len(),
            db_name
        );
        for pid in &pids {
            // Best effort: pg_terminate_backend returns false if the backend already exited
            let _ = sqlx::query("SELECT pg_terminate_backend($1)")
                .bind(*pid)
                .execute(&mut conn)
                .await;
        }

        // Wait for all sessions on the target database to disappear.
        let deadline = Instant::now() + SESSION_TERMINATION_TIMEOUT;
        loop {
            let count: i64 =
                sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE datname = $1")
                    .bind(&db_name)
                    .fetch_one(&mut conn)
                    .await?;

            if count == 0 {
                break;
            }

            if Instant::now() > deadline {
                eprintln!(
                    "test db cleanup: timeout waiting for sessions to terminate on {:?}; {} sessions remain",
                    db_name, count
                );
                // Continue anyway - DROP DATABASE might still succeed or fail with a clear error
                break;
            }

            tokio::time::sleep(SESSION_POLL_INTERVAL).await;
        }
    } else {
        eprintln!(
            "test db cleanup: no sessions to terminate for {:?}",
            db_name
        );
    }

    // DROP DATABASE must run outside an explicit transaction block.
    // The db_name is generated by sqlx as "_sqlx_test_" + urlsafe_base64
    // (alphanumeric + underscore), so interpolating it is free of SQL
    // injection risk. It MUST be double-quoted: the base64 alphabet contains
    // uppercase letters and an unquoted identifier would be folded to
    // lowercase by PostgreSQL, matching nothing while IF EXISTS masks the
    // no-op as success (this is exactly how orphaned databases with missing
    // tracking rows were produced). This mirrors sqlx's own
    // `drop database if exists {db_name:?}` / `create database {db_name:?}`.
    let drop_sql: String = format!("DROP DATABASE IF EXISTS {db_name:?}");
    match sqlx::query(AssertSqlSafe(drop_sql))
        .execute(&mut conn)
        .await
    {
        Ok(_) => {
            eprintln!("test db cleanup: dropped database {:?}", db_name);
        }
        Err(e) => {
            eprintln!(
                "test db cleanup: failed to drop database {:?}: {e:#}",
                db_name
            );
            // Explicitly release the advisory lock before returning the error
            let _ = sqlx::query("SELECT pg_advisory_unlock($1)")
                .bind(CLEANUP_ADVISORY_LOCK_KEY)
                .execute(&mut conn)
                .await;
            return Err(e);
        }
    }

    // Remove from the sqlx tracking table. This runs only after DROP DATABASE
    // succeeded, and cleanup is reported successful only if this DELETE also
    // succeeds: a failed DELETE leaves a stale tracking row that must stay
    // visible for investigation, never be silently discarded.
    if let Err(e) = sqlx::query("DELETE FROM _sqlx_test.databases WHERE db_name = $1")
        .bind(&db_name)
        .execute(&mut conn)
        .await
    {
        eprintln!(
            "test db cleanup: failed to delete tracking row for {:?}: {e:#}",
            db_name
        );
        // Explicitly release the advisory lock before returning the error
        let _ = sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(CLEANUP_ADVISORY_LOCK_KEY)
            .execute(&mut conn)
            .await;
        return Err(e);
    }

    // Explicitly release the advisory lock before closing the connection.
    // This is safer than relying on connection close to release it.
    let _ = sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(CLEANUP_ADVISORY_LOCK_KEY)
        .execute(&mut conn)
        .await;

    // Close the cleanup connection
    let _ = conn.close().await;

    eprintln!("test db cleanup: END for {:?}", db_name);
    Ok(())
}

impl Drop for Db {
    fn drop(&mut self) {
        let db_name = self.context.db_name.clone();
        eprintln!("test db cleanup: Db::drop() called for {:?}", db_name);

        // Run cleanup in a dedicated thread with an isolated Tokio runtime.
        // This avoids depending on the async runtime of #[async_std::test] or #[tokio::test],
        // which may already be shutting down when Drop runs.
        let handle = std::thread::spawn(move || -> Result<(), Error> {
            // Create a fresh current-thread Tokio runtime for this cleanup.
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("test db cleanup: failed to create Tokio runtime: {e:#}");
                    return Err(Error::Io(std::io::Error::other(format!(
                        "runtime build failed: {e}"
                    ))));
                }
            };

            rt.block_on(async { cleanup_test_database(db_name).await })
        });

        // Wait for cleanup to complete. This ensures Db::drop() does not return
        // while cleanup is still pending.
        match handle.join() {
            Ok(Ok(())) => {
                // Cleanup succeeded: DROP DATABASE and the tracking-row DELETE
                // both confirmed. Nothing further to do.
            }
            Ok(Err(e)) => {
                // Cleanup failed: the database and/or its tracking row survive
                // for investigation. Do not claim success.
                eprintln!(
                    "test db cleanup: failed for {:?}: {e:#}",
                    self.context.db_name
                );
            }
            Err(e) => {
                // Thread panicked: same as above, cleanup did not complete.
                eprintln!(
                    "test db cleanup: thread panicked for {:?}: {e:?}",
                    self.context.db_name
                );
            }
        }
        eprintln!(
            "test db cleanup: Db::drop() finished for {:?}",
            self.context.db_name
        );
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
