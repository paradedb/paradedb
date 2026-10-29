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
#[case::plain("plain")]
#[case::parent("parent")]
#[case::nested_parent("nested")]
#[case::direct_leaf("leaf")]
#[case::all_leaves("all_leaves")]
#[async_std::test]
async fn verification_locks_heap_before_index(database: Db, #[case] target: &str) -> Result<()> {
    let mut writer = database.connection().await;
    "CREATE EXTENSION IF NOT EXISTS pg_search CASCADE".execute(&mut writer);
    let (heap, verification, expected_heap_checks) = if target == "plain" {
        "CREATE TABLE lock_plain(id integer);
         INSERT INTO lock_plain VALUES (1);
         CREATE INDEX lock_plain_idx ON lock_plain USING paradedb(id);"
            .execute(&mut writer);
        (
            "lock_plain",
            "pdb.verify_index('lock_plain_idx', heapallindexed => true)".to_string(),
            1,
        )
    } else {
        "CREATE TABLE lock_parent(id integer) PARTITION BY RANGE(id)".execute(&mut writer);
        let leaf_parent = if target == "nested" {
            "CREATE TABLE lock_intermediate PARTITION OF lock_parent
             FOR VALUES FROM (0) TO (20) PARTITION BY RANGE(id)"
                .execute(&mut writer);
            "lock_intermediate"
        } else {
            "lock_parent"
        };
        format!(
            "CREATE TABLE lock_leaf PARTITION OF {leaf_parent} FOR VALUES FROM (0) TO (10);
             CREATE TABLE lock_other PARTITION OF {leaf_parent} FOR VALUES FROM (10) TO (20);
             INSERT INTO lock_parent VALUES (1), (11);
             CREATE INDEX lock_parent_idx ON lock_parent USING paradedb(id);"
        )
        .execute(&mut writer);
        let (leaf_index,): (String,) = "SELECT indexrelid::regclass::text FROM pg_index
            WHERE indrelid = 'lock_leaf'::regclass"
            .fetch_one(&mut writer);
        let (verification, count) = match target {
            "parent" | "nested" => (
                "pdb.verify_index('lock_parent_idx', heapallindexed => true)".to_string(),
                2,
            ),
            "leaf" => (
                format!("pdb.verify_index('{leaf_index}', heapallindexed => true)"),
                1,
            ),
            "all_leaves" => (
                "pdb.verify_all_indexes(index_pattern => 'lock_leaf%', heapallindexed => true)"
                    .to_string(),
                1,
            ),
            _ => panic!("unknown verification test case: {target}"),
        };
        ("lock_leaf", verification, count)
    };
    let mut reader = database.connection().await;
    "SET statement_timeout = '15s'".execute(&mut writer);
    "SET statement_timeout = '15s'".execute(&mut reader);
    let (reader_pid,): (i32,) = "SELECT pg_backend_pid()".fetch_one(&mut reader);
    let mut observer = database.connection().await;

    let mut transaction = writer.begin().await?;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "LOCK TABLE {heap} IN ACCESS EXCLUSIVE MODE"
    )))
    .execute(&mut *transaction)
    .await?;
    let query = format!(
        "SELECT bool_and(passed),
        count(*) FILTER (WHERE check_name LIKE '%: heap_references_valid') FROM {verification}"
    );
    let pending = async_std::task::spawn(async move {
        sqlx::query_as::<_, (bool, i64)>(sqlx::AssertSqlSafe(query.as_str()))
            .fetch_one(&mut reader)
            .await
    });

    // Synchronize on the real heap-lock wait, not a sleep or the reader starting.
    async_std::future::timeout(Duration::from_secs(10), async {
        loop {
            let waiting: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT FROM pg_locks
                 WHERE pid = $1 AND relation = $2::text::regclass
                   AND mode = 'AccessShareLock' AND NOT granted)",
            )
            .bind(reader_pid)
            .bind(heap)
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
        "SELECT EXISTS (SELECT FROM pg_locks l JOIN pg_index i ON i.indexrelid = l.relation
         WHERE l.pid = $1 AND i.indrelid = $2::text::regclass AND l.granted)",
    )
    .bind(reader_pid)
    .bind(heap)
    .fetch_one(&mut observer)
    .await?;

    // REINDEX needs the leaf-index lock. Acquiring it before the heap in the
    // reader would make these sessions wait on each other.
    let reindex = sqlx::query(sqlx::AssertSqlSafe(format!("REINDEX TABLE {heap}")))
        .execute(&mut *transaction)
        .await;
    if reindex.is_ok() {
        transaction.commit().await?;
    } else {
        transaction.rollback().await?;
    }
    let verified = pending.await;
    assert!(
        !index_locked_before_heap,
        "the verifier locked the leaf index before obtaining the leaf heap lock"
    );
    reindex?;
    assert_eq!(verified?, (true, expected_heap_checks));
    Ok(())
}
