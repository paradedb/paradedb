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
use sqlx::{PgConnection, Postgres, Transaction};
use std::time::Duration;

/// Wait until the reader is blocked on `locked_heap`, then return whether it
/// holds a granted lock on any index of `indexed_heap`.
/// The heaps may differ when testing release of a previously visited sibling.
pub async fn index_locked_while_waiting_for_heap(
    observer: &mut PgConnection,
    reader_pid: i32,
    locked_heap: &str,
    indexed_heap: &str,
) -> Result<bool> {
    // Starting the reader or sleeping alone would let an index-first path pass.
    async_std::future::timeout(Duration::from_secs(10), async {
        loop {
            let waiting: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT FROM pg_locks
                 WHERE pid = $1 AND relation = $2::text::regclass
                   AND mode = 'AccessShareLock' AND NOT granted)",
            )
            .bind(reader_pid)
            .bind(locked_heap)
            .fetch_one(&mut *observer)
            .await?;
            if waiting {
                return Ok::<_, sqlx::Error>(());
            }
            async_std::task::sleep(Duration::from_millis(10)).await;
        }
    })
    .await??;
    Ok(sqlx::query_scalar(
        "SELECT EXISTS (SELECT FROM pg_locks l JOIN pg_index i ON i.indexrelid = l.relation
         WHERE l.pid = $1 AND i.indrelid = $2::text::regclass AND l.granted)",
    )
    .bind(reader_pid)
    .bind(indexed_heap)
    .fetch_one(observer)
    .await?)
}

/// Commit successful DDL or roll back its error before awaiting the blocked reader.
pub async fn execute_ddl_and_finish(
    mut transaction: Transaction<'_, Postgres>,
    statement: &str,
) -> Result<(), sqlx::Error> {
    let result = sqlx::query(sqlx::AssertSqlSafe(statement))
        .execute(&mut *transaction)
        .await;
    if result.is_ok() {
        transaction.commit().await?;
    } else {
        transaction.rollback().await?;
    }
    result.map(|_| ())
}
