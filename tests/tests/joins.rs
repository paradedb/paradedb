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

mod fixtures;

use fixtures::*;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;

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
    assert!(explain.contains("VisibilityFilterExec"), "{explain}");
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
#[ignore = "known issue: duplicate-name sort keys above JoinScan can diverge from fallback ordering"]
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
    assert!(explain.contains("VisibilityFilterExec"), "{explain}");
    assert!(explain.contains("TantivyLookupExec"), "{explain}");
    assert!(explain.contains("SegmentedTopKExec"), "{explain}");

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
