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
use tests::fixtures::*;

// These cases deliberately hold transactions that affect other databases' removal horizons.
static TOAST_SNAPSHOT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[rstest]
#[case::mixed_segment(100)]
#[case::empty_input(1)]
fn mutable_vacuum_removes_dead_ctids_while_toast_merge_is_deferred(
    database: Db,
    #[case] mutable_segment_rows: i32,
) {
    let _snapshot_lock = TOAST_SNAPSHOT_LOCK.lock().unwrap();
    let mut conn = block_on(database.connection());
    let mut old = block_on(database.connection());
    r#"
        CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;
        CREATE TABLE toast_vacuum (id int PRIMARY KEY, message text)
            WITH (autovacuum_enabled = false, toast.autovacuum_enabled = false);
        ALTER TABLE toast_vacuum ALTER COLUMN message SET STORAGE EXTERNAL;
        CREATE INDEX toast_vacuum_idx ON toast_vacuum USING paradedb (id, message)
            WITH (mutable_segment_rows = 100, layer_sizes = '0', background_layer_sizes = '0');
    "#
    .execute(&mut conn);
    format!("ALTER INDEX toast_vacuum_idx SET (mutable_segment_rows = {mutable_segment_rows})")
        .execute(&mut conn);
    for id in 1..=4 {
        format!("INSERT INTO toast_vacuum VALUES ({id}, 'token ' || repeat(md5('{id}'), 1000))")
            .execute(&mut conn);
    }
    "DELETE FROM toast_vacuum WHERE id <= 2".execute(&mut conn);
    "BEGIN ISOLATION LEVEL REPEATABLE READ".execute(&mut old);
    let _: (i64,) = "SELECT txid_current()".fetch_one(&mut old);
    "DELETE FROM toast_vacuum WHERE id = 3".execute(&mut conn);
    "VACUUM (INDEX_CLEANUP ON) toast_vacuum".execute(&mut conn);

    let (docs,): (i64,) =
        "SELECT sum(num_docs)::bigint FROM paradedb.index_info('toast_vacuum_idx')"
            .fetch_one(&mut conn);
    assert_eq!(docs, 2, "VACUUM must remove the fully dead ctids");
    let rows: Vec<(i32,)> =
        "SELECT id FROM toast_vacuum WHERE message ||| 'token' ORDER BY id".fetch(&mut old);
    assert_eq!(rows, vec![(3,), (4,)]);

    "INSERT INTO toast_vacuum VALUES (5, 'token newest')".execute(&mut conn);
    let rows: Vec<(i32,)> =
        "SELECT id FROM toast_vacuum WHERE message ||| 'token' ORDER BY id".fetch(&mut conn);
    assert_eq!(rows, vec![(4,), (5,)]);
    "ROLLBACK".execute(&mut old);
    "VACUUM (INDEX_CLEANUP ON) toast_vacuum".execute(&mut conn);
    let (mutable,): (i64,) =
        "SELECT count(*) FROM paradedb.index_info('toast_vacuum_idx') WHERE mutable"
            .fetch_one(&mut conn);
    assert_eq!(
        mutable, 0,
        "materialization must resume after the old snapshot ends"
    );
}
