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

/// Recursively search an EXPLAIN (FORMAT JSON) plan tree for any node that
/// declares `Workers Planned > 0`. Used to assert that parallelism survived
/// planning — see issue #4665, where GENERIC prepared plans regressed to 0
/// workers because of a selectivity collapse.
fn plan_has_parallel_workers(v: &Value) -> bool {
    match v {
        Value::Object(obj) => {
            if let Some(workers) = obj.get("Workers Planned").and_then(|w| w.as_i64()) {
                if workers > 0 {
                    return true;
                }
            }
            obj.values().any(plan_has_parallel_workers)
        }
        Value::Array(arr) => arr.iter().any(plan_has_parallel_workers),
        _ => false,
    }
}

#[rstest]
fn self_referencing_var(mut conn: PgConnection) {
    r#"
    DROP TABLE IF EXISTS test;
    CREATE TABLE test (
        id bigint NOT NULL PRIMARY KEY,
        value text
    );

    INSERT INTO test (id, value) SELECT x, md5(x::text) FROM generate_series(1, 100) x;
    UPDATE test SET value = 'value contains id = ' || id WHERE id BETWEEN 10 and 20;

    CREATE INDEX idxtest ON test USING bm25 (id, value) WITH (key_field='id');
    "#
    .execute(&mut conn);

    let results =
        "SELECT id FROM test WHERE value @@@ paradedb.with_index('idxtest', paradedb.term('value', id::text)) ORDER BY id;".fetch::<(i64,)>(&mut conn);
    assert_eq!(
        results,
        vec![
            (10,),
            (11,),
            (12,),
            (13,),
            (14,),
            (15,),
            (16,),
            (17,),
            (18,),
            (19,),
            (20,),
        ]
    );
}

#[rstest]
fn parallel_with_subselect(mut conn: PgConnection) {
    if pg_major_version(&mut conn) < 16 {
        // Unstable results without `debug_parallel_query`.
        return;
    }
    "SET debug_parallel_query TO on".execute(&mut conn);

    r#"
    DROP TABLE IF EXISTS test;
    CREATE TABLE test (
        id bigint NOT NULL PRIMARY KEY,
        value text
    );

    INSERT INTO test (id, value) SELECT x, md5(x::text) FROM generate_series(1, 100) x;
    UPDATE test SET value = 'value contains id = ' || id WHERE id BETWEEN 10 and 20;

    CREATE INDEX idxtest ON test USING bm25 (id, value) WITH (key_field='id');
    "#
    .execute(&mut conn);

    "PREPARE foo AS SELECT count(*) FROM test WHERE value @@@ (select $1);".execute(&mut conn);
    let (count,) = "EXECUTE foo('contains')".fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 11);

    // next 4 executions use one plan, and the 5th shouldn't change
    for _ in 0..5 {
        let (plan,) = "EXPLAIN (ANALYZE, FORMAT JSON) EXECUTE foo('contains');"
            .fetch_one::<(Value,)>(&mut conn);
        eprintln!("{plan:#?}");
        let plan = plan
            .pointer("/0/Plan/Plans/1/Plans/0")
            .unwrap()
            .as_object()
            .unwrap();
        pretty_assertions::assert_eq!(
            plan.get("Custom Plan Provider"),
            Some(&Value::String(String::from("ParadeDB Base Scan")))
        );
    }
}

#[rstest]
fn parallel_function_with_agg_subselect(mut conn: PgConnection) {
    r#"
    DROP TABLE IF EXISTS test;
    CREATE TABLE test (
        id bigint NOT NULL PRIMARY KEY,
        value text
    );

    INSERT INTO test (id, value) SELECT x, md5(x::text) FROM generate_series(1, 100) x;
    UPDATE test SET value = 'value contains id = ' || id WHERE id BETWEEN 10 and 20;

    CREATE INDEX idxtest ON test USING bm25 (id, value) WITH (key_field='id');
    "#
    .execute(&mut conn);

    if pg_major_version(&mut conn) >= 16 {
        "SET debug_parallel_query TO on".execute(&mut conn);
    }

    "PREPARE foo AS SELECT id FROM test WHERE id @@@ paradedb.term_set((select array_agg(paradedb.term('value', token)) from paradedb.tokenize(paradedb.tokenizer('default'), $1))) ORDER BY id;".execute(&mut conn);

    let results = "EXECUTE foo('no matches')".fetch::<(i64,)>(&mut conn);
    assert_eq!(results.len(), 0);

    let results = "EXECUTE foo('value contains id')".fetch::<(i64,)>(&mut conn);
    assert_eq!(
        results,
        vec![
            (10,),
            (11,),
            (12,),
            (13,),
            (14,),
            (15,),
            (16,),
            (17,),
            (18,),
            (19,),
            (20,),
        ]
    );
}

#[rstest]
fn test_issue2061(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    )
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX search_idx ON mock_items
    USING bm25 (id, description, category, rating, in_stock, created_at, metadata, weight_range)
    WITH (key_field='id');
    "#
    .execute(&mut conn);

    let results = r#"
    SELECT id, description, pdb.score(id)
    FROM mock_items
    WHERE id @@@ paradedb.match('description', (SELECT description FROM mock_items WHERE id = 1))
    ORDER BY pdb.score(id) DESC;
    "#
    .fetch::<(i32, String, f32)>(&mut conn);

    assert_eq!(
        results,
        vec![
            (1, "Ergonomic metal keyboard".into(), 9.485788),
            (2, "Plastic Keyboard".into(), 3.2668595),
        ]
    )
}

/// Issue #4665: CUSTOM and GENERIC prepared plans must return the same rows
/// AND retain parallelism when the WHERE clause uses a parameterized BM25
/// search predicate. Plan-shape check guards against the selectivity
/// regression that collapsed GENERIC row estimates to 0 workers.
#[rstest]
fn generic_plan_consistent_results_issue_4665(mut conn: PgConnection) {
    if pg_major_version(&mut conn) < 16 {
        // `debug_parallel_query` is only available from PG16.
        return;
    }
    "SET debug_parallel_query TO on".execute(&mut conn);

    r#"
    CREATE TABLE issue_4665 (
        id SERIAL PRIMARY KEY,
        content TEXT
    );

    INSERT INTO issue_4665 (content)
    SELECT 'document about ' ||
           (ARRAY['technology', 'science', 'cooking', 'sports'])[1 + (i % 4)]
           || ' number ' || i
    FROM generate_series(1, 200) AS i;

    CREATE INDEX issue_4665_idx ON issue_4665
    USING bm25 (id, content) WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    // Get results + plan with CUSTOM plan (constant is visible to planner)
    "SET plan_cache_mode = force_custom_plan".execute(&mut conn);
    "PREPARE stmt_custom(text) AS
     SELECT id FROM issue_4665
     WHERE content ||| $1
     ORDER BY pdb.score(id) DESC
     LIMIT 10"
        .execute(&mut conn);
    let custom_results = "EXECUTE stmt_custom('technology')".fetch::<(i32,)>(&mut conn);
    let (custom_plan,) = "EXPLAIN (ANALYZE, FORMAT JSON) EXECUTE stmt_custom('technology')"
        .fetch_one::<(Value,)>(&mut conn);
    "DEALLOCATE stmt_custom".execute(&mut conn);

    // Get results + plan with GENERIC plan (Param node, not Const)
    "SET plan_cache_mode = force_generic_plan".execute(&mut conn);
    "PREPARE stmt_generic(text) AS
     SELECT id FROM issue_4665
     WHERE content ||| $1
     ORDER BY pdb.score(id) DESC
     LIMIT 10"
        .execute(&mut conn);
    let generic_results = "EXECUTE stmt_generic('technology')".fetch::<(i32,)>(&mut conn);
    let (generic_plan,) = "EXPLAIN (ANALYZE, FORMAT JSON) EXECUTE stmt_generic('technology')"
        .fetch_one::<(Value,)>(&mut conn);
    "DEALLOCATE stmt_generic".execute(&mut conn);

    assert_eq!(
        custom_results, generic_results,
        "CUSTOM and GENERIC plans must return identical rows for parameterized WHERE"
    );
    assert!(!custom_results.is_empty(), "should have matches");

    assert!(
        plan_has_parallel_workers(&custom_plan),
        "CUSTOM plan should have Workers Planned > 0: {custom_plan:#?}"
    );
    assert!(
        plan_has_parallel_workers(&generic_plan),
        "GENERIC plan should have Workers Planned > 0 (issue #4665): {generic_plan:#?}"
    );

    "RESET plan_cache_mode".execute(&mut conn);
}

/// Issue #4665 follow-up: Parameterized LIMIT must produce the same results
/// as a constant LIMIT in both CUSTOM and GENERIC modes.
#[rstest]
fn generic_plan_parameterized_limit_issue_4665(mut conn: PgConnection) {
    r#"
    CREATE TABLE issue_4665_plim (
        id SERIAL PRIMARY KEY,
        content TEXT
    );

    INSERT INTO issue_4665_plim (content)
    SELECT 'document about ' ||
           (ARRAY['technology', 'science', 'cooking', 'sports'])[1 + (i % 4)]
           || ' number ' || i
    FROM generate_series(1, 200) AS i;

    CREATE INDEX issue_4665_plim_idx ON issue_4665_plim
    USING bm25 (id, content) WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    // Baseline: constant LIMIT
    let baseline = "SELECT id FROM issue_4665_plim
                    WHERE content ||| 'technology'
                    ORDER BY pdb.score(id) DESC
                    LIMIT 5"
        .fetch::<(i32,)>(&mut conn);

    // CUSTOM plan with parameterized LIMIT
    "SET plan_cache_mode = force_custom_plan".execute(&mut conn);
    "PREPARE stmt_plim_c(text, int) AS
     SELECT id FROM issue_4665_plim
     WHERE content ||| $1
     ORDER BY pdb.score(id) DESC
     LIMIT $2"
        .execute(&mut conn);
    let custom_results = "EXECUTE stmt_plim_c('technology', 5)".fetch::<(i32,)>(&mut conn);
    "DEALLOCATE stmt_plim_c".execute(&mut conn);

    // GENERIC plan with parameterized LIMIT
    "SET plan_cache_mode = force_generic_plan".execute(&mut conn);
    "PREPARE stmt_plim_g(text, int) AS
     SELECT id FROM issue_4665_plim
     WHERE content ||| $1
     ORDER BY pdb.score(id) DESC
     LIMIT $2"
        .execute(&mut conn);
    let generic_results = "EXECUTE stmt_plim_g('technology', 5)".fetch::<(i32,)>(&mut conn);
    "DEALLOCATE stmt_plim_g".execute(&mut conn);

    assert_eq!(
        custom_results, baseline,
        "CUSTOM plan with parameterized LIMIT must match constant LIMIT baseline"
    );
    assert_eq!(
        generic_results, baseline,
        "GENERIC plan with parameterized LIMIT must match constant LIMIT baseline"
    );

    "RESET plan_cache_mode".execute(&mut conn);
}

/// Issue #4665: The natural CUSTOM→GENERIC transition (after 5 executions)
/// must not change result correctness AND must retain parallel workers in
/// the GENERIC plan.
#[rstest]
fn generic_plan_natural_transition_issue_4665(mut conn: PgConnection) {
    if pg_major_version(&mut conn) < 16 {
        // `debug_parallel_query` is only available from PG16.
        return;
    }
    "SET debug_parallel_query TO on".execute(&mut conn);

    r#"
    CREATE TABLE issue_4665_nat (
        id SERIAL PRIMARY KEY,
        content TEXT
    );

    INSERT INTO issue_4665_nat (content)
    SELECT 'document about ' ||
           (ARRAY['technology', 'science', 'cooking', 'sports'])[1 + (i % 4)]
           || ' number ' || i
    FROM generate_series(1, 200) AS i;

    CREATE INDEX issue_4665_nat_idx ON issue_4665_nat
    USING bm25 (id, content) WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    "PREPARE stmt_nat(text) AS
     SELECT id FROM issue_4665_nat
     WHERE content ||| $1
     ORDER BY pdb.score(id) DESC
     LIMIT 10"
        .execute(&mut conn);

    // First execution captures expected results (CUSTOM plan)
    let expected = "EXECUTE stmt_nat('technology')".fetch::<(i32,)>(&mut conn);
    assert!(!expected.is_empty(), "should have matches");

    // Execute 6 more times — PostgreSQL switches to GENERIC around execution 6
    for i in 0..6 {
        let results = "EXECUTE stmt_nat('technology')".fetch::<(i32,)>(&mut conn);
        assert_eq!(
            results,
            expected,
            "execution {} must match first execution results",
            i + 2
        );
    }

    // After the natural transition to GENERIC, the plan must still be parallel.
    let (plan,) = "EXPLAIN (ANALYZE, FORMAT JSON) EXECUTE stmt_nat('technology')"
        .fetch_one::<(Value,)>(&mut conn);
    assert!(
        plan_has_parallel_workers(&plan),
        "post-transition GENERIC plan should have Workers Planned > 0 (issue #4665): {plan:#?}"
    );

    "DEALLOCATE stmt_nat".execute(&mut conn);
}

/// Parameterized OFFSET must produce correct results in GENERIC mode.
///
/// Pre-fix: GENERIC TopK fetched K=LIMIT (ignoring OFFSET), so PG's outer
/// `Limit OFFSET` skipped too many rows. With OFFSET > LIMIT the bug
/// returned **0 rows** unambiguously — chosen here so partial results
/// can't mask the regression.
#[rstest]
#[case::const_limit_param_offset("text, int", "LIMIT 3 OFFSET $2", "'technology', 7")]
#[case::param_limit_const_offset("text, int", "LIMIT $2 OFFSET 7", "'technology', 3")]
#[case::param_limit_param_offset("text, int, int", "LIMIT $2 OFFSET $3", "'technology', 3, 7")]
fn generic_plan_parameterized_offset(
    mut conn: PgConnection,
    #[case] param_types: &str,
    #[case] limit_clause: &str,
    #[case] execute_args: &str,
) {
    r#"
    DROP TABLE IF EXISTS param_offset_test CASCADE;
    CREATE TABLE param_offset_test (
        id SERIAL PRIMARY KEY,
        content TEXT
    );
    INSERT INTO param_offset_test (content)
    SELECT 'document about technology number ' || i
    FROM generate_series(1, 200) AS i;
    CREATE INDEX param_offset_idx ON param_offset_test
    USING bm25 (id, content) WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    let baseline = "SELECT id FROM param_offset_test
                    WHERE content ||| 'technology'
                    ORDER BY pdb.score(id) DESC
                    LIMIT 3 OFFSET 7"
        .fetch::<(i32,)>(&mut conn);
    assert_eq!(baseline.len(), 3, "baseline should return 3 rows");

    let prepare_template = format!(
        "SELECT id FROM param_offset_test WHERE content ||| $1
         ORDER BY pdb.score(id) DESC {limit_clause}"
    );

    "SET plan_cache_mode = force_custom_plan".execute(&mut conn);
    format!("PREPARE off_c({param_types}) AS {prepare_template}").execute(&mut conn);
    let custom = format!("EXECUTE off_c({execute_args})").fetch::<(i32,)>(&mut conn);
    "DEALLOCATE off_c".execute(&mut conn);

    "SET plan_cache_mode = force_generic_plan".execute(&mut conn);
    format!("PREPARE off_g({param_types}) AS {prepare_template}").execute(&mut conn);
    let generic = format!("EXECUTE off_g({execute_args})").fetch::<(i32,)>(&mut conn);
    "DEALLOCATE off_g".execute(&mut conn);

    assert_eq!(custom, baseline, "CUSTOM must match baseline");
    assert_eq!(generic, baseline, "GENERIC must match baseline");

    "RESET plan_cache_mode".execute(&mut conn);
}

/// JoinScan must survive parameterized LIMIT/OFFSET (previously disabled
/// with NOTICE: "JoinScan not used: activation checks failed (LIMIT / ...)").
#[rstest]
#[case::param_limit_only("text, int", "LIMIT $2", "'electronics', 10", 10)]
#[case::param_limit_param_offset(
    "text, int, int",
    "LIMIT $2 OFFSET $3",
    "'electronics', 10, 0",
    10
)]
fn joinscan_survives_parameterized_limit(
    mut conn: PgConnection,
    #[case] param_types: &str,
    #[case] limit_clause: &str,
    #[case] execute_args: &str,
    #[case] expected_rows: usize,
) {
    "SET paradedb.enable_join_custom_scan = on".execute(&mut conn);
    "SET max_parallel_workers_per_gather = 0".execute(&mut conn);
    "SET enable_indexscan TO OFF".execute(&mut conn);

    r#"
    DROP TABLE IF EXISTS js_prods CASCADE;
    DROP TABLE IF EXISTS js_cats CASCADE;
    CREATE TABLE js_prods (
        id SERIAL PRIMARY KEY,
        name TEXT,
        cat_id INT
    );
    CREATE TABLE js_cats (
        id SERIAL PRIMARY KEY,
        label TEXT
    );
    INSERT INTO js_cats (label) VALUES ('electronics'), ('clothing'), ('food');
    INSERT INTO js_prods (name, cat_id)
    SELECT 'product ' || i || ' in electronics', 1 + (i % 3)
    FROM generate_series(1, 100) AS i;
    CREATE INDEX js_prods_idx ON js_prods
    USING bm25 (id, name, cat_id) WITH (key_field = 'id');
    CREATE INDEX js_cats_idx ON js_cats
    USING bm25 (id, label) WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    let prepare_template = format!(
        "SELECT p.id, p.name FROM js_prods p
         JOIN js_cats c ON p.cat_id = c.id
         WHERE p.name ||| $1
         ORDER BY pdb.score(p.id) DESC
         {limit_clause}"
    );

    "SET plan_cache_mode = force_custom_plan".execute(&mut conn);
    format!("PREPARE js_c({param_types}) AS {prepare_template}").execute(&mut conn);
    let mut custom_results =
        format!("EXECUTE js_c({execute_args})").fetch::<(i32, String)>(&mut conn);
    "DEALLOCATE js_c".execute(&mut conn);

    "SET plan_cache_mode = force_generic_plan".execute(&mut conn);
    format!("PREPARE js_g({param_types}) AS {prepare_template}").execute(&mut conn);
    let mut generic_results =
        format!("EXECUTE js_g({execute_args})").fetch::<(i32, String)>(&mut conn);

    // All `electronics` matches share identical scores, so the ORDER BY tie
    // is unstable. Compare the row set, not the order.
    custom_results.sort_by_key(|r| r.0);
    generic_results.sort_by_key(|r| r.0);
    assert_eq!(
        custom_results, generic_results,
        "JoinScan with param LIMIT must return the same set of rows in both modes"
    );
    assert_eq!(
        custom_results.len(),
        expected_rows,
        "LIMIT must be respected"
    );

    // GENERIC plan must actually use JoinScan (not fall back to NestedLoop).
    let (plan,) = format!("EXPLAIN (FORMAT JSON) EXECUTE js_g({execute_args})")
        .fetch_one::<(Value,)>(&mut conn);
    let plan_text = format!("{plan:#}");
    assert!(
        plan_text.contains("ParadeDB Join Scan"),
        "GENERIC mode with param LIMIT must keep JoinScan: {plan_text}"
    );
    "DEALLOCATE js_g".execute(&mut conn);

    "RESET plan_cache_mode".execute(&mut conn);
    "RESET paradedb.enable_join_custom_scan".execute(&mut conn);
    "RESET max_parallel_workers_per_gather".execute(&mut conn);
    "RESET enable_indexscan".execute(&mut conn);
}

/// Snippet functions must not panic when formatting arguments are
/// parameterized in GENERIC plan mode. Pre-fix: panicked with
/// "pdb.snippets()'s arguments must be literals" on the 6th execution.
#[rstest]
fn snippet_with_parameterized_args(mut conn: PgConnection) {
    r#"
    DROP TABLE IF EXISTS snippet_param_test CASCADE;
    CREATE TABLE snippet_param_test (
        id SERIAL PRIMARY KEY,
        content TEXT
    );
    INSERT INTO snippet_param_test (content) VALUES
    ('the quick brown fox jumps over the lazy dog'),
    ('a technology document about computers and technology advances'),
    ('science is great for learning new things about the world');
    CREATE INDEX snippet_param_idx ON snippet_param_test
    USING bm25 (id, content) WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    // Baseline: constant args
    let baseline = r#"
    SELECT id, pdb.snippet(content, '<b>', '</b>')
    FROM snippet_param_test
    WHERE content ||| 'technology'
    ORDER BY pdb.score(id) DESC
    "#
    .fetch::<(i32, String)>(&mut conn);
    assert!(!baseline.is_empty(), "baseline should have matches");

    // CUSTOM plan with parameterized start/end tags
    "SET plan_cache_mode = force_custom_plan".execute(&mut conn);
    "PREPARE snip_c(text, text, text) AS
     SELECT id, pdb.snippet(content, $2, $3)
     FROM snippet_param_test
     WHERE content ||| $1
     ORDER BY pdb.score(id) DESC"
        .execute(&mut conn);
    let custom = "EXECUTE snip_c('technology', '<b>', '</b>')".fetch::<(i32, String)>(&mut conn);
    "DEALLOCATE snip_c".execute(&mut conn);

    // GENERIC plan — this was panicking before the fix
    "SET plan_cache_mode = force_generic_plan".execute(&mut conn);
    "PREPARE snip_g(text, text, text) AS
     SELECT id, pdb.snippet(content, $2, $3)
     FROM snippet_param_test
     WHERE content ||| $1
     ORDER BY pdb.score(id) DESC"
        .execute(&mut conn);
    let generic = "EXECUTE snip_g('technology', '<b>', '</b>')".fetch::<(i32, String)>(&mut conn);
    "DEALLOCATE snip_g".execute(&mut conn);

    assert_eq!(
        custom, baseline,
        "CUSTOM with param tags must match baseline"
    );
    assert_eq!(
        generic, baseline,
        "GENERIC with param tags must match baseline"
    );

    // Also test pdb.snippet_positions with parameterized limit/offset
    "SET plan_cache_mode = force_generic_plan".execute(&mut conn);
    "PREPARE snip_pos_g(text, int, int) AS
     SELECT id, pdb.snippet_positions(content, $2, $3)
     FROM snippet_param_test
     WHERE content ||| $1
     ORDER BY pdb.score(id) DESC"
        .execute(&mut conn);
    let pos_result = "EXECUTE snip_pos_g('technology', 5, 0)".execute_result(&mut conn);
    assert!(
        pos_result.is_ok(),
        "snippet_positions with param args must not error in GENERIC mode: {pos_result:?}"
    );
    "DEALLOCATE snip_pos_g".execute(&mut conn);

    "RESET plan_cache_mode".execute(&mut conn);
}

/// pdb.agg() must not panic when the JSON argument is parameterized.
///
/// Pre-fix: panicked with "pdb.agg argument must be a constant" — a Rust
/// panic that crashes the backend.
///
/// Post-fix: AggregateScan declines pushdown (NOTICE) and PG attempts the
/// standard aggregate path. Because `pdb.agg` is a custom-scan-only
/// placeholder it then returns a normal SQL error rather than crashing,
/// and the connection stays alive. That's the contract this test enforces.
#[rstest]
fn pdb_agg_with_parameterized_json(mut conn: PgConnection) {
    r#"
    DROP TABLE IF EXISTS agg_param_test CASCADE;
    CREATE TABLE agg_param_test (
        id SERIAL PRIMARY KEY,
        content TEXT,
        category TEXT
    );
    INSERT INTO agg_param_test (content, category)
    SELECT 'document ' || i, (ARRAY['a','b','c'])[1 + (i % 3)]
    FROM generate_series(1, 100) AS i;
    CREATE INDEX agg_param_idx ON agg_param_test
    USING bm25 (id, content, category) WITH (
        key_field = 'id',
        text_fields = '{"category": {"fast": true}}'
    );
    "#
    .execute(&mut conn);

    // Baseline: constant JSON literal pushes through the custom aggregate scan.
    let baseline = r#"
    SELECT pdb.agg('{"terms":{"field":"category"}}'::jsonb)
    FROM agg_param_test
    WHERE content ||| 'document'
    "#
    .fetch::<(serde_json::Value,)>(&mut conn);
    assert!(!baseline.is_empty(), "baseline should produce agg results");

    // GENERIC plan with parameterized JSON. The bug was a Rust panic; the
    // fix returns Err so aggregate pushdown is skipped. PG then tries the
    // placeholder `pdb.agg` and surfaces a controlled SQL error (XX000)
    // instead of crashing the backend. With force_generic_plan the GENERIC
    // path is used immediately — no need to burn through CUSTOM executes.
    "SET plan_cache_mode = force_generic_plan".execute(&mut conn);
    "PREPARE agg_g(jsonb) AS
     SELECT pdb.agg($1)
     FROM agg_param_test
     WHERE content ||| 'document'"
        .execute(&mut conn);

    let last_result =
        "EXECUTE agg_g('{\"terms\":{\"field\":\"category\"}}')".execute_result(&mut conn);

    // The connection must still be alive and the error (if any) must be a
    // normal SQL error, not a backend crash.
    let still_alive = "SELECT 1".execute_result(&mut conn);
    assert!(
        still_alive.is_ok(),
        "backend must still be alive after parameterized pdb.agg in GENERIC mode \
         (last error: {last_result:?}, post-check error: {still_alive:?})"
    );

    "DEALLOCATE agg_g".execute(&mut conn);
    "RESET plan_cache_mode".execute(&mut conn);
}
