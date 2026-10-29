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
use rstest::rstest;
use sqlx::Connection;
use std::time::Duration;
use tests::fixtures::*;

#[rstest]
#[async_std::test]
async fn direct_metadata_locks_heap_before_index(
    database: Db,
    #[values("pdb.index_segments", "paradedb.fsm_info", "paradedb.fsm_size")] function: &str,
) -> Result<()> {
    let mut writer = database.connection().await;
    "CREATE EXTENSION pg_search CASCADE;
     CREATE TABLE direct_lock_docs(id integer);
     INSERT INTO direct_lock_docs VALUES (1);
     CREATE INDEX direct_lock_idx ON direct_lock_docs USING paradedb(id);"
        .execute(&mut writer);
    let mut reader = database.connection().await;
    let mut observer = database.connection().await;
    "SET statement_timeout = '15s'".execute(&mut writer);
    "SET statement_timeout = '15s'".execute(&mut reader);
    let (reader_pid,): (i32,) = "SELECT pg_backend_pid()".fetch_one(&mut reader);
    let mut transaction = writer.begin().await?;
    sqlx::query("LOCK TABLE direct_lock_docs IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *transaction)
        .await?;
    let query = format!("SELECT count(*) FROM {function}('direct_lock_idx')");
    let pending = async_std::task::spawn(async move {
        sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
            .execute(&mut reader)
            .await
    });

    // Observe the actual heap wait before issuing DDL. Merely starting the
    // reader or sleeping would allow an index-first implementation to pass.
    async_std::future::timeout(Duration::from_secs(10), async {
        loop {
            let waiting: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT FROM pg_locks
                 WHERE pid = $1 AND relation = 'direct_lock_docs'::regclass
                   AND mode = 'AccessShareLock' AND NOT granted)",
            )
            .bind(reader_pid)
            .fetch_one(&mut observer)
            .await?;
            if waiting {
                return Ok::<_, sqlx::Error>(());
            }
            async_std::task::sleep(Duration::from_millis(10)).await;
        }
    })
    .await??;
    let index_locked_before_heap: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT FROM pg_locks
         WHERE pid = $1 AND relation = 'direct_lock_idx'::regclass AND granted)",
    )
    .bind(reader_pid)
    .fetch_one(&mut observer)
    .await?;
    let dropped = sqlx::query("DROP INDEX direct_lock_idx")
        .execute(&mut *transaction)
        .await;
    if dropped.is_ok() {
        transaction.commit().await?;
    } else {
        transaction.rollback().await?;
    }
    let result = pending.await;
    assert!(
        !index_locked_before_heap,
        "{function} locked the index before obtaining the heap lock"
    );
    dropped?;
    // DROP won the race while the reader waited safely on the table. The
    // reader must reject the missing index rather than read stale storage.
    let error = result.expect_err("the index was dropped while the reader waited");
    let database_error = error
        .as_database_error()
        .expect("PostgreSQL must report the dropped relation");
    let code = database_error.code();
    assert!(
        matches!(code.as_deref(), Some("42P01" | "XX000")),
        "{error}"
    );
    // relation_open reports XX000 if the catalog entry disappears after its
    // lock is granted; an earlier missing-relation check reports 42P01.
    assert!(
        database_error
            .message()
            .contains("could not open relation with OID")
            || database_error.message().contains("does not exist"),
        "{error}"
    );
    Ok(())
}
