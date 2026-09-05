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

//! A `partition_by` build under `CONCURRENTLY` cuts its partitions from a heap sample, then
//! re-reads the rows it scanned to fill them. Between those two passes other backends keep
//! writing, so the re-read has to land on the versions the scan chose, not on whatever the
//! chain holds by then. These tests race real writers against the build and check the index
//! against the table.

use anyhow::Result;
use rstest::*;
use sqlx::PgConnection;
use tests::fixtures::*;

/// The rows a build has to account for, whatever a concurrent writer does to them.
const ROWS: i64 = 60_000;

async fn setup(conn: &mut PgConnection) {
    sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;")
        .execute(&mut *conn)
        .await
        .expect("create extension");

    sqlx::query(
        r#"
        CREATE TABLE cic_race (
            id BIGSERIAL PRIMARY KEY,
            tenant_id BIGINT,
            body TEXT,
            touched BIGINT DEFAULT 0
        );
        "#,
    )
    .execute(&mut *conn)
    .await
    .expect("create table");

    sqlx::query(
        r#"
        INSERT INTO cic_race (tenant_id, body)
        SELECT (i * 7919) % 500, 'lorem ipsum ' || i || ' ' || repeat('padding here ', 8)
        FROM generate_series(1, $1) i;
        "#,
    )
    .bind(ROWS)
    .execute(&mut *conn)
    .await
    .expect("seed rows");
}

/// Every row of the table has to be in the index, and no row twice.
async fn assert_index_matches_table(conn: &mut PgConnection) {
    let (indexed,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM cic_race WHERE id @@@ pdb.all();")
            .fetch_one(&mut *conn)
            .await
            .expect("count via index");
    let (actual,): (i64,) = sqlx::query_as("SELECT count(*) FROM cic_race;")
        .fetch_one(&mut *conn)
        .await
        .expect("count via heap");
    assert_eq!(
        indexed, actual,
        "the index has to hold exactly the table's rows"
    );

    // A row the build indexed under a stale version would come back with a `body` that
    // disagrees with the heap, which the join below catches.
    let (mismatched,): (i64,) = sqlx::query_as(
        r#"
        SELECT count(*) FROM cic_race t
        WHERE t.id @@@ pdb.all()
          AND t.body <> (SELECT body FROM cic_race h WHERE h.id = t.id);
        "#,
    )
    .fetch_one(&mut *conn)
    .await
    .expect("compare indexed rows to the heap");
    assert_eq!(mismatched, 0, "every indexed row has to match the heap");
}

/// `UPDATE`s of a non-indexed column keep the tuples on their page, so the build's ctids grow
/// HOT chains under it. The re-read has to pick the member its scan saw.
#[rstest]
#[async_std::test]
async fn hot_updates_during_a_concurrent_build(database: Db) -> Result<()> {
    let mut builder = database.connection().await;
    let mut writer = database.connection().await;
    setup(&mut builder).await;

    let build = async {
        sqlx::query(
            r#"
            CREATE INDEX CONCURRENTLY cic_race_idx ON cic_race
            USING bm25 (id, tenant_id, body)
            WITH (key_field = 'id', partition_by = 'tenant_id', target_segment_count = 8,
                  numeric_fields = '{"tenant_id": {"fast": true}}');
            "#,
        )
        .execute(&mut builder)
        .await
        .expect("concurrent build");
    };

    let churn = async {
        for round in 0..40 {
            sqlx::query("UPDATE cic_race SET touched = touched + 1 WHERE id % 37 = $1;")
                .bind(round % 37)
                .execute(&mut writer)
                .await
                .expect("hot update");
        }
    };

    futures::join!(build, churn);

    let mut conn = database.connection().await;
    assert_index_matches_table(&mut conn).await;
    Ok(())
}

/// `UPDATE`s of an indexed column break the HOT chain and move rows to new ctids, while
/// `INSERT`s and `DELETE`s change the set the build has to cover.
#[rstest]
#[async_std::test]
async fn writes_during_a_concurrent_build(database: Db) -> Result<()> {
    let mut builder = database.connection().await;
    let mut writer = database.connection().await;
    setup(&mut builder).await;

    let build = async {
        sqlx::query(
            r#"
            CREATE INDEX CONCURRENTLY cic_race_idx ON cic_race
            USING bm25 (id, tenant_id, body)
            WITH (key_field = 'id', partition_by = 'tenant_id', target_segment_count = 8,
                  numeric_fields = '{"tenant_id": {"fast": true}}');
            "#,
        )
        .execute(&mut builder)
        .await
        .expect("concurrent build");
    };

    let churn = async {
        for round in 0..30 {
            sqlx::query("UPDATE cic_race SET body = body || ' revised' WHERE id % 53 = $1;")
                .bind(round % 53)
                .execute(&mut writer)
                .await
                .expect("indexed update");
            sqlx::query(
                "INSERT INTO cic_race (tenant_id, body) SELECT $1, 'late row ' || g
                 FROM generate_series(1, 50) g;",
            )
            .bind(round)
            .execute(&mut writer)
            .await
            .expect("insert");
            sqlx::query("DELETE FROM cic_race WHERE id = $1;")
                .bind(round * 101 + 1)
                .execute(&mut writer)
                .await
                .expect("delete");
        }
    };

    futures::join!(build, churn);

    let mut conn = database.connection().await;
    assert_index_matches_table(&mut conn).await;
    Ok(())
}
