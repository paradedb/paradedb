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
use sqlx::{AssertSqlSafe, Connection};
use std::time::Duration;
use tests::fixtures::*;

#[rstest]
#[case::unchanged(("note = 'after'", true))]
#[case::changed(("body = 'beta', note = 'after'", false))]
#[case::null(("body = NULL, note = 'after'", false))]
#[async_std::test]
async fn recheck_replacement_row(
    database: Db,
    #[case] concurrent_update: (&str, bool),
    #[values(false, true)] partial: bool,
    #[values(false, true)] update_query: bool,
    #[values(false, true)] index_note: bool,
    #[values(false, true)] nullable_anchor: bool,
    #[values("body === 'alpha'", "NOT (body === 'beta')")] predicate: &str,
) -> Result<()> {
    let (update, matches) = concurrent_update;
    let mut writer = database.connection().await;
    "CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;
     CREATE TABLE epq_rows (id int PRIMARY KEY, body text, note text, anchor text);
     INSERT INTO epq_rows VALUES (1, 'alpha', 'before', NULL);"
        .execute(&mut writer);
    format!(
        "CREATE INDEX epq_rows_idx ON epq_rows USING paradedb ({}id, (body::pdb.literal){}) {}",
        if nullable_anchor { "anchor, " } else { "" },
        if index_note {
            ", (note::pdb.literal)"
        } else {
            ""
        },
        if partial { "WHERE id > 0" } else { "" },
    )
    .execute(&mut writer);

    let mut reader = database.connection().await;
    "SET paradedb.enable_custom_scan = off;
     SET enable_indexscan = off;
     SET enable_indexonlyscan = off;
     SET enable_bitmapscan = off;
     SET statement_timeout = '30s';"
        .execute(&mut reader);
    let (reader_pid,): (i32,) = "SELECT pg_backend_pid()".fetch_one(&mut reader);
    let query = if update_query {
        format!(
            "UPDATE epq_rows SET note = note || '_reader' WHERE id > 0 AND {predicate}
             RETURNING id, body, note"
        )
    } else {
        format!("SELECT id, body, note FROM epq_rows WHERE id > 0 AND {predicate} FOR UPDATE")
    };
    let plan: Vec<(String,)> = format!("EXPLAIN (COSTS OFF) {query}").fetch(&mut reader);
    assert!(
        plan.iter().any(|(line,)| line.contains("Seq Scan")),
        "{plan:?}"
    );

    let mut transaction = writer.begin().await?;
    sqlx::query(AssertSqlSafe(format!(
        "UPDATE epq_rows SET {update} WHERE id = 1"
    )))
    .execute(&mut *transaction)
    .await?;
    if matches {
        let hot_updates: i64 =
            sqlx::query_scalar("SELECT pg_stat_get_xact_tuples_hot_updated('epq_rows'::regclass)")
                .fetch_one(&mut *transaction)
                .await?;
        assert_eq!(hot_updates, i64::from(!index_note));
    }
    let pending = async_std::task::spawn(async move {
        sqlx::query_as::<_, (i32, String, String)>(AssertSqlSafe(query))
            .fetch_all(&mut reader)
            .await
    });

    // Wait for the actual row-lock conflict so the reader's snapshot predates the commit.
    let mut observer = database.connection().await;
    async_std::future::timeout(Duration::from_secs(20), async {
        loop {
            let blocked: bool = sqlx::query_scalar("SELECT cardinality(pg_blocking_pids($1)) > 0")
                .bind(reader_pid)
                .fetch_one(&mut observer)
                .await?;
            if blocked {
                return Ok::<_, sqlx::Error>(());
            }
            async_std::task::sleep(Duration::from_millis(10)).await;
        }
    })
    .await??;
    transaction.commit().await?;

    let rows = pending.await?;
    let expected = if matches {
        vec![(
            1,
            "alpha".to_owned(),
            if update_query {
                "after_reader"
            } else {
                "after"
            }
            .to_owned(),
        )]
    } else {
        vec![]
    };
    assert_eq!(rows, expected);
    Ok(())
}
