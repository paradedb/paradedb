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
use futures::FutureExt;
use rstest::*;
use sqlx::{AssertSqlSafe, Connection, Executor, PgConnection};
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Duration;
use tests::fixtures::*;

async fn option_count(conn: &mut PgConnection, option: &str) -> Result<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM paradedb._typmod_cache WHERE typmod = ARRAY[$1]")
            .bind(option)
            .fetch_one(conn)
            .await?,
    )
}

async fn lookup(conn: &mut PgConnection, id: i32) -> sqlx::Result<String> {
    sqlx::query_scalar("SELECT paradedb.generic_typmod_out($1)::text")
        .bind(id)
        .fetch_one(conn)
        .await
}

/// Saves `option` and caches its id in this backend, then deletes the row, so that only the
/// cache can still resolve the id.
async fn cache_only(conn: &mut PgConnection, option: &str) -> Result<i32> {
    let id: i32 = sqlx::query_scalar("SELECT paradedb._save_typmod(ARRAY[$1])")
        .bind(option)
        .fetch_one(&mut *conn)
        .await?;
    lookup(conn, id).await?;
    sqlx::query("DELETE FROM paradedb._typmod_cache WHERE id = $1")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    Ok(id)
}

async fn assert_index_works(conn: &mut PgConnection) -> Result<()> {
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM typmod_subtransaction WHERE id @@@ pdb.all()")
            .fetch_one(conn)
            .await?;
    assert_eq!(count, 1);
    Ok(())
}

/// `ROLLBACK PREPARED` fires neither the commit nor the abort callbacks of the backend that
/// prepared the transaction, so the prepare itself must forget the entries.
const ROLLBACK_PREPARED: &str = "PREPARE TRANSACTION '{gid}'; ROLLBACK PREPARED '{gid}'; BEGIN";

/// Keep the fixture alive until failure cleanup finishes: `Db::drop` must not try to drop
/// the database while it still has prepared transactions. Register identifiers before
/// PREPARE, since the server can prepare successfully even if the client loses its response.
struct PreparedTransactionCleanup {
    database: Arc<Db>,
    gids: Vec<String>,
}

impl PreparedTransactionCleanup {
    fn new(database: &Arc<Db>) -> Self {
        Self {
            database: Arc::clone(database),
            gids: Vec::new(),
        }
    }

    fn register(&mut self, gid: String) {
        self.gids.push(gid);
    }
}

impl Drop for PreparedTransactionCleanup {
    fn drop(&mut self) {
        if self.gids.is_empty() {
            return;
        }
        let database = Arc::clone(&self.database);
        let gids = std::mem::take(&mut self.gids);
        // Like Db::drop, use the async-std executor so cleanup outlives the test's Tokio runtime.
        async_std::task::spawn(async move {
            let mut conn = database.connection().await;
            for gid in gids {
                let rollback = format!("ROLLBACK PREPARED '{}'", gid.replace('\'', "''"));
                if let Err(error) = conn.execute(AssertSqlSafe(rollback)).await {
                    // Normally the test already rolled it back, or never reached PREPARE.
                    if error.as_database_error().and_then(|e| e.code()).as_deref() != Some("42704")
                    {
                        eprintln!("Failed to clean up prepared transaction {gid}: {error}");
                    }
                }
            }
            // Close the cleanup connection before releasing the last fixture reference.
            conn.close().await.ok();
            drop(database);
        });
    }
}

#[rstest]
#[case::early_return(false)]
#[case::panic(true)]
#[tokio::test]
async fn prepared_transactions_cleaned_up_on_failure(
    database: Db,
    #[case] panic: bool,
) -> Result<()> {
    let database = Arc::new(database);
    let mut observer = database.connection().await;
    if !prepared_transactions_enabled(&mut observer).await? {
        return Ok(());
    }
    let gid = prepared_gid(&mut observer).await?;
    let gids = vec![format!("{gid}_a"), format!("{gid}_b")];
    let result = AssertUnwindSafe(async {
        let mut cleanup = PreparedTransactionCleanup::new(&database);
        // Cleanup must continue when an identifier never reached PREPARE.
        cleanup.register(format!("{gid}_not_prepared"));
        let mut conn = database.connection().await;
        for gid in &gids {
            cleanup.register(gid.clone());
            conn.execute("BEGIN").await?;
            conn.execute(AssertSqlSafe(format!("PREPARE TRANSACTION '{gid}'")))
                .await?;
        }
        let count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM pg_prepared_xacts WHERE gid = ANY($1)")
                .bind(&gids)
                .fetch_one(&mut observer)
                .await?;
        assert_eq!(count, 2);
        if panic {
            panic!("simulated failure after PREPARE");
        }
        Err::<(), anyhow::Error>(anyhow::anyhow!("simulated failure after PREPARE"))
    })
    .catch_unwind()
    .await;

    let cleaned = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let count: i64 =
                sqlx::query_scalar("SELECT count(*) FROM pg_prepared_xacts WHERE gid = ANY($1)")
                    .bind(&gids)
                    .fetch_one(&mut observer)
                    .await?;
            if count == 0 {
                return Ok::<(), sqlx::Error>(());
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await;
    // Even a regression in the guard must not leave this test's prepared transactions behind.
    for gid in &gids {
        observer
            .execute(AssertSqlSafe(format!("ROLLBACK PREPARED '{gid}'")))
            .await
            .ok();
    }
    cleaned.expect("failure cleanup left prepared transactions behind")?;
    if panic {
        let panic = result.expect_err("the simulated panic should have unwound");
        assert_eq!(
            panic.downcast_ref::<&str>(),
            Some(&"simulated failure after PREPARE")
        );
    } else {
        assert_eq!(
            result.unwrap().unwrap_err().to_string(),
            "simulated failure after PREPARE"
        );
    }
    Ok(())
}

/// `PREPARE TRANSACTION` needs `max_prepared_transactions > 0`, which Postgres disables by
/// default. CI enables it, so a local cluster without it skips the prepared cases only.
async fn prepared_transactions_enabled(conn: &mut PgConnection) -> Result<bool> {
    let max_prepared: String = sqlx::query_scalar("SHOW max_prepared_transactions")
        .fetch_one(conn)
        .await?;
    if max_prepared == "0" {
        eprintln!("Skipping test: max_prepared_transactions is 0");
    }
    Ok(max_prepared != "0")
}

/// A prepared transaction identifier is cluster-wide, so the test database's name keeps
/// parallel tests apart.
async fn prepared_gid(conn: &mut PgConnection) -> Result<String> {
    Ok(sqlx::query_scalar("SELECT current_database()")
        .fetch_one(conn)
        .await?)
}

/// Fills in [`ROLLBACK_PREPARED`], or returns `None` when the case must be skipped.
async fn prepared_rollback(
    conn: &mut PgConnection,
    rollback: &str,
    cleanup: &mut PreparedTransactionCleanup,
) -> Result<Option<String>> {
    if !rollback.contains("{gid}") {
        return Ok(Some(rollback.to_string()));
    }
    if !prepared_transactions_enabled(&mut *conn).await? {
        return Ok(None);
    }
    let gid = prepared_gid(conn).await?;
    cleanup.register(gid.clone());
    Ok(Some(rollback.replace("{gid}", &gid)))
}

/// Runs `statements` one at a time: a multi-statement query runs in an implicit transaction
/// block, which `ROLLBACK PREPARED` refuses.
async fn execute_each(conn: &mut PgConnection, statements: &str) -> Result<()> {
    for statement in statements.split(';').map(str::trim) {
        if !statement.is_empty() {
            conn.execute(AssertSqlSafe(statement)).await?;
        }
    }
    Ok(())
}

#[rstest]
#[case::savepoint("SAVEPOINT attempt", "ROLLBACK TO SAVEPOINT attempt", 1)]
#[case::nested(
    "SAVEPOINT outer_attempt; SAVEPOINT attempt",
    "RELEASE SAVEPOINT attempt; ROLLBACK TO SAVEPOINT outer_attempt",
    1
)]
#[case::repeated("SAVEPOINT attempt", "ROLLBACK TO SAVEPOINT attempt", 2)]
#[case::full_transaction("", "ROLLBACK; BEGIN", 1)]
#[case::prepared_transaction("", ROLLBACK_PREPARED, 2)]
#[tokio::test]
async fn retry_index_after_rollback(
    database: Db,
    #[case] savepoint: &str,
    #[case] rollback: &str,
    #[case] attempts: usize,
) -> Result<()> {
    let database = Arc::new(database);
    let mut cleanup = PreparedTransactionCleanup::new(&database);
    let mut conn = database.connection().await;
    let Some(rollback) = prepared_rollback(&mut conn, rollback, &mut cleanup).await? else {
        return Ok(());
    };
    conn.execute(
        "CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;
         CREATE TABLE typmod_subtransaction (id integer PRIMARY KEY, body text);
         INSERT INTO typmod_subtransaction VALUES (1, 'hello world')",
    )
    .await?;

    // A second transaction on the same backend must register its callbacks again.
    for round in 0..2 {
        let option = format!("alias=subtransaction_body_{round}");
        let create_index = format!(
            "CREATE INDEX typmod_subtransaction_idx ON typmod_subtransaction
             USING paradedb (id, (body::pdb.simple('{option}')))"
        );
        assert_eq!(option_count(&mut conn, &option).await?, 0);
        conn.execute("BEGIN").await?;
        for _ in 0..attempts {
            conn.execute(AssertSqlSafe(savepoint)).await?;
            conn.execute(AssertSqlSafe(create_index.as_str())).await?;
            assert_eq!(option_count(&mut conn, &option).await?, 1);
            assert_index_works(&mut conn).await?;
            execute_each(&mut conn, &rollback).await?;
            assert_eq!(option_count(&mut conn, &option).await?, 0);
        }

        conn.execute(AssertSqlSafe(create_index.as_str())).await?;
        conn.execute("COMMIT").await?;
        assert_eq!(option_count(&mut conn, &option).await?, 1);
        assert_index_works(&mut conn).await?;

        // The original backend's caches must not hide missing persistent settings.
        let mut fresh = database.connection().await;
        assert_index_works(&mut fresh).await?;
        conn.execute("DROP INDEX typmod_subtransaction_idx").await?;
    }
    Ok(())
}

/// A PL/pgSQL `EXCEPTION` block is the subtransaction abort most users hit, without ever
/// writing a `SAVEPOINT`.
#[rstest]
#[tokio::test]
async fn retry_index_after_exception_block(database: Db) -> Result<()> {
    let mut conn = database.connection().await;
    conn.execute(
        "CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;
         CREATE TABLE typmod_subtransaction (id integer PRIMARY KEY, body text);
         INSERT INTO typmod_subtransaction VALUES (1, 'hello world')",
    )
    .await?;

    let option = "alias=exception_body";
    conn.execute(AssertSqlSafe(format!(
        "DO $$
         BEGIN
           BEGIN
             EXECUTE $q$CREATE INDEX typmod_subtransaction_idx ON typmod_subtransaction
                      USING paradedb (id, (body::pdb.simple('{option}')))$q$;
             RAISE EXCEPTION 'retry';
           EXCEPTION WHEN OTHERS THEN
             NULL;
           END;
           EXECUTE $q$CREATE INDEX typmod_subtransaction_idx ON typmod_subtransaction
                    USING paradedb (id, (body::pdb.simple('{option}')))$q$;
         END $$"
    )))
    .await?;
    assert_eq!(option_count(&mut conn, option).await?, 1);
    assert_index_works(&mut conn).await?;

    let mut fresh = database.connection().await;
    assert_index_works(&mut fresh).await?;
    Ok(())
}

/// Two prepared transactions in a row, with nothing committed on the connection in between.
/// The commit callbacks pgrx keeps across a prepare never run here, so only the prepare itself
/// can reset the callback registration and let the second transaction register its own.
///
/// The first transaction only saves its settings: an index would keep its table lock while
/// prepared and block the second transaction's index creation.
#[rstest]
#[tokio::test]
async fn retry_index_after_consecutive_prepared_rollbacks(database: Db) -> Result<()> {
    let database = Arc::new(database);
    let mut cleanup = PreparedTransactionCleanup::new(&database);
    let mut conn = database.connection().await;
    if !prepared_transactions_enabled(&mut conn).await? {
        return Ok(());
    }
    conn.execute(
        "CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;
         CREATE TABLE typmod_subtransaction (id integer PRIMARY KEY, body text);
         INSERT INTO typmod_subtransaction VALUES (1, 'hello world')",
    )
    .await?;
    let gid = prepared_gid(&mut conn).await?;
    cleanup.register(format!("{gid}_a"));
    cleanup.register(format!("{gid}_b"));
    let create_index = |option: &str| {
        format!(
            "CREATE INDEX typmod_subtransaction_idx ON typmod_subtransaction
             USING paradedb (id, (body::pdb.simple('{option}')))"
        )
    };

    let option_a = "alias=consecutive_body_a";
    let option_b = "alias=consecutive_body_b";
    conn.execute("BEGIN").await?;
    conn.execute(AssertSqlSafe(format!(
        "SELECT 'x'::pdb.simple('{option_a}')"
    )))
    .await?;
    conn.execute(AssertSqlSafe(format!("PREPARE TRANSACTION '{gid}_a'")))
        .await?;
    conn.execute("BEGIN").await?;
    conn.execute(AssertSqlSafe(create_index(option_b).as_str()))
        .await?;
    conn.execute(AssertSqlSafe(format!("PREPARE TRANSACTION '{gid}_b'")))
        .await?;

    let mut other = database.connection().await;
    for suffix in ["a", "b"] {
        other
            .execute(AssertSqlSafe(format!("ROLLBACK PREPARED '{gid}_{suffix}'")))
            .await?;
    }
    assert_eq!(option_count(&mut conn, option_a).await?, 0);
    assert_eq!(option_count(&mut conn, option_b).await?, 0);

    conn.execute(AssertSqlSafe(create_index(option_b).as_str()))
        .await?;
    assert_eq!(option_count(&mut conn, option_b).await?, 1);
    assert_index_works(&mut conn).await?;

    let mut fresh = database.connection().await;
    assert_index_works(&mut fresh).await?;
    Ok(())
}

/// The savepoint inserts the row itself, so index creation finds it in the table instead of
/// inserting it. The id it caches from there must be forgotten on rollback too.
#[rstest]
#[tokio::test]
async fn retry_index_after_rollback_of_found_row(database: Db) -> Result<()> {
    let mut conn = database.connection().await;
    conn.execute(
        "CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;
         CREATE TABLE typmod_subtransaction (id integer PRIMARY KEY, body text);
         INSERT INTO typmod_subtransaction VALUES (1, 'hello world')",
    )
    .await?;

    let option = "alias=found_row_body";
    let create_index = format!(
        "CREATE INDEX typmod_subtransaction_idx ON typmod_subtransaction
         USING paradedb (id, (body::pdb.simple('{option}')))"
    );
    conn.execute("BEGIN; SAVEPOINT attempt").await?;
    sqlx::query("SELECT paradedb._save_typmod(ARRAY[$1])")
        .bind(option)
        .execute(&mut conn)
        .await?;
    conn.execute(AssertSqlSafe(create_index.as_str())).await?;
    conn.execute("ROLLBACK TO SAVEPOINT attempt").await?;

    conn.execute(AssertSqlSafe(create_index.as_str())).await?;
    conn.execute("COMMIT").await?;
    assert_eq!(option_count(&mut conn, option).await?, 1);

    let mut fresh = database.connection().await;
    assert_index_works(&mut fresh).await?;
    Ok(())
}

#[rstest]
#[case::savepoint("SAVEPOINT attempt", "ROLLBACK TO SAVEPOINT attempt")]
#[case::nested(
    "SAVEPOINT outer_attempt; SAVEPOINT attempt",
    "RELEASE SAVEPOINT attempt; ROLLBACK TO SAVEPOINT outer_attempt"
)]
#[case::prepared_transaction("", ROLLBACK_PREPARED)]
#[tokio::test]
async fn rolled_back_typmod_is_not_loadable(
    database: Db,
    #[case] savepoint: &str,
    #[case] rollback: &str,
) -> Result<()> {
    let database = Arc::new(database);
    let mut cleanup = PreparedTransactionCleanup::new(&database);
    let mut conn = database.connection().await;
    let Some(rollback) = prepared_rollback(&mut conn, rollback, &mut cleanup).await? else {
        return Ok(());
    };
    conn.execute("CREATE EXTENSION IF NOT EXISTS pg_search CASCADE")
        .await?;

    // Also check callback registration after the previous transaction aborts.
    for round in 0..2 {
        let option = format!("alias=rolled_back_body_{round}");
        conn.execute("BEGIN").await?;
        conn.execute(AssertSqlSafe(savepoint)).await?;
        // Insert directly so the first cache access goes through load_typmod, without
        // save_typmod having registered the transaction callbacks for it.
        let id: i32 = sqlx::query_scalar("SELECT paradedb._save_typmod(ARRAY[$1])")
            .bind(&option)
            .fetch_one(&mut conn)
            .await?;
        lookup(&mut conn, id).await?;
        execute_each(&mut conn, &rollback).await?;
        assert_eq!(option_count(&mut conn, &option).await?, 0);

        // Test the load cache independently: retrying index creation only proves that
        // the save cache forgot the old ID, since the retry can allocate a new one.
        let error = lookup(&mut conn, id)
            .await
            .expect_err("a rolled-back typmod must not remain in the load cache");
        assert_eq!(
            error.as_database_error().unwrap().message(),
            "stored tokenizer options could not be found"
        );
        conn.execute("ROLLBACK").await?;
    }
    Ok(())
}

/// A rollback removes only the entries it added, so settings cached before it stay cached:
/// in an earlier transaction, earlier in the same transaction, or in a savepoint released
/// before a sibling savepoint rolls back.
#[rstest]
#[case::earlier_transaction_savepoint(
    "",
    "BEGIN",
    "SAVEPOINT attempt",
    "ROLLBACK TO SAVEPOINT attempt"
)]
#[case::earlier_transaction_full("", "BEGIN", "", "ROLLBACK; BEGIN")]
#[case::earlier_transaction_prepared("", "BEGIN", "", ROLLBACK_PREPARED)]
#[case::same_transaction("BEGIN", "", "SAVEPOINT attempt", "ROLLBACK TO SAVEPOINT attempt")]
#[case::released_sibling(
    "BEGIN; SAVEPOINT released",
    "RELEASE SAVEPOINT released",
    "SAVEPOINT attempt",
    "ROLLBACK TO SAVEPOINT attempt"
)]
#[tokio::test]
async fn rollback_keeps_entries_cached_before_it(
    database: Db,
    #[case] before_caching: &str,
    #[case] after_caching: &str,
    #[case] savepoint: &str,
    #[case] rollback: &str,
) -> Result<()> {
    let database = Arc::new(database);
    let mut cleanup = PreparedTransactionCleanup::new(&database);
    let mut conn = database.connection().await;
    let Some(rollback) = prepared_rollback(&mut conn, rollback, &mut cleanup).await? else {
        return Ok(());
    };
    conn.execute("CREATE EXTENSION IF NOT EXISTS pg_search CASCADE")
        .await?;

    conn.execute(AssertSqlSafe(before_caching)).await?;
    let cached = cache_only(&mut conn, "alias=cached_before").await?;
    conn.execute(AssertSqlSafe(after_caching)).await?;

    conn.execute(AssertSqlSafe(savepoint)).await?;
    // A new entry, so the rollback has something of its own to remove.
    conn.execute("SELECT 'x'::pdb.simple('alias=added_then_rolled_back')")
        .await?;
    execute_each(&mut conn, &rollback).await?;

    assert_eq!(lookup(&mut conn, cached).await?, "('alias=cached_before')");
    conn.execute("ROLLBACK").await?;
    Ok(())
}
