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
use rstest::*;
use sqlx::{AssertSqlSafe, Executor, PgConnection};
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

#[rstest]
#[case::savepoint("SAVEPOINT attempt", "ROLLBACK TO SAVEPOINT attempt", 1)]
#[case::nested(
    "SAVEPOINT outer_attempt; SAVEPOINT attempt",
    "RELEASE SAVEPOINT attempt; ROLLBACK TO SAVEPOINT outer_attempt",
    1
)]
#[case::repeated("SAVEPOINT attempt", "ROLLBACK TO SAVEPOINT attempt", 2)]
#[case::full_transaction("", "ROLLBACK; BEGIN", 1)]
#[tokio::test]
async fn retry_index_after_rollback(
    database: Db,
    #[case] savepoint: &str,
    #[case] rollback: &str,
    #[case] attempts: usize,
) -> Result<()> {
    let mut conn = database.connection().await;
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
            conn.execute(AssertSqlSafe(rollback)).await?;
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
#[tokio::test]
async fn rolled_back_typmod_is_not_loadable(
    database: Db,
    #[case] savepoint: &str,
    #[case] rollback: &str,
) -> Result<()> {
    let mut conn = database.connection().await;
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
        conn.execute(AssertSqlSafe(rollback)).await?;
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
    let mut conn = database.connection().await;
    conn.execute("CREATE EXTENSION IF NOT EXISTS pg_search CASCADE")
        .await?;

    conn.execute(AssertSqlSafe(before_caching)).await?;
    let cached = cache_only(&mut conn, "alias=cached_before").await?;
    conn.execute(AssertSqlSafe(after_caching)).await?;

    conn.execute(AssertSqlSafe(savepoint)).await?;
    // A new entry, so the rollback has something of its own to remove.
    conn.execute("SELECT 'x'::pdb.simple('alias=added_then_rolled_back')")
        .await?;
    conn.execute(AssertSqlSafe(rollback)).await?;

    assert_eq!(lookup(&mut conn, cached).await?, "('alias=cached_before')");
    conn.execute("ROLLBACK").await?;
    Ok(())
}
