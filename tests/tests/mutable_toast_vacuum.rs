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

use futures::executor::block_on;
use rstest::*;
use sqlx::PgConnection;
use tests::fixtures::*;

#[rstest]
fn mutable_vacuum_removes_dead_ctids_while_toast_merge_is_deferred(database: Db) {
    let mut conn = block_on(database.connection());
    let mut old = block_on(database.connection());
    "CREATE EXTENSION IF NOT EXISTS pg_search CASCADE".execute(&mut conn);

    run_mutable_vacuum_scenario(
        &mut conn,
        &mut old,
        "toast_vacuum_mixed",
        "toast_vacuum_mixed_idx",
        100,
    );
    run_mutable_vacuum_scenario(
        &mut conn,
        &mut old,
        "toast_vacuum_empty",
        "toast_vacuum_empty_idx",
        1,
    );
}

fn run_mutable_vacuum_scenario(
    conn: &mut PgConnection,
    old: &mut PgConnection,
    table: &str,
    index: &str,
    mutable_segment_rows: i32,
) {
    format!(
        "CREATE TABLE {table} (id int PRIMARY KEY, message text) \
         WITH (autovacuum_enabled = false, toast.autovacuum_enabled = false); \
         ALTER TABLE {table} ALTER COLUMN message SET STORAGE EXTERNAL; \
         CREATE INDEX {index} ON {table} USING paradedb (id, message) \
         WITH (mutable_segment_rows = {mutable_segment_rows}, layer_sizes = '0', background_layer_sizes = '0');"
    )
    .execute(conn);
    for id in 1..=4 {
        format!("INSERT INTO {table} VALUES ({id}, 'token ' || repeat(md5('{id}'), 1000))")
            .execute(conn);
    }
    format!("DELETE FROM {table} WHERE id <= 2").execute(conn);
    "BEGIN ISOLATION LEVEL REPEATABLE READ".execute(old);
    let _: (i64,) = "SELECT txid_current()".fetch_one(old);
    format!("DELETE FROM {table} WHERE id = 3").execute(conn);
    format!("VACUUM (INDEX_CLEANUP ON) {table}").execute(conn);

    let (docs,): (i64,) =
        format!("SELECT sum(num_docs)::bigint FROM paradedb.index_info('{index}')").fetch_one(conn);
    assert_eq!(
        docs, 2,
        "VACUUM must remove the fully dead ctids in {index}"
    );
    let rows: Vec<(i32,)> =
        format!("SELECT id FROM {table} WHERE message ||| 'token' ORDER BY id").fetch(old);
    assert_eq!(rows, vec![(3,), (4,)]);

    format!("INSERT INTO {table} VALUES (5, 'token newest')").execute(conn);
    let rows: Vec<(i32,)> =
        format!("SELECT id FROM {table} WHERE message ||| 'token' ORDER BY id").fetch(conn);
    assert_eq!(rows, vec![(4,), (5,)]);
    "ROLLBACK".execute(old);
    format!("VACUUM (INDEX_CLEANUP ON) {table}").execute(conn);
    let (mutable,): (i64,) =
        format!("SELECT count(*) FROM paradedb.index_info('{index}') WHERE mutable")
            .fetch_one(conn);
    assert_eq!(
        mutable, 0,
        "materialization must resume after the old snapshot ends in {index}"
    );
}
