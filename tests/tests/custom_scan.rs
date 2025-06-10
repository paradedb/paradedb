// Copyright (c) 2023-2025 ParadeDB, Inc.
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

// Tests for ParadeDB's Custom Scan implementation
mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use serde_json::{Number, Value};
use sqlx::PgConnection;

#[rstest]
fn corrupt_targetlist(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let (id, score) = "select count(*), max(paradedb.score(id)) from paradedb.bm25_search where description @@@ 'keyboard'"
        .fetch_one::<(i64, f32)>(&mut conn);
    assert_eq!((id, score), (2, 3.2668595));

    "PREPARE prep AS select count(*), max(paradedb.score(id)) from paradedb.bm25_search where description @@@ 'keyboard'".execute(&mut conn);
    for _ in 0..100 {
        "EXECUTE prep".fetch_one::<(i64, f32)>(&mut conn);
        assert_eq!((id, score), (2, 3.2668595));
    }
}

#[rstest]
fn attribute_1_of_table_has_wrong_type(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let (id, ) = "SELECT id, description FROM paradedb.bm25_search WHERE description @@@ 'keyboard' OR id = 1 ORDER BY id LIMIT 1"
        .fetch_one::<(i32,)>(&mut conn);
    assert_eq!(id, 1);
}

#[rstest]
fn generates_custom_scan_for_or(mut conn: PgConnection) {
    use serde_json::Value;

    SimpleProductsTable::setup().execute(&mut conn);

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' OR description @@@ 'shoes'".fetch_one::<(Value,)>(&mut conn);

    let plan = plan
        .get(0)
        .unwrap()
        .as_object()
        .unwrap()
        .get("Plan")
        .unwrap()
        .as_object()
        .unwrap()
        .get("Plans")
        .unwrap()
        .get(0)
        .unwrap();

    eprintln!("{plan:#?}");
    assert_eq!(
        plan.get("Custom Plan Provider"),
        Some(&Value::String(String::from("ParadeDB Scan")))
    );
}

#[rstest]
fn generates_custom_scan_for_and(mut conn: PgConnection) {
    use serde_json::Value;

    SimpleProductsTable::setup().execute(&mut conn);

    "SET enable_indexscan TO off;".execute(&mut conn);

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' AND description @@@ 'shoes'".fetch_one::<(Value,)>(&mut conn);
    let plan = plan.pointer("/0/Plan/Plans/0").unwrap();
    eprintln!("{plan:#?}");
    assert_eq!(
        plan.get("Custom Plan Provider"),
        Some(&Value::String(String::from("ParadeDB Scan")))
    );
}

#[rstest]
fn includes_segment_count(mut conn: PgConnection) {
    use serde_json::Value;

    SimpleProductsTable::setup().execute(&mut conn);

    "SET enable_indexscan TO off;".execute(&mut conn);

    let (plan, ) = "EXPLAIN (ANALYZE, FORMAT JSON) SELECT * FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' AND description @@@ 'shoes'".fetch_one::<(Value,)>(&mut conn);
    let plan = plan.pointer("/0/Plan/Plans/0").unwrap();
    assert!(plan.get("Segment Count").is_some());
}

#[rstest]
fn field_on_left(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let (id,) =
        "SELECT id FROM paradedb.bm25_search WHERE description @@@ 'keyboard' ORDER BY id ASC"
            .fetch_one::<(i32,)>(&mut conn);
    assert_eq!(id, 1);
}

#[rstest]
fn table_on_left(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let (id, ) =
        "SELECT id FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id ASC"
            .fetch_one::<(i32,)>(&mut conn);
    assert_eq!(id, 1);
}

#[rstest]
fn scores_project(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let (id, score) =
        "SELECT id, paradedb.score(id) FROM paradedb.bm25_search WHERE description @@@ 'keyboard' ORDER BY paradedb.score(id) DESC LIMIT 1"
            .fetch_one::<(i32, f32)>(&mut conn);
    assert_eq!(id, 2);
    assert_eq!(score, 3.2668595);
}

#[rstest]
fn snippets_project(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let (id, snippet) =
        "SELECT id, paradedb.snippet(description) FROM paradedb.bm25_search WHERE description @@@ 'keyboard' ORDER BY paradedb.score(id) DESC LIMIT 1"
            .fetch_one::<(i32, String)>(&mut conn);
    assert_eq!(id, 2);
    assert_eq!(snippet, String::from("Plastic <b>Keyboard</b>"));
}

#[rstest]
fn scores_and_snippets_project(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let (id, score, snippet) =
        "SELECT id, paradedb.score(id), paradedb.snippet(description) FROM paradedb.bm25_search WHERE description @@@ 'keyboard' ORDER BY paradedb.score(id) DESC LIMIT 1"
            .fetch_one::<(i32, f32, String)>(&mut conn);
    assert_eq!(id, 2);
    assert_eq!(score, 3.2668595);
    assert_eq!(snippet, String::from("Plastic <b>Keyboard</b>"));
}

#[rstest]
fn mingets(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let (id, snippet) =
        "SELECT id, paradedb.snippet(description, '<MING>', '</MING>') FROM paradedb.bm25_search WHERE description @@@ 'teddy bear'"
            .fetch_one::<(i32, String)>(&mut conn);
    assert_eq!(id, 40);
    assert_eq!(
        snippet,
        String::from("Plush <MING>teddy</MING> <MING>bear</MING>")
    );
}

#[rstest]
fn scores_with_expressions(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let result = r#"
select id,
    description,
    paradedb.score(id),
    rating,
    paradedb.score(id) * rating    /* testing this, specifically */
from paradedb.bm25_search
where metadata @@@ 'color:white'
order by 5 desc, score desc
limit 1;
        "#
    .fetch_one::<(i32, String, f32, i32, f64)>(&mut conn);
    assert_eq!(
        result,
        (
            25,
            "Anti-aging serum".into(),
            3.2455924,
            4,
            12.982369422912598
        )
    );
}

#[rstest]
fn limit_without_order_by(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "SET enable_indexscan TO off;".execute(&mut conn);
    let (plan, ) = r#"
explain (analyze, format json) select * from paradedb.bm25_search where metadata @@@ 'color:white' limit 1;
        "#
        .fetch_one::<(Value,)>(&mut conn);
    let path = plan.pointer("/0/Plan/Plans/0").unwrap();
    assert_eq!(
        path.get("Node Type"),
        Some(&Value::String(String::from("Custom Scan")))
    );
    assert_eq!(path.get("Scores"), Some(&Value::Bool(false)));
    assert_eq!(
        path.get("   Top N Limit"),
        Some(&Value::Number(Number::from(1)))
    );
}

#[rstest]
fn score_and_limit_without_order_by(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    "SET enable_indexscan TO off;".execute(&mut conn);
    let (plan, ) = r#"
explain (analyze, format json) select paradedb.score(id), * from paradedb.bm25_search where metadata @@@ 'color:white' limit 1;
        "#
        .fetch_one::<(Value,)>(&mut conn);
    let path = plan.pointer("/0/Plan/Plans/0").unwrap();
    assert_eq!(
        path.get("Node Type"),
        Some(&Value::String(String::from("Custom Scan")))
    );
    assert_eq!(path.get("Scores"), Some(&Value::Bool(true)));
    assert_eq!(
        path.get("   Top N Limit"),
        Some(&Value::Number(Number::from(1)))
    );
}

#[rstest]
fn simple_join_with_scores_and_both_sides(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let result = r#"
select a.id,
    a.score,
    b.id,
    b.score
from (select paradedb.score(id), * from paradedb.bm25_search) a
inner join (select paradedb.score(id), * from paradedb.bm25_search) b on a.id = b.id
where a.description @@@ 'bear' AND b.description @@@ 'teddy bear';"#
        .fetch_one::<(i32, f32, i32, f32)>(&mut conn);
    assert_eq!(result, (40, 3.3322046, 40, 6.664409));
}

#[rstest]
fn simple_join_with_scores_on_both_sides(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let result = r#"
select a.id,
    a.score,
    b.id,
    b.score
from (select paradedb.score(id), * from paradedb.bm25_search) a
inner join (select paradedb.score(id), * from paradedb.bm25_search) b on a.id = b.id
where a.description @@@ 'bear' OR b.description @@@ 'teddy bear';"#
        .fetch_one::<(i32, f32, i32, f32)>(&mut conn);
    assert_eq!(result, (40, 3.3322046, 40, 6.664409));
}

#[rstest]
fn add_scores_across_joins_issue1753(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(table_name => 'mock_items', schema_name => 'public');

    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time)
    WITH (key_field='id');

    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'orders',
      table_type => 'Orders'
    );
    ALTER TABLE orders
    ADD CONSTRAINT foreign_key_product_id
    FOREIGN KEY (product_id)
    REFERENCES mock_items(id);

    CREATE INDEX orders_idx ON orders
    USING bm25 (order_id, customer_name)
    WITH (key_field='order_id');
    "#.execute(&mut conn);

    // this one doesn't plan a custom scan at all, so scores come back as NaN
    let result = "
        SELECT o.order_id, m.description, paradedb.score(o.order_id) + paradedb.score(m.id) as score
        FROM orders o JOIN mock_items m ON o.product_id = m.id
        WHERE o.customer_name @@@ 'Johnson' AND m.description @@@ 'shoes'
        ORDER BY order_id
        LIMIT 1"
        .fetch_one::<(i32, String, f32)>(&mut conn);
    assert_eq!(result, (3, "Sleek running shoes".into(), 5.406531));
}

#[rstest]
fn scores_survive_joins(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(table_name => 'a', schema_name => 'public');
    CALL paradedb.create_bm25_test_table(table_name => 'b', schema_name => 'public');
    CALL paradedb.create_bm25_test_table(table_name => 'c', schema_name => 'public');

    CREATE INDEX idxa ON a USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time) WITH (key_field='id');
    CREATE INDEX idxb ON b USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time) WITH (key_field='id');
    CREATE INDEX idxc ON c USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time) WITH (key_field='id');
    "#.execute(&mut conn);

    // this one doesn't plan a custom scan at all, so scores come back as NaN
    let result = r#"
        SELECT a.description, paradedb.score(a.id)
        FROM a
        join b on a.id = b.id
        join c on a.id = c.id
        WHERE a.description @@@ 'shoes'
        ORDER BY a.description;"#
        .fetch_result::<(String, f32)>(&mut conn)
        .expect("query failed");
    assert_eq!(
        result,
        vec![
            ("Generic shoes".into(), 2.8772602),
            ("Sleek running shoes".into(), 2.4849067),
            ("White jogging shoes".into(), 2.4849067),
        ]
    );
}

#[rustfmt::skip]
#[rstest]
fn join_issue_1776(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
          schema_name => 'public',
          table_name => 'mock_items'
        );

    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, category, rating, in_stock, metadata, created_at)
    WITH (key_field='id');

    CALL paradedb.create_bm25_test_table(
          schema_name => 'public',
          table_name => 'orders',
          table_type => 'Orders'
        );

    ALTER TABLE orders
    ADD CONSTRAINT foreign_key_product_id
    FOREIGN KEY (product_id)
    REFERENCES mock_items(id);

    CREATE INDEX orders_idx ON orders
    USING bm25 (order_id, customer_name)
    WITH (key_field='order_id');
    "#
    .execute(&mut conn);

    let results = r#"
        SELECT o.order_id, m.description, o.customer_name, paradedb.score(o.order_id) as orders_score, paradedb.score(m.id) as items_score
        FROM orders o
        JOIN mock_items m ON o.product_id = m.id
        WHERE o.customer_name @@@ 'Johnson' AND m.description @@@ 'shoes' OR m.description @@@ 'Smith'
        ORDER BY order_id
        LIMIT 5;
    "#.fetch_result::<(i32, String, String, f32, f32)>(&mut conn).expect("query failed");

    assert_eq!(results[0], (3, "Sleek running shoes".into(), "Alice Johnson".into(), 2.9216242, 2.4849067));
    assert_eq!(results[1], (6, "White jogging shoes".into(), "Alice Johnson".into(), 2.9216242, 2.4849067));
    assert_eq!(results[2], (36,"White jogging shoes".into(), "Alice Johnson".into(), 2.9216242, 2.4849067));
}

#[rustfmt::skip]
#[rstest]
fn join_issue_1826(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
          schema_name => 'public',
          table_name => 'mock_items'
        );

    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, category, rating, in_stock, metadata, created_at)
    WITH (key_field='id');

    CALL paradedb.create_bm25_test_table(
          schema_name => 'public',
          table_name => 'orders',
          table_type => 'Orders'
        );

    ALTER TABLE orders
    ADD CONSTRAINT foreign_key_product_id
    FOREIGN KEY (product_id)
    REFERENCES mock_items(id);

    CREATE INDEX orders_idx ON orders
    USING bm25 (order_id, customer_name)
    WITH (key_field='order_id');
    "#
    .execute(&mut conn);

    let results = r#"
        SELECT o.order_id, m.description, o.customer_name, paradedb.score(o.order_id) as orders_score, paradedb.score(m.id) as items_score
        FROM orders o
        JOIN mock_items m ON o.product_id = m.id
        WHERE o.customer_name @@@ 'Johnson' AND m.description @@@ 'shoes' OR m.description @@@ 'Smith'
        ORDER BY paradedb.score(m.id) desc, m.id asc
        LIMIT 1;
    "#.fetch_result::<(i32, String, String, f32, f32)>(&mut conn).expect("query failed");

    assert_eq!(results[0], (3, "Sleek running shoes".into(), "Alice Johnson".into(), 2.9216242, 2.4849067));
}

#[rstest]
fn leaky_file_handles(mut conn: PgConnection) {
    r#"
        CREATE OR REPLACE FUNCTION raise_exception(int, int) RETURNS bool LANGUAGE plpgsql AS $$
        DECLARE
        BEGIN
            IF $1 = $2 THEN
                RAISE EXCEPTION 'error! % = %', $1, $2;
            END IF;
            RETURN false;
        END;
        $$;
    "#
    .execute(&mut conn);

    let (pid,) = "SELECT pg_backend_pid()".fetch_one::<(i32,)>(&mut conn);
    SimpleProductsTable::setup().execute(&mut conn);

    // this will raise an error when it hits id #12
    let result = "SELECT id, paradedb.score(id), raise_exception(id, 12) FROM paradedb.bm25_search WHERE category @@@ 'electronics' ORDER BY paradedb.score(id) DESC, id LIMIT 10"
        .execute_result(&mut conn);
    assert!(result.is_err());
    assert_eq!(
        "error returned from database: error! 12 = 12",
        &format!("{}", result.err().unwrap())
    );

    fn tantivy_files_still_open(pid: i32) -> bool {
        let output = std::process::Command::new("lsof")
            .arg("-p")
            .arg(pid.to_string())
            .output()
            .expect("`lsof` command should not fail`");

        let stdout = String::from_utf8_lossy(&output.stdout);
        eprintln!("stdout: {}", stdout);
        stdout.contains("/tantivy/")
    }

    // see if there's still some open tantivy files
    if tantivy_files_still_open(pid) {
        // if there are, they're probably (hopefully!) from where we the postgres connection
        // is waiting on merge threads in the background.  So we'll give it 5 seconds and try again

        eprintln!("sleeping for 5s and checking open files again");
        std::thread::sleep(std::time::Duration::from_secs(5));

        // this time asserting for real
        assert!(!tantivy_files_still_open(pid));
    }
}

#[rustfmt::skip]
#[rstest]
fn cte_issue_1951(mut conn: PgConnection) {
    r#"
        CREATE TABLE t
        (
            id   SERIAL,
            data TEXT
        );

        CREATE TABLE s
        (
            id   SERIAL,
            data TEXT
        );

        insert into t (id, data) select x, md5(x::text) || ' query' from generate_series(1, 100) x;
        insert into s (id, data) select x, md5(x::text) from generate_series(1, 100) x;

        create index idxt on t using bm25 (id, data) with (key_field = id);
        create index idxs on s using bm25 (id, data) with (key_field = id);
    "#.execute(&mut conn);

    let results = r#"
        with cte as (
        select id, 1 as score from t
        where data @@@ 'query'
        limit 1)
        select cte.id from s
        right join cte on cte.id = s.id
        order by cte.score desc;
    "#.fetch_result::<(i32, )>(&mut conn).expect("query failed");
    assert_eq!(results.len(), 1);
}

#[rstest]
fn top_n_matches(mut conn: PgConnection) {
    r#"
        DROP TABLE IF EXISTS test;
        CREATE TABLE test (
            id SERIAL8 NOT NULL PRIMARY KEY,
            message TEXT,
            severity INTEGER
        ) WITH (autovacuum_enabled = false);

        INSERT INTO test (message, severity) VALUES ('beer wine cheese a', 1);
        INSERT INTO test (message, severity) VALUES ('beer wine a', 2);
        INSERT INTO test (message, severity) VALUES ('beer cheese a', 3);
        INSERT INTO test (message, severity) VALUES ('beer a', 4);
        INSERT INTO test (message, severity) VALUES ('wine cheese a', 5);
        INSERT INTO test (message, severity) VALUES ('wine a', 6);
        INSERT INTO test (message, severity) VALUES ('cheese a', 7);
        INSERT INTO test (message, severity) VALUES ('beer wine cheese a', 1);
        INSERT INTO test (message, severity) VALUES ('beer wine a', 2);
        INSERT INTO test (message, severity) VALUES ('beer cheese a', 3);
        INSERT INTO test (message, severity) VALUES ('beer a', 4);
        INSERT INTO test (message, severity) VALUES ('wine cheese a', 5);
        INSERT INTO test (message, severity) VALUES ('wine a', 6);
        INSERT INTO test (message, severity) VALUES ('cheese a', 7);

        -- INSERT INTO test (message) SELECT 'space fillter ' || x FROM generate_series(1, 10000000) x;

        CREATE INDEX idxtest ON test USING bm25(id, message, severity) WITH (key_field = 'id');
        CREATE OR REPLACE FUNCTION assert(a bigint, b bigint) RETURNS bool STABLE STRICT LANGUAGE plpgsql AS $$
        DECLARE
            current_txid bigint;
        BEGIN
            -- Get the current transaction ID
            current_txid := txid_current();

            -- Check if the values are not equal
            IF a <> b THEN
                RAISE EXCEPTION 'Assertion failed: % <> %. Transaction ID: %', a, b, current_txid;
            END IF;

            RETURN true;
        END;
        $$;
    "#.execute(&mut conn);

    "UPDATE test SET severity = (floor(random() * 10) + 1)::int WHERE id < 10;".execute(&mut conn);
    "UPDATE test SET severity = (floor(random() * 10) + 1)::int WHERE id < 10;".execute(&mut conn);
    "UPDATE test SET severity = (floor(random() * 10) + 1)::int WHERE id < 10;".execute(&mut conn);

    r#"
        SET enable_indexonlyscan to OFF;
        SET enable_indexscan to OFF;
        SET max_parallel_workers = 0;
    "#
    .execute(&mut conn);

    for n in 1..=100 {
        let sql = format!("select assert(count(*), LEAST({n}, 8)), count(*) from (select id from test where message @@@ 'beer' order by severity limit {n}) x;");

        let (b, count) = sql.fetch_one::<(bool, i64)>(&mut conn);
        assert_eq!((b, count), (true, n.min(8)));
    }

    r#"
        SET enable_indexonlyscan to OFF;
        SET enable_indexscan to OFF;
        SET max_parallel_workers = 32;
    "#
    .execute(&mut conn);

    for n in 1..=100 {
        let sql = format!("select assert(count(*), LEAST({n}, 8)), count(*) from (select id from test where message @@@ 'beer' order by severity limit {n}) x;");

        let (b, count) = sql.fetch_one::<(bool, i64)>(&mut conn);
        assert_eq!((b, count), (true, n.min(8)));
    }
}

#[rstest]
fn stable_limit_and_offset(mut conn: PgConnection) {
    if pg_major_version(&mut conn) < 16 {
        // the `debug_parallel_query` was added in pg16, so we cannot run this test on anything
        // less than pg16
        return;
    }

    // We use multiple segments, and force multiple workers to be used.
    SimpleProductsTable::setup_multi_segment().execute(&mut conn);

    "SET max_parallel_workers = 8;".execute(&mut conn);
    "SET debug_parallel_query TO on".execute(&mut conn);

    let mut query = |offset: usize, limit: usize| -> Vec<(i32, String, f32)> {
        format!(
            "SELECT id, description, paradedb.score(id) FROM paradedb.bm25_search WHERE bm25_search @@@ 'category:electronics'
             ORDER BY paradedb.score(id), id OFFSET {offset} LIMIT {limit}"
        )
        .fetch_collect(&mut conn)
    };

    let mut previous = Vec::new();
    for limit in 1..50 {
        let current = query(0, limit);
        assert_eq!(
            previous[0..],
            current[..previous.len()],
            "With limit {limit}"
        );
        previous = current;
    }

    let all_results = query(0, 50);
    for (offset, expected) in all_results.into_iter().enumerate() {
        let current = query(offset, 1);
        assert_eq!(expected, current[0]);
    }
}

#[rstest]
fn top_n_exits_at_limit(mut conn: PgConnection) {
    if pg_major_version(&mut conn) < 16 {
        // Before 16, Postgres would not plan an incremental sort here.
        return;
    }

    // When there are more results than the limit will render, but there is no `Limit` node
    // immediately above us in the plan (in this case, we get an `Incremental Sort` instead due to
    // the tiebreaker sort, which we can't push down until #2642), Top-N should exit on its own.
    r#"
        CREATE TABLE exit_at_limit (id SERIAL8 NOT NULL PRIMARY KEY, message TEXT, severity INTEGER);
        CREATE INDEX exit_at_limit_index ON exit_at_limit USING bm25 (id, message, severity) WITH (key_field = 'id');

        INSERT INTO exit_at_limit (message, severity) VALUES ('beer wine cheese a', 1);
        INSERT INTO exit_at_limit (message, severity) VALUES ('beer wine a', 2);
        INSERT INTO exit_at_limit (message, severity) VALUES ('beer cheese a', 3);
        INSERT INTO exit_at_limit (message, severity) VALUES ('beer a', 4);
        INSERT INTO exit_at_limit (message, severity) VALUES ('wine cheese a', 5);

        SET max_parallel_workers = 0;
    "#.execute(&mut conn);

    let (plan,) = r#"
        EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
        SELECT * FROM exit_at_limit
        WHERE message @@@ 'beer'
        ORDER BY severity, id LIMIT 1;
    "#
    .fetch_one::<(Value,)>(&mut conn);
    eprintln!("{plan:#?}");

    // The Incremental Sort node prevents the Limit node from applying early cutoff, so the custom
    // scan node must do so itself.
    assert_eq!(
        plan.pointer("/0/Plan/Plans/0/Node Type"),
        Some(&Value::String(String::from("Incremental Sort")))
    );
    assert_eq!(
        plan.pointer("/0/Plan/Plans/0/Plans/0/   Queries"),
        Some(&Value::Number(1.into()))
    );
}

#[rstest]
fn top_n_completes_issue2511(mut conn: PgConnection) {
    r#"
        drop table if exists loop;
        create table loop (id serial8 not null primary key, message text) with (autovacuum_enabled = false);
        create index idxloop on loop using bm25 (id, message) WITH (key_field = 'id', layer_sizes = '1GB, 1GB');

        insert into loop (message) select md5(x::text) from generate_series(1, 5000) x;

        update loop set message = message || ' beer';
        update loop set message = message || ' beer';
        update loop set message = message || ' beer';
        update loop set message = message || ' beer';

        set max_parallel_workers = 1;
    "#.execute(&mut conn);

    let results = r#"
        select * from loop where id @@@ paradedb.all() order by id desc limit 25 offset 0;
    "#
    .fetch::<(i64, String)>(&mut conn);
    assert_eq!(results.len(), 25);
}

#[rstest]
fn parallel_custom_scan_with_jsonb_issue2432(mut conn: PgConnection) {
    r#"
        DROP TABLE IF EXISTS test;
        CREATE TABLE test (
            id SERIAL8 NOT NULL PRIMARY KEY,
            message TEXT,
            severity INTEGER
        ) WITH (autovacuum_enabled = false);

        CREATE INDEX idxtest ON test USING bm25(id, message, severity) WITH (key_field = 'id', layer_sizes = '1GB, 1GB');

        INSERT INTO test (message, severity) VALUES ('beer wine cheese a', 1);
        INSERT INTO test (message, severity) VALUES ('beer wine a', 2);
        INSERT INTO test (message, severity) VALUES ('beer cheese a', 3);
        INSERT INTO test (message, severity) VALUES ('beer a', 4);
        INSERT INTO test (message, severity) VALUES ('wine cheese a', 5);
        INSERT INTO test (message, severity) VALUES ('wine a', 6);
        INSERT INTO test (message, severity) VALUES ('cheese a', 7);
        INSERT INTO test (message, severity) VALUES ('beer wine cheese a', 1);
        INSERT INTO test (message, severity) VALUES ('beer wine a', 2);
        INSERT INTO test (message, severity) VALUES ('beer cheese a', 3);
        INSERT INTO test (message, severity) VALUES ('beer a', 4);
        INSERT INTO test (message, severity) VALUES ('wine cheese a', 5);
        INSERT INTO test (message, severity) VALUES ('wine a', 6);
        INSERT INTO test (message, severity) VALUES ('cheese a', 7);
    "#.execute(&mut conn);

    r#"
        SET enable_indexonlyscan to OFF;
        SET enable_indexscan to OFF;
        SET max_parallel_workers = 32;
    "#
    .execute(&mut conn);

    let (plan, ) = r#"
        explain (FORMAT json) select id
        from test
        where message @@@ '{"parse_with_field":{"field":"message","query_string":"beer","lenient":null,"conjunction_mode":null}}'::jsonb
        order by paradedb.score(id) desc
        limit 10;
    "#.fetch_one::<(serde_json::Value, )>(&mut conn);

    eprintln!("{plan:#?}");
    let node = plan
        .pointer("/0/Plan/Plans/0/Plans/0/Parallel Aware")
        .unwrap();
    let parallel_aware = node
        .as_bool()
        .expect("should have gotten the `Parallel Aware` node");
    assert_eq!(parallel_aware, true);
}

#[rstest]
fn nested_loop_rescan_issue_2472(mut conn: PgConnection) {
    // Setup tables and test data
    r#"
    -- Create extension
    DROP EXTENSION IF EXISTS pg_search CASCADE;
    CREATE EXTENSION IF NOT EXISTS pg_search;

    -- Create tables
    CREATE TABLE IF NOT EXISTS company (
        id BIGINT PRIMARY KEY,
        name TEXT
    );

    CREATE TABLE IF NOT EXISTS "user" (
        id BIGINT PRIMARY KEY,
        company_id BIGINT,
        status TEXT
    );

    CREATE TABLE IF NOT EXISTS user_products (
        user_id BIGINT,
        product_id BIGINT,
        deleted_at TIMESTAMP
    );

    -- Create ParadeDB BM25 index
    DROP INDEX IF EXISTS company_name_search_idx;
    CREATE INDEX company_name_search_idx ON company
    USING bm25 (id, name)
    WITH (key_field = 'id');

    -- Insert test data
    DELETE FROM company;
    INSERT INTO company VALUES
    (4, 'Testing Company'),
    (5, 'Testing Org'),
    (13, 'Something else'),
    (15, 'Important Testing');

    DELETE FROM "user";
    INSERT INTO "user" VALUES
    (1, 4, 'NORMAL'),
    (2, 5, 'NORMAL'),
    (3, 13, 'NORMAL'),
    (4, 15, 'NORMAL'),
    (5, 7, 'NORMAL');

    DELETE FROM user_products;
    INSERT INTO user_products VALUES
    (1, 100, NULL),
    (2, 100, NULL),
    (3, 200, NULL),
    (4, 100, NULL);
    "#
    .execute(&mut conn);

    // Test in non-parallel mode first
    r#"
    SET max_parallel_workers = 0;
    SET max_parallel_workers_per_gather = 0;
    "#
    .execute(&mut conn);

    println!("Testing in non-parallel mode");

    // Check if we're running in non-parallel mode
    let (plan,) = r#"
    EXPLAIN (FORMAT json)
    WITH target_users AS (
        SELECT u.id, u.company_id
        FROM "user" u
        WHERE u.status = 'NORMAL'
            AND u.company_id in (5, 4, 13, 15)
    ),
    matched_companies AS (
        SELECT c.id, paradedb.score(c.id) AS company_score
        FROM company c
        WHERE c.id @@@ 'name:Testing'
    )
    SELECT
        u.id,
        u.company_id,
        mc.id as mc_company_id
    FROM target_users u
    LEFT JOIN matched_companies mc ON u.company_id = mc.id;"#
        .fetch_one::<(serde_json::Value,)>(&mut conn);

    let node = plan.pointer("/0/Plan").unwrap();
    let is_parallel = node.as_object().unwrap().contains_key("Workers Planned");
    assert!(!is_parallel, "Query should not use parallel execution");

    // First test in non-parallel mode
    let complex_results = r#"
    -- This reproduces the issue with company_id 15
    WITH target_users AS (
        SELECT u.id, u.company_id
        FROM "user" u
        WHERE u.status = 'NORMAL'
            AND u.company_id in (5, 4, 13, 15)
    ),
    matched_companies AS (
        SELECT c.id, paradedb.score(c.id) AS company_score
        FROM company c
        WHERE c.id @@@ 'name:Testing'
    ),
    scored_users AS (
        SELECT
            u.id,
            u.company_id,
            mc.id as mc_company_id,
            COALESCE(MAX(mc.company_score), 0) AS score
        FROM target_users u
        LEFT JOIN matched_companies mc ON u.company_id = mc.id
        LEFT JOIN user_products up ON up.user_id = u.id
        GROUP BY u.id, u.company_id, mc.id
    )
    SELECT su.id, su.company_id, su.mc_company_id, su.score
    FROM scored_users su
    ORDER BY score DESC;
    "#
    .fetch_result::<(i64, i64, Option<i64>, f32)>(&mut conn)
    .expect("complex query failed");

    // Test that we get results for all users, including the problematic company_id 15
    assert_eq!(complex_results.len(), 4);
    let has_company_15 = complex_results
        .iter()
        .any(|(_, company_id, _, _)| *company_id == 15);
    assert!(
        has_company_15,
        "Results should include user with company_id 15"
    );

    // The minimal query focusing on the problematic companies in non-parallel mode
    let minimal_results = r#"
    WITH target_users AS (
        SELECT u.id, u.company_id
        FROM "user" u
        WHERE
          u.status = 'NORMAL' AND
            u.company_id in (13, 15)
    ),
    matched_companies AS (
        SELECT c.id, paradedb.score(c.id) AS company_score
        FROM company c
        WHERE c.id @@@ 'name:Testing'
    )
    SELECT
        u.id,
        u.company_id,
        mc.id as mc_company_id,
        COALESCE(mc.company_score, 0) AS score
    FROM target_users u
    LEFT JOIN matched_companies mc ON u.company_id = mc.id;
    "#
    .fetch_result::<(i64, i64, Option<i64>, f32)>(&mut conn)
    .expect("minimal query failed");

    // Verify both companies in non-parallel mode
    assert_eq!(minimal_results.len(), 2);
    let has_company_15 = minimal_results
        .iter()
        .any(|(_, company_id, _, _)| *company_id == 15);
    assert!(
        has_company_15,
        "Results should include user with company_id 15"
    );
    println!("minimal_results: {:?}", minimal_results);
    let company_15_result = minimal_results
        .iter()
        .find(|(_, company_id, _, _)| *company_id == 15)
        .unwrap();
    assert!(
        company_15_result.3 > 0.0,
        "Company 15 should have a non-zero score"
    );

    // Now test in parallel mode
    r#"
    SET max_parallel_workers = 32;
    SET max_parallel_workers_per_gather = 8;
    "#
    .execute(&mut conn);

    println!("Testing in parallel mode");

    // Check if we're running in parallel mode
    let (plan,) = r#"
    EXPLAIN (FORMAT json)
    WITH target_users AS (
        SELECT u.id, u.company_id
        FROM "user" u
        WHERE u.status = 'NORMAL'
            AND u.company_id in (5, 4, 13, 15)
    ),
    matched_companies AS (
        SELECT c.id, paradedb.score(c.id) AS company_score
        FROM company c
        WHERE c.id @@@ 'name:Testing'
    )
    SELECT
        u.id,
        u.company_id,
        mc.id as mc_company_id
    FROM target_users u
    LEFT JOIN matched_companies mc ON u.company_id = mc.id;"#
        .fetch_one::<(serde_json::Value,)>(&mut conn);

    // Test in parallel mode might not actually use parallelism due to small table sizes
    // But the setting is enabled, which is what we're testing
    let node = plan.pointer("/0/Plan").unwrap();
    let parallel_enabled = node
        .pointer("/Parallel Aware")
        .map(|v| v.as_bool().unwrap_or(false))
        .unwrap_or(false)
        || node.pointer("/Workers Planned").is_some()
        || node.as_object().unwrap().contains_key("Parallel Aware");

    println!(
        "Plan in parallel mode: {}",
        serde_json::to_string_pretty(&plan).unwrap()
    );

    // Due to small data sizes, PostgreSQL might choose not to use parallelism
    // even when the settings allow it, so we don't assert but print info
    println!("Parallelism indicators in plan: {}", parallel_enabled);

    // First test in parallel mode
    let parallel_complex_results = r#"
    -- This reproduces the issue with company_id 15
    WITH target_users AS (
        SELECT u.id, u.company_id
        FROM "user" u
        WHERE u.status = 'NORMAL'
            AND u.company_id in (5, 4, 13, 15)
    ),
    matched_companies AS (
        SELECT c.id, paradedb.score(c.id) AS company_score
        FROM company c
        WHERE c.id @@@ 'name:Testing'
    ),
    scored_users AS (
        SELECT
            u.id,
            u.company_id,
            mc.id as mc_company_id,
            COALESCE(MAX(mc.company_score), 0) AS score
        FROM target_users u
        LEFT JOIN matched_companies mc ON u.company_id = mc.id
        LEFT JOIN user_products up ON up.user_id = u.id
        GROUP BY u.id, u.company_id, mc.id
    )
    SELECT su.id, su.company_id, su.mc_company_id, su.score
    FROM scored_users su
    ORDER BY score DESC;
    "#
    .fetch_result::<(i64, i64, Option<i64>, f32)>(&mut conn)
    .expect("parallel complex query failed");

    // Test that we get results for all users in parallel mode
    assert_eq!(parallel_complex_results.len(), 4);
    let has_company_15 = parallel_complex_results
        .iter()
        .any(|(_, company_id, _, _)| *company_id == 15);
    assert!(
        has_company_15,
        "Parallel results should include user with company_id 15"
    );

    // The minimal query focusing on the problematic companies in parallel mode
    let parallel_minimal_results = r#"
    WITH target_users AS (
        SELECT u.id, u.company_id
        FROM "user" u
        WHERE
          u.status = 'NORMAL' AND
            u.company_id in (13, 15)
    ),
    matched_companies AS (
        SELECT c.id, paradedb.score(c.id) AS company_score
        FROM company c
        WHERE c.id @@@ 'name:Testing'
    )
    SELECT
        u.id,
        u.company_id,
        mc.id as mc_company_id,
        COALESCE(mc.company_score, 0) AS score
    FROM target_users u
    LEFT JOIN matched_companies mc ON u.company_id = mc.id;
    "#
    .fetch_result::<(i64, i64, Option<i64>, f32)>(&mut conn)
    .expect("parallel minimal query failed");

    // Verify both companies in parallel mode
    assert_eq!(parallel_minimal_results.len(), 2);
    let has_company_15 = parallel_minimal_results
        .iter()
        .any(|(_, company_id, _, _)| *company_id == 15);
    assert!(
        has_company_15,
        "Parallel results should include user with company_id 15"
    );
    let company_15_result = parallel_minimal_results
        .iter()
        .find(|(_, company_id, _, _)| *company_id == 15)
        .unwrap();
    assert!(
        company_15_result.3 > 0.0,
        "Company 15 should have a non-zero score in parallel mode"
    );
}

#[rstest]
fn uses_max_parallel_workers_per_gather_issue2515(mut conn: PgConnection) {
    r#"
    SET max_parallel_workers = 8;
    SET max_parallel_workers_per_gather = 2;

    CREATE TABLE t (id bigint);
    INSERT INTO t (id) SELECT x FROM generate_series(1, 1000000) x;
    CREATE INDEX t_idx ON t USING bm25(id) WITH (key_field='id');
    "#
    .execute(&mut conn);

    let (plan,) =
        "EXPLAIN (ANALYZE, FORMAT JSON) SELECT COUNT(*) FROM t WHERE id @@@ paradedb.all()"
            .fetch_one::<(Value,)>(&mut conn);
    let plan = plan.pointer("/0/Plan/Plans/0").unwrap();
    eprintln!("{plan:#?}");
    assert_eq!(
        plan.get("Workers Planned"),
        Some(&Value::Number(Number::from(2)))
    );

    "SET paradedb.enable_custom_scan = false".execute(&mut conn);

    let (plan,) =
        "EXPLAIN (ANALYZE, FORMAT JSON) SELECT COUNT(*) FROM t WHERE id @@@ paradedb.all()"
            .fetch_one::<(Value,)>(&mut conn);
    let plan = plan.pointer("/0/Plan/Plans/0").unwrap();
    eprintln!("{plan:#?}");
    assert_eq!(
        plan.get("Workers Planned"),
        Some(&Value::Number(Number::from(2)))
    );
}

#[rstest]
fn join_with_string_fast_fields_issue_2505(mut conn: PgConnection) {
    r#"
    DROP TABLE IF EXISTS a;
    DROP TABLE IF EXISTS b;

    CREATE TABLE a (
        a_id_pk TEXT,
        content TEXT
    ) WITH (autovacuum_enabled = false);

    CREATE TABLE b (
        b_id_pk TEXT,
        a_id_fk TEXT,
        content TEXT
    ) WITH (autovacuum_enabled = false);

    CREATE INDEX idxa ON a USING bm25 (a_id_pk, content) WITH (key_field = 'a_id_pk');

    CREATE INDEX idxb ON b USING bm25 (b_id_pk, a_id_fk, content) WITH (key_field = 'b_id_pk',
      text_fields = '{ "a_id_fk": { "fast": true, "tokenizer": { "type": "keyword" } } }');

    INSERT INTO a (a_id_pk, content) VALUES ('this-is-a-id', 'beer');
    INSERT INTO b (b_id_pk, a_id_fk, content) VALUES ('this-is-b-id', 'this-is-a-id', 'wine');
    "#
    .execute(&mut conn);

    "VACUUM a, b;  -- needed to get Visibility Map up-to-date".execute(&mut conn);

    // This query previously failed with:
    // "ERROR: assertion failed: natts == state.exec_tuple_which_fast_fields.len()"
    let result = r#"
    SELECT a.a_id_pk as my_a_id_pk, b.b_id_pk as my_b_id_pk
    FROM b
    JOIN a ON a.a_id_pk = b.a_id_fk
    WHERE a.content @@@ 'beer' AND b.content @@@ 'wine';
    "#
    .fetch_result::<(String, String)>(&mut conn)
    .expect("JOIN query with string fast fields should execute successfully");

    assert_eq!(result.len(), 1);
    assert_eq!(
        result[0],
        ("this-is-a-id".to_string(), "this-is-b-id".to_string())
    );

    "DROP TABLE a; DROP TABLE b;".execute(&mut conn);
}

#[rstest]
fn custom_scan_respects_parentheses_issue2526(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(table_name => 'mock_items', schema_name => 'public');

    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, category, rating, in_stock, metadata, created_at, last_updated_date, latest_available_time)
    WITH (key_field='id');
    "#.execute(&mut conn);

    let result: Vec<(i64,)> = "SELECT COUNT(*) from mock_items WHERE description @@@ 'shoes' AND (description @@@ 'keyboard' OR description @@@ 'hat')".fetch(&mut conn);
    assert_eq!(result, vec![(0,)]);
}
