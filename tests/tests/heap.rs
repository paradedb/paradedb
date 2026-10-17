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
use lockfree_object_pool::MutexObjectPool;
use proptest::prelude::*;
use rstest::*;
use sqlx::PgConnection;
use tests::fixtures::*;

#[rstest]
#[case::vacuumed_toast(false)]
#[case::old_snapshot(true)]
fn mutable_merge_defers_recently_dead_toast(database: Db, #[case] same_database: bool) {
    let mut conn = block_on(database.connection());
    let other_database = block_on(Db::new());
    let mut old = block_on(if same_database {
        database.connection()
    } else {
        other_database.connection()
    });
    r#"
        CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;
        CREATE TABLE toast_merge (id int PRIMARY KEY, message text)
            WITH (autovacuum_enabled = false, toast.autovacuum_enabled = false);
        ALTER TABLE toast_merge ALTER COLUMN message SET STORAGE EXTERNAL;
        CREATE INDEX toast_merge_idx ON toast_merge USING paradedb (id, message)
            WITH (mutable_segment_rows = 100, target_segment_count = 1,
                  layer_sizes = '0', background_layer_sizes = '0');
        INSERT INTO toast_merge
            SELECT i, 'token ' || repeat(md5(i::text), 1000) FROM generate_series(1, 2) i;
    "#
    .execute(&mut conn);
    let (toast_relation,): (String,) =
        "SELECT reltoastrelid::regclass::text FROM pg_class WHERE oid = 'toast_merge'::regclass"
            .fetch_one(&mut conn);
    "BEGIN ISOLATION LEVEL REPEATABLE READ".execute(&mut old);
    let _: (i64,) = "SELECT txid_current()".fetch_one(&mut old);

    "DELETE FROM toast_merge WHERE id = 1".execute(&mut conn);
    format!("VACUUM {toast_relation}").execute(&mut conn);
    let (toast_values,): (i64,) = format!("SELECT count(DISTINCT chunk_id) FROM {toast_relation}")
        .fetch_one(if same_database { &mut old } else { &mut conn });
    assert_eq!(toast_values, if same_database { 2 } else { 1 });

    "ALTER INDEX toast_merge_idx SET (mutable_segment_rows = 0, layer_sizes = '100mb')"
        .execute(&mut conn);
    let (global_target,): (String,) =
        "SHOW paradedb.global_target_segment_count".fetch_one(&mut conn);
    let target = global_target.parse::<i32>().unwrap().max(1);
    for id in 3..=target + 3 {
        format!("INSERT INTO toast_merge VALUES ({id}, 'token newest')").execute(&mut conn);
    }
    let (mutable,): (i64,) =
        "SELECT count(*) FROM paradedb.index_info('toast_merge_idx') WHERE mutable"
            .fetch_one(&mut conn);
    assert_eq!(mutable, 1, "the deferred input must remain visible");
    let rows: Vec<(i32,)> =
        "SELECT id FROM toast_merge WHERE message ||| 'token' ORDER BY id".fetch(&mut conn);
    assert_eq!(rows, (2..=target + 3).map(|id| (id,)).collect::<Vec<_>>());
    if same_database {
        let rows: Vec<(i32,)> =
            "SELECT id FROM toast_merge WHERE message ||| 'token' ORDER BY id".fetch(&mut old);
        assert_eq!(rows, vec![(1,), (2,)]);
    }

    "ROLLBACK".execute(&mut old);
    "VACUUM toast_merge".execute(&mut conn);
    for id in target + 4..=2 * target + 4 {
        format!("INSERT INTO toast_merge VALUES ({id}, 'token newest')").execute(&mut conn);
    }
    let (mutable,): (i64,) =
        "SELECT count(*) FROM paradedb.index_info('toast_merge_idx') WHERE mutable"
            .fetch_one(&mut conn);
    assert_eq!(mutable, 0, "merging must resume after VACUUM");
    let rows: Vec<(i32,)> =
        "SELECT id FROM toast_merge WHERE message ||| 'token' ORDER BY id".fetch(&mut conn);
    assert_eq!(
        rows,
        (2..=2 * target + 4).map(|id| (id,)).collect::<Vec<_>>()
    );
}

#[rstest]
fn mvcc_heap_filter(mut conn: PgConnection) {
    r#"
        CALL paradedb.create_paradedb_test_table(table_name => 'heap_and_clauses_table', schema_name => 'public');

        CREATE INDEX heap_and_clauses_idx ON heap_and_clauses_table
        USING paradedb (id, description);
    "#.execute(&mut conn);

    // Ensure that heap filters continue to be applied correctly in the presence of updates.
    for _ in 0..128 {
        let results: Vec<(i32, String)> = r#"
            SELECT id, description
            FROM heap_and_clauses_table
            WHERE id @@@ paradedb.match('description', 'Sleek running', conjunction_mode := true)
            AND description ILIKE 'Sleek running shoes'
            ORDER BY id;
        "#
        .fetch(&mut conn);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 3);
        assert_eq!(results[0].1, "Sleek running shoes");

        r#"
            UPDATE heap_and_clauses_table SET last_updated_date = NOW();
        "#
        .execute(&mut conn);
    }
}

#[rstest]
fn mvcc_snippet(mut conn: PgConnection) {
    r#"
        CALL paradedb.create_paradedb_test_table(table_name => 'mock_items', schema_name => 'public');
        
        CREATE INDEX mock_items_idx ON mock_items
        USING paradedb (id, description);
    "#
    .execute(&mut conn);

    // Ensure that snippet lookups from the heap succeed in the presence of updates.
    for _ in 0..128 {
        let results: Vec<(i32, String)> = r#"
            SELECT id, pdb.snippet(description)
            FROM mock_items
            WHERE description ||| 'shoes'
            ORDER BY id
            LIMIT 5;
        "#
        .fetch(&mut conn);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].0, 3);
        assert_eq!(results[0].1, "Sleek running <b>shoes</b>");
        assert_eq!(results[1].0, 4);
        assert_eq!(results[1].1, "White jogging <b>shoes</b>");
        assert_eq!(results[2].0, 5);
        assert_eq!(results[2].1, "Generic <b>shoes</b>");

        r#"
            UPDATE mock_items SET last_updated_date = NOW();
        "#
        .execute(&mut conn);
    }
}

#[rstest]
#[tokio::test]
async fn visibility_map_shortcuts(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );
    "CREATE EXTENSION IF NOT EXISTS pg_search CASCADE".execute(&mut pool.pull());
    let default: (String,) =
        "SHOW paradedb.enable_visibility_map_shortcuts".fetch_one(&mut pool.pull());
    assert_eq!(default.0, "on");

    proptest!(ProptestConfig::with_cases(32), |(
        num_docs in 256u32..2048,
        padding in 32u32..600,
        fillfactor in 50u32..100,
        segment_count in 1u32..5,
        descending in any::<bool>(),
        match_every in 2u32..8,
        mutations in prop::collection::vec((0u8..3, any::<u16>(), 1u32..256), 1..12),
    )| {
        let mut conn = pool.pull();
        let direction = if descending { "DESC NULLS LAST" } else { "ASC NULLS FIRST" };
        format!(
            r#"
            SET max_parallel_workers_per_gather = 0;
            DROP TABLE IF EXISTS visibility_blocks;
            CREATE TABLE visibility_blocks (
                id int PRIMARY KEY, title text, value int, padding text
            ) WITH (autovacuum_enabled = false, fillfactor = {fillfactor});
            INSERT INTO visibility_blocks
            SELECT id, CASE WHEN id % {match_every} = 0 THEN 'database' ELSE 'postgres' END,
                   id, repeat('x', {padding}) FROM generate_series(1, {num_docs}) id;
            CREATE INDEX visibility_blocks_idx ON visibility_blocks USING paradedb (id, title, value)
            WITH (sort_by = 'ctid {direction}', mutable_segment_rows = 0,
                  target_segment_count = {segment_count}, background_layer_sizes = '0');
            "#
        ).execute(&mut conn);
        "VACUUM (INDEX_CLEANUP ON, ANALYZE) visibility_blocks".execute(&mut conn);

        for phase in 0..3 {
            if phase == 1 {
                // Overlapping runs create varied dirty blocks, dead index entries, and HOT chains.
                for &(operation, offset, length) in &mutations {
                    let first = u32::from(offset) % num_docs + 1;
                    let last = (first + length - 1).min(num_docs);
                    let statement = match operation {
                        0 => "UPDATE visibility_blocks SET padding = 'changed'",
                        1 => "UPDATE visibility_blocks SET title = CASE WHEN title = 'database' THEN 'postgres' ELSE 'database' END, value = -value",
                        _ => "DELETE FROM visibility_blocks",
                    };
                    format!("{statement} WHERE id BETWEEN {first} AND {last}").execute(&mut conn);
                }
            } else if phase == 2 {
                "VACUUM (INDEX_CLEANUP ON, ANALYZE) visibility_blocks".execute(&mut conn);
            }
            let expected_count: (i64,) =
                "SELECT count(*) FROM visibility_blocks".fetch_one(&mut conn);
            let expected_aggregate: (i64, Option<i64>) =
                "SELECT count(*), sum(value) FROM visibility_blocks WHERE title = 'database'"
                    .fetch_one(&mut conn);
            let expected_rows: Vec<(i32, i32, String)> =
                "SELECT id, value, ctid::text FROM visibility_blocks WHERE title = 'database' ORDER BY id"
                    .fetch(&mut conn);

            for enabled in [true, false] {
                format!("SET paradedb.enable_visibility_map_shortcuts = {enabled}").execute(&mut conn);
                for workers in [0, 2] {
                    format!("SET max_parallel_workers_per_gather = {workers}").execute(&mut conn);
                    let count: (i64,) =
                        "SELECT count(*) FROM visibility_blocks WHERE id @@@ pdb.all()"
                            .fetch_one(&mut conn);
                    prop_assert_eq!(count, expected_count, "phase {}, shortcuts {}, workers {}", phase, enabled, workers);
                }
                "SET max_parallel_workers_per_gather = 0".execute(&mut conn);
                let query = "SELECT count(*), sum(value) FROM visibility_blocks WHERE title === 'database'";
                let (plan,): (serde_json::Value,) =
                    format!("EXPLAIN (FORMAT JSON) {query}").fetch_one(&mut conn);
                prop_assert!(plan.to_string().contains("ParadeDB Aggregate Scan"));
                let aggregate: (i64, Option<i64>) = query.fetch_one(&mut conn);
                let rows: Vec<(i32, i32, String)> =
                    "SELECT id, value, ctid::text FROM visibility_blocks WHERE title === 'database' ORDER BY id"
                        .fetch(&mut conn);
                prop_assert_eq!(aggregate, expected_aggregate, "phase {}, shortcuts {}", phase, enabled);
                prop_assert_eq!(&rows, &expected_rows, "phase {}, shortcuts {}", phase, enabled);
            }
        }
    });
}
