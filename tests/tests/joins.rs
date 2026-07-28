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

use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;
use tests::fixtures::*;

#[rstest]
fn joins_return_correct_results(mut conn: PgConnection) -> Result<(), sqlx::Error> {
    r#"
    DROP TABLE IF EXISTS a;
    DROP TABLE IF EXISTS b;
    CREATE TABLE a (
        id bigint,
        value text
    );
    CREATE TABLE b (
        id bigint,
        value text
    );
    
    INSERT INTO public.a VALUES (1, 'beer wine');
    INSERT INTO public.a VALUES (2, 'beer wine');
    INSERT INTO public.a VALUES (3, 'cheese');
    INSERT INTO public.a VALUES (4, 'food stuff');
    INSERT INTO public.a VALUES (5, 'only_in_a');

    INSERT INTO public.b VALUES (1, 'beer');
    INSERT INTO public.b VALUES (2, 'wine');
    INSERT INTO public.b VALUES (3, 'cheese');
    INSERT INTO public.b VALUES (4, 'wine beer cheese');
                            -- mind the gap
    INSERT INTO public.b VALUES (6, 'only_in_b');

-- loading all this extra data makes the test take too long on CI
--    INSERT INTO a (id, value) SELECT x, md5(random()::text) FROM generate_series(7, 10000) x;
--    INSERT INTO b (id, value) SELECT x, md5(random()::text) FROM generate_series(7, 10000) x;
        
    CREATE INDEX idxa ON public.a USING bm25 (id, value) WITH (key_field=id, text_fields='{"value": {}}');
    CREATE INDEX idxb ON public.b USING bm25 (id, value) WITH (key_field=id, text_fields='{"value": {}}');
    "#
        .execute(&mut conn);

    type RowType = (Option<i64>, Option<i64>, Option<String>, Option<String>);
    // the pg_search queries also ORDER BY pdb.score() to ensure we get a paradedb CustomScan
    let queries = [
        [
            "select a.id, b.id, a.value a, b.value b from a left join b on a.id = b.id where a.value @@@   'beer'   or b.value @@@   'wine'   or a.value @@@ 'only_in_a' or b.value @@@ 'only_in_b' order by a.id, b.id, pdb.score(a.id), pdb.score(b.id);",
            "select a.id, b.id, a.value a, b.value b from a left join b on a.id = b.id where a.value ilike '%beer%' or b.value ilike '%wine%' or a.value   = 'only_in_a' or b.value @@@ 'only_in_b' order by a.id, b.id;",
        ],
        [
            "select a.id, b.id, a.value a, b.value b from a right join b on a.id = b.id where a.value @@@   'beer'   or b.value @@@   'wine'   or a.value @@@ 'only_in_a' or b.value @@@ 'only_in_b' order by a.id, b.id, pdb.score(a.id), pdb.score(b.id);",
            "select a.id, b.id, a.value a, b.value b from a right join b on a.id = b.id where a.value ilike '%beer%' or b.value ilike '%wine%' or a.value   = 'only_in_a' or b.value @@@ 'only_in_b' order by a.id, b.id;",
        ],
        [
            "select a.id, b.id, a.value a, b.value b from a inner join b on a.id = b.id where a.value @@@   'beer'   or b.value @@@   'wine'   or a.value @@@ 'only_in_a' or b.value @@@ 'only_in_b' order by a.id, b.id, pdb.score(a.id), pdb.score(b.id);",
            "select a.id, b.id, a.value a, b.value b from a inner join b on a.id = b.id where a.value ilike '%beer%' or b.value ilike '%wine%' or a.value   = 'only_in_a' or b.value @@@ 'only_in_b' order by a.id, b.id;",
        ],
        [
            "select a.id, b.id, a.value a, b.value b from a full join b on a.id = b.id where a.value @@@   'beer'   or b.value @@@   'wine'   or a.value @@@ 'only_in_a' or b.value @@@ 'only_in_b' order by a.id, b.id, pdb.score(a.id), pdb.score(b.id);",
            "select a.id, b.id, a.value a, b.value b from a full join b on a.id = b.id where a.value ilike '%beer%' or b.value ilike '%wine%' or a.value   = 'only_in_a' or b.value @@@ 'only_in_b' order by a.id, b.id;",
        ],
    ];

    for [pg_search, postgres] in queries {
        eprintln!("pg_search: {pg_search:?}");
        eprintln!("postgres: {postgres:?}");

        let (pg_search_plan,) =
            format!("EXPLAIN (ANALYZE, FORMAT JSON) {pg_search}").fetch_one::<(Value,)>(&mut conn);
        eprintln!("pg_search_plan: {pg_search_plan:#?}");
        assert!(format!("{pg_search_plan:?}").contains("ParadeDB Base Scan"));

        let pg_search = pg_search.fetch_result::<RowType>(&mut conn)?;
        let postgres = postgres.fetch_result::<RowType>(&mut conn)?;

        assert_eq!(pg_search, postgres);
    }

    Ok(())
}

#[rstest]
fn snippet_from_join(mut conn: PgConnection) -> Result<(), sqlx::Error> {
    r#"
    CREATE TABLE a (
        id bigint,
        value text
    );
    CREATE TABLE b (
        id bigint,
        value text
    );

    INSERT INTO a (id, value) VALUES (1, 'beer'), (2, 'wine'), (3, 'cheese');
    INSERT INTO b (id, value) VALUES (1, 'beer'), (2, 'wine'), (3, 'cheese');

    CREATE INDEX idxa ON a USING bm25 (id, value) WITH (key_field='id', text_fields='{"value": {}}');
    CREATE INDEX idxb ON b USING bm25 (id, value) WITH (key_field='id', text_fields='{"value": {}}');
    "#
        .execute(&mut conn);

    let (snippet, ) = r#"select pdb.snippet(a.value) from a left join b on a.id = b.id where a.value @@@ 'beer';"#
        .fetch_one::<(String,)>(&mut conn);
    assert_eq!(snippet, String::from("<b>beer</b>"));

    let (snippet, ) = r#"select pdb.snippet(b.value) from a left join b on a.id = b.id where a.value @@@ 'beer' and b.value @@@ 'beer';"#
        .fetch_one::<(String,)>(&mut conn);
    assert_eq!(snippet, String::from("<b>beer</b>"));

    // NB:  the result of this is wrong for now...
    let results = r#"select a.id, b.id, pdb.snippet(a.value), pdb.snippet(b.value) from a left join b on a.id = b.id where a.value @@@ 'beer' or b.value @@@ 'wine' order by a.id, b.id;"#
        .fetch_result::<(i64, i64, Option<String>, Option<String>)>(&mut conn)?;

    // ... this is what we'd actually expect from the above query
    let expected = vec![
        (1, 1, Some(String::from("<b>beer</b>")), None),
        (2, 2, None, Some(String::from("<b>wine</b>"))),
    ];

    assert_eq!(results, expected);

    Ok(())
}

#[rstest]
fn joinscan_self_join_matches_fallback(mut conn: PgConnection) -> Result<(), sqlx::Error> {
    r#"
    SET paradedb.enable_custom_scan = on;
    SET paradedb.enable_join_custom_scan = on;
    SET max_parallel_workers_per_gather = 0;
    SET enable_indexscan = off;

    DROP TABLE IF EXISTS dup_items;
    CREATE TABLE dup_items (
        id integer PRIMARY KEY,
        grp integer,
        body text,
        side text,
        ord text
    );

    INSERT INTO dup_items (id, grp, body, side, ord) VALUES
        (100, 1, 'left token', 'a', 'z100'),
        (200, 1, 'left token', 'a', 'z200'),
        (300, 1, 'left token', 'a', 'z300'),
        (1,   1, 'right token', 'b', 'a001'),
        (2,   1, 'right token', 'b', 'a002'),
        (3,   1, 'right token', 'b', 'a003'),
        (400, 2, 'left token', 'a', 'z400'),
        (500, 2, 'left token', 'a', 'z500'),
        (4,   2, 'right token', 'b', 'a004'),
        (5,   2, 'right token', 'b', 'a005');

    CREATE INDEX dup_items_idx ON dup_items USING bm25 (id, grp, body, ord, side)
    WITH (
        key_field = 'id',
        numeric_fields = '{"grp": {"fast": true}}',
        text_fields = '{"ord": {"fast": true}, "side": {"fast": true}}'
    );

    ANALYZE dup_items;
    "#
    .execute(&mut conn);

    let query = r#"
        SELECT a.ord AS a_ord, b.ord AS b_ord, a.id AS a_id, b.id AS b_id
        FROM dup_items a
        JOIN dup_items b ON a.grp = b.grp
        WHERE a.body @@@ 'left'
          AND b.body @@@ 'right'
        ORDER BY b.ord ASC, a.id ASC
        LIMIT 3
    "#;

    let explain_lines: Vec<String> =
        format!("EXPLAIN (COSTS OFF, VERBOSE) {query}").fetch_scalar(&mut conn);
    let explain = explain_lines.join("\n");

    assert!(
        explain.contains("Custom Scan (ParadeDB Join Scan)"),
        "{explain}"
    );
    // The VisibilityFilterExec is absorbed into SegmentedTopKExec, which now owns MVCC
    // visibility checking, so it no longer appears as a separate node in the plan.
    assert!(!explain.contains("VisibilityFilterExec"), "{explain}");
    assert!(explain.contains("TantivyLookupExec"), "{explain}");
    assert!(explain.contains("SegmentedTopKExec"), "{explain}");

    type Row = (String, String, i32, i32);

    "SET paradedb.enable_join_custom_scan = on;".execute(&mut conn);
    let joinscan_rows = query.fetch_result::<Row>(&mut conn)?;

    "SET paradedb.enable_join_custom_scan = off;".execute(&mut conn);
    let fallback_rows = query.fetch_result::<Row>(&mut conn)?;

    assert_eq!(joinscan_rows, fallback_rows);
    assert_eq!(
        joinscan_rows,
        vec![
            ("z100".into(), "a001".into(), 100, 1),
            ("z200".into(), "a001".into(), 200, 1),
            ("z300".into(), "a001".into(), 300, 1),
        ]
    );

    Ok(())
}

#[rstest]
fn joinscan_self_join_duplicate_name_sort_matches_fallback(
    mut conn: PgConnection,
) -> Result<(), sqlx::Error> {
    r#"
    SET paradedb.enable_custom_scan = on;
    SET paradedb.enable_join_custom_scan = on;
    SET max_parallel_workers_per_gather = 0;
    SET enable_indexscan = off;

    DROP TABLE IF EXISTS dup_items;
    CREATE TABLE dup_items (
        id integer PRIMARY KEY,
        grp integer,
        body text,
        side text,
        ord text
    );

    INSERT INTO dup_items (id, grp, body, side, ord) VALUES
        (100, 1, 'left token', 'a', 'z100'),
        (200, 1, 'left token', 'a', 'z200'),
        (300, 1, 'left token', 'a', 'z300'),
        (1,   1, 'right token', 'b', 'a001'),
        (2,   1, 'right token', 'b', 'a002'),
        (3,   1, 'right token', 'b', 'a003'),
        (400, 2, 'left token', 'a', 'z400'),
        (500, 2, 'left token', 'a', 'z500'),
        (4,   2, 'right token', 'b', 'a004'),
        (5,   2, 'right token', 'b', 'a005');

    CREATE INDEX dup_items_idx ON dup_items USING bm25 (id, grp, body, ord, side)
    WITH (
        key_field = 'id',
        numeric_fields = '{"grp": {"fast": true}}',
        text_fields = '{"ord": {"fast": true}, "side": {"fast": true}}'
    );

    ANALYZE dup_items;
    "#
    .execute(&mut conn);

    let query = r#"
        SELECT a.ord AS a_ord, b.ord AS b_ord, a.id AS a_id, b.id AS b_id
        FROM dup_items a
        JOIN dup_items b ON a.grp = b.grp
        WHERE a.body @@@ 'left'
          AND b.body @@@ 'right'
        ORDER BY a.ord ASC, b.ord ASC
        LIMIT 3
    "#;

    let explain_lines: Vec<String> =
        format!("EXPLAIN (COSTS OFF, VERBOSE) {query}").fetch_scalar(&mut conn);
    let explain = explain_lines.join("\n");

    assert!(
        explain.contains("Custom Scan (ParadeDB Join Scan)"),
        "{explain}"
    );
    // The VisibilityFilterExec is absorbed into SegmentedTopKExec, which now owns MVCC
    // visibility checking, so it no longer appears as a separate node in the plan.
    assert!(!explain.contains("VisibilityFilterExec"), "{explain}");
    assert!(explain.contains("TantivyLookupExec"), "{explain}");
    assert!(explain.contains("SegmentedTopKExec"), "{explain}");
    // Regression guard: both sort keys must appear at distinct physical indices.
    // A single-key collapse would silently return wrong ordering.
    assert!(
        explain.contains("ord@3") && explain.contains("ord@1"),
        "Expected both sort keys at distinct physical indices in plan:\n{explain}"
    );

    type Row = (String, String, i32, i32);

    "SET paradedb.enable_join_custom_scan = on;".execute(&mut conn);
    let joinscan_rows = query.fetch_result::<Row>(&mut conn)?;

    "SET paradedb.enable_join_custom_scan = off;".execute(&mut conn);
    let fallback_rows = query.fetch_result::<Row>(&mut conn)?;

    assert_eq!(joinscan_rows, fallback_rows);
    assert_eq!(
        fallback_rows,
        vec![
            ("z100".into(), "a001".into(), 100, 1),
            ("z100".into(), "a002".into(), 100, 2),
            ("z100".into(), "a003".into(), 100, 3),
        ]
    );

    Ok(())
}

#[rstest]
fn joinscan_cross_table_duplicate_output_name_matches_fallback(
    mut conn: PgConnection,
) -> Result<(), sqlx::Error> {
    r#"
    SET max_parallel_workers_per_gather = 0;
    SET enable_indexscan = off;

    DROP TABLE IF EXISTS misbind_products CASCADE;
    DROP TABLE IF EXISTS misbind_suppliers CASCADE;

    CREATE TABLE misbind_products (
        id INTEGER PRIMARY KEY,
        name TEXT,
        description TEXT,
        supplier_id INTEGER
    );

    CREATE TABLE misbind_suppliers (
        id INTEGER PRIMARY KEY,
        name TEXT,
        info TEXT
    );

    INSERT INTO misbind_products (id, name, description, supplier_id) VALUES
        (1,  'a_item', 'wireless product one',      1),
        (2,  'b_item', 'wireless product two',      2),
        (3,  'c_item', 'wireless product three',    3),
        (4,  'd_item', 'wireless product four',     4),
        (5,  'e_item', 'wireless product five',     5),
        (6,  'f_item', 'wireless product six',      6),
        (7,  'g_item', 'wireless product seven',    7),
        (8,  'h_item', 'wireless product eight',    8),
        (9,  'i_item', 'wireless product nine',     9),
        (10, 'j_item', 'wireless product ten',      10),
        (11, 'k_item', 'wireless product eleven',   11),
        (12, 'l_item', 'wireless product twelve',   12),
        (13, 'm_item', 'wireless product thirteen', 13),
        (14, 'n_item', 'wireless product fourteen', 14),
        (15, 'o_item', 'wireless product fifteen',  15);

    INSERT INTO misbind_suppliers (id, name, info) VALUES
        (1,  'zzz_sup', 'electronics supplier one'),
        (2,  'yyy_sup', 'electronics supplier two'),
        (3,  'xxx_sup', 'electronics supplier three'),
        (4,  'www_sup', 'electronics supplier four'),
        (5,  'vvv_sup', 'electronics supplier five'),
        (6,  'uuu_sup', 'electronics supplier six'),
        (7,  'ttt_sup', 'electronics supplier seven'),
        (8,  'sss_sup', 'electronics supplier eight'),
        (9,  'rrr_sup', 'electronics supplier nine'),
        (10, 'qqq_sup', 'electronics supplier ten'),
        (11, 'ppp_sup', 'electronics supplier eleven'),
        (12, 'ooo_sup', 'electronics supplier twelve'),
        (13, 'nnn_sup', 'electronics supplier thirteen'),
        (14, 'mmm_sup', 'electronics supplier fourteen'),
        (15, 'lll_sup', 'electronics supplier fifteen');

    CREATE INDEX misbind_products_bm25 ON misbind_products
    USING bm25 (id, name, description, supplier_id)
    WITH (
        key_field = 'id',
        text_fields = '{"name": {"fast": true}, "description": {"fast": true}}',
        numeric_fields = '{"supplier_id": {"fast": true}}'
    );

    CREATE INDEX misbind_suppliers_bm25 ON misbind_suppliers
    USING bm25 (id, name, info)
    WITH (
        key_field = 'id',
        text_fields = '{"name": {"fast": true}}'
    );

    ANALYZE misbind_products;
    ANALYZE misbind_suppliers;
    "#
    .execute(&mut conn);

    // Both tables have a column called `name`. Ordering by p.name (a fast field)
    // while s.name is also a fast field exposes the duplicate-name misbinding bug
    // where JoinScan was sorting on the wrong physical column index.
    let query = r#"
        SELECT p.name AS p_name, s.name AS s_name
        FROM misbind_products p
        JOIN misbind_suppliers s ON p.supplier_id = s.id
        WHERE p.description @@@ 'wireless' AND s.info @@@ 'electronics'
        ORDER BY p.name ASC
        LIMIT 10
    "#;

    type Row = (String, String);

    "SET paradedb.enable_custom_scan = on; SET paradedb.enable_join_custom_scan = on; DISCARD PLANS;"
        .execute(&mut conn);
    let joinscan_rows = query.fetch_result::<Row>(&mut conn)?;

    "SET paradedb.enable_custom_scan = off; SET paradedb.enable_join_custom_scan = off; DISCARD PLANS;"
        .execute(&mut conn);
    let fallback_rows = query.fetch_result::<Row>(&mut conn)?;

    assert_eq!(
        joinscan_rows, fallback_rows,
        "JoinScan returned different rows than Postgres fallback -- likely a column misbinding bug"
    );

    r#"
    SET paradedb.enable_custom_scan = on;
    SET paradedb.enable_join_custom_scan = on;
    DROP TABLE IF EXISTS misbind_products CASCADE;
    DROP TABLE IF EXISTS misbind_suppliers CASCADE;
    RESET max_parallel_workers_per_gather;
    RESET enable_indexscan;
    "#
    .execute(&mut conn);

    Ok(())
}

#[rstest]
fn joinscan_nullable_numeric_composite_sort_matches_fallback(
    mut conn: PgConnection,
) -> Result<(), sqlx::Error> {
    // Regression test for https://github.com/paradedb/paradedb/issues/5560:
    // ORDER BY a nullable NUMERIC composite field crashed SegmentedTopKExec with
    // "RowConverter column schema mismatch, expected BinaryView got Utf8View".
    // NULL sort keys were materialized as Utf8View nulls while the NUMERIC
    // column (a Bytes fast field) was declared BinaryView to the RowConverter.
    r#"
    SET paradedb.enable_custom_scan = on;
    SET paradedb.enable_join_custom_scan = on;
    SET max_parallel_workers_per_gather = 0;
    SET enable_hashjoin = off;
    SET enable_mergejoin = off;
    SET enable_nestloop = off;

    DROP TABLE IF EXISTS nn_parent;
    DROP TABLE IF EXISTS nn_child;
    DROP TYPE IF EXISTS nn_ps;
    CREATE TABLE nn_parent (id int PRIMARY KEY, kind text, score_num numeric);
    CREATE TABLE nn_child  (id bigint PRIMARY KEY, parent_id bigint);

    -- deterministic scores (not random()) so Join Scan and fallback results compare equal
    INSERT INTO nn_parent
    SELECT g,
           CASE WHEN g % 3 = 0 THEN 'novel' ELSE 'manga' END,
           CASE WHEN g % 4 = 0 THEN NULL ELSE ((g * 37) % 1000)::numeric / 1000 END
    FROM generate_series(1, 2000) g;
    INSERT INTO nn_child SELECT g, ((g * 7) % 2000) + 1 FROM generate_series(1, 400) g;

    CREATE TYPE nn_ps AS (kind pdb.literal_normalized, score_num numeric);
    CREATE INDEX nn_parent_bm25 ON nn_parent USING bm25 (id, (ROW(kind, score_num)::nn_ps))
    WITH (key_field = 'id');
    CREATE INDEX nn_child_bm25 ON nn_child USING bm25 (id, parent_id)
    WITH (key_field = 'id');
    ANALYZE nn_parent;
    ANALYZE nn_child;
    "#
    .execute(&mut conn);

    // NULLS LAST exercises the crash; NULLS FIRST forces NULL rows into the
    // final top-k so their sort placement is verified too. The p.id tiebreaker
    // is evaluated directly; a deferred second key remains uncovered until #5567.
    for nulls in ["LAST", "FIRST"] {
        let query = format!(
            r#"
            SELECT p.id
            FROM nn_parent p JOIN nn_child c ON c.parent_id = p.id
            WHERE p.kind @@@ pdb.term('manga') AND c.id @@@ pdb.all()
            ORDER BY p.score_num DESC NULLS {nulls}, p.id
            LIMIT 5
            "#
        );

        let explain_lines: Vec<String> =
            format!("EXPLAIN (COSTS OFF, VERBOSE) {query}").fetch_scalar(&mut conn);
        let explain = explain_lines.join("\n");
        assert!(
            explain.contains("Custom Scan (ParadeDB Join Scan)"),
            "{explain}"
        );
        assert!(explain.contains("SegmentedTopKExec"), "{explain}");

        "SET paradedb.enable_join_custom_scan = on; DISCARD PLANS;".execute(&mut conn);
        let joinscan_rows = query.as_str().fetch_result::<(i32,)>(&mut conn)?;

        "SET paradedb.enable_join_custom_scan = off; DISCARD PLANS;".execute(&mut conn);
        let fallback_rows = query.as_str().fetch_result::<(i32,)>(&mut conn)?;

        "SET paradedb.enable_join_custom_scan = on;".execute(&mut conn);

        assert_eq!(joinscan_rows, fallback_rows, "NULLS {nulls}");
        assert_eq!(joinscan_rows.len(), 5, "NULLS {nulls}");
    }

    Ok(())
}
