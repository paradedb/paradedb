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

//! `CREATE INDEX CONCURRENTLY` commits between its phases, so anything the validation pass
//! leaves behind outlives the transaction that owns it. Rows have to reach the index through
//! `aminsert` for that pass to run at all, which takes a second backend writing during the
//! build.

use anyhow::Result;
use rstest::*;
use sqlx::PgConnection;
use tests::fixtures::*;

const ROWS: i64 = 60_000;

async fn setup(conn: &mut PgConnection) {
    sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;")
        .execute(&mut *conn)
        .await
        .expect("create extension");

    sqlx::query(
        r#"
        CREATE TABLE cic_writes (
            id BIGSERIAL PRIMARY KEY,
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
        INSERT INTO cic_writes (body)
        SELECT 'lorem ipsum ' || i || ' ' || repeat('padding here ', 8)
        FROM generate_series(1, $1) i;
        "#,
    )
    .bind(ROWS)
    .execute(&mut *conn)
    .await
    .expect("seed rows");
}

#[rstest]
#[async_std::test]
async fn writes_during_a_concurrent_build(database: Db) -> Result<()> {
    let mut builder = database.connection().await;
    let mut writer = database.connection().await;
    setup(&mut builder).await;

    let build = async {
        sqlx::query(
            r#"
            CREATE INDEX CONCURRENTLY cic_writes_idx ON cic_writes
            USING bm25 (id, body) WITH (key_field = 'id');
            "#,
        )
        .execute(&mut builder)
        .await
        .expect("concurrent build");
    };

    let churn = async {
        for round in 0..40 {
            sqlx::query("UPDATE cic_writes SET body = body || ' revised' WHERE id % 53 = $1;")
                .bind(round % 53)
                .execute(&mut writer)
                .await
                .expect("indexed update");
            sqlx::query(
                "INSERT INTO cic_writes (body) SELECT 'late row ' || g
                 FROM generate_series(1, 50) g;",
            )
            .execute(&mut writer)
            .await
            .expect("insert");
        }
    };

    futures::join!(build, churn);

    let mut conn = database.connection().await;
    let (indexed,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM cic_writes WHERE id @@@ pdb.all();")
            .fetch_one(&mut conn)
            .await
            .expect("count via index");
    let (actual,): (i64,) = sqlx::query_as("SELECT count(*) FROM cic_writes;")
        .fetch_one(&mut conn)
        .await
        .expect("count via heap");
    assert_eq!(
        indexed, actual,
        "the index has to hold exactly the table's rows"
    );

    Ok(())
}
