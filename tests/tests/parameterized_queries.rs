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
/// Pre-fix bugs:
///   - LIMIT 5 OFFSET $2: GENERIC fell back to ColumnarExecState (full scan
///     + Sort), producing a different row order than CUSTOM.
///   - LIMIT $1 OFFSET 5: GENERIC's TopK fetched only `LIMIT` rows; PG's outer
///     Limit OFFSET 5 then skipped them all, returning **0 rows**.
///   - LIMIT $1 OFFSET $2: same TopK undercount bug.
///
/// All three cases must now return identical rows to the unprepared baseline.
#[rstest]
fn generic_plan_parameterized_offset(mut conn: PgConnection) {
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

    // Baseline: constant LIMIT + constant OFFSET
    let baseline = "SELECT id FROM param_offset_test
                    WHERE content ||| 'technology'
                    ORDER BY pdb.score(id) DESC
                    LIMIT 5 OFFSET 5"
        .fetch::<(i32,)>(&mut conn);
    assert_eq!(baseline.len(), 5, "baseline should return 5 rows");

    // --- Case 1: Const LIMIT + Param OFFSET ---
    "SET plan_cache_mode = force_custom_plan".execute(&mut conn);
    "PREPARE off1_c(text, int) AS
     SELECT id FROM param_offset_test WHERE content ||| $1
     ORDER BY pdb.score(id) DESC LIMIT 5 OFFSET $2"
        .execute(&mut conn);
    let custom_1 = "EXECUTE off1_c('technology', 5)".fetch::<(i32,)>(&mut conn);
    "DEALLOCATE off1_c".execute(&mut conn);

    "SET plan_cache_mode = force_generic_plan".execute(&mut conn);
    "PREPARE off1_g(text, int) AS
     SELECT id FROM param_offset_test WHERE content ||| $1
     ORDER BY pdb.score(id) DESC LIMIT 5 OFFSET $2"
        .execute(&mut conn);
    let generic_1 = "EXECUTE off1_g('technology', 5)".fetch::<(i32,)>(&mut conn);
    "DEALLOCATE off1_g".execute(&mut conn);

    assert_eq!(custom_1, baseline, "Case 1 CUSTOM must match baseline");
    assert_eq!(generic_1, baseline, "Case 1 GENERIC must match baseline");

    // --- Case 2: Param LIMIT + Const OFFSET (was returning 0 rows) ---
    "SET plan_cache_mode = force_custom_plan".execute(&mut conn);
    "PREPARE off2_c(text, int) AS
     SELECT id FROM param_offset_test WHERE content ||| $1
     ORDER BY pdb.score(id) DESC LIMIT $2 OFFSET 5"
        .execute(&mut conn);
    let custom_2 = "EXECUTE off2_c('technology', 5)".fetch::<(i32,)>(&mut conn);
    "DEALLOCATE off2_c".execute(&mut conn);

    "SET plan_cache_mode = force_generic_plan".execute(&mut conn);
    "PREPARE off2_g(text, int) AS
     SELECT id FROM param_offset_test WHERE content ||| $1
     ORDER BY pdb.score(id) DESC LIMIT $2 OFFSET 5"
        .execute(&mut conn);
    let generic_2 = "EXECUTE off2_g('technology', 5)".fetch::<(i32,)>(&mut conn);
    "DEALLOCATE off2_g".execute(&mut conn);

    assert_eq!(custom_2, baseline, "Case 2 CUSTOM must match baseline");
    assert_eq!(
        generic_2, baseline,
        "Case 2 GENERIC must match baseline (was returning 0 rows pre-fix)"
    );

    // --- Case 3: Param LIMIT + Param OFFSET ---
    "SET plan_cache_mode = force_custom_plan".execute(&mut conn);
    "PREPARE off3_c(text, int, int) AS
     SELECT id FROM param_offset_test WHERE content ||| $1
     ORDER BY pdb.score(id) DESC LIMIT $2 OFFSET $3"
        .execute(&mut conn);
    let custom_3 = "EXECUTE off3_c('technology', 5, 5)".fetch::<(i32,)>(&mut conn);
    "DEALLOCATE off3_c".execute(&mut conn);

    "SET plan_cache_mode = force_generic_plan".execute(&mut conn);
    "PREPARE off3_g(text, int, int) AS
     SELECT id FROM param_offset_test WHERE content ||| $1
     ORDER BY pdb.score(id) DESC LIMIT $2 OFFSET $3"
        .execute(&mut conn);
    let generic_3 = "EXECUTE off3_g('technology', 5, 5)".fetch::<(i32,)>(&mut conn);
    "DEALLOCATE off3_g".execute(&mut conn);

    assert_eq!(custom_3, baseline, "Case 3 CUSTOM must match baseline");
    assert_eq!(generic_3, baseline, "Case 3 GENERIC must match baseline");

    "RESET plan_cache_mode".execute(&mut conn);
}

/// JoinScan must survive parameterized LIMIT (was previously disabled with
/// NOTICE: "JoinScan not used: activation checks failed (LIMIT / ...)").
#[rstest]
fn joinscan_survives_parameterized_limit(mut conn: PgConnection) {
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

    // CUSTOM mode with parameterized LIMIT
    "SET plan_cache_mode = force_custom_plan".execute(&mut conn);
    "PREPARE js_c(text, int) AS
     SELECT p.id, p.name FROM js_prods p
     JOIN js_cats c ON p.cat_id = c.id
     WHERE p.name ||| $1
     ORDER BY pdb.score(p.id) DESC
     LIMIT $2"
        .execute(&mut conn);
    let mut custom_results = "EXECUTE js_c('electronics', 10)".fetch::<(i32, String)>(&mut conn);
    "DEALLOCATE js_c".execute(&mut conn);

    // GENERIC mode with parameterized LIMIT
    "SET plan_cache_mode = force_generic_plan".execute(&mut conn);
    "PREPARE js_g(text, int) AS
     SELECT p.id, p.name FROM js_prods p
     JOIN js_cats c ON p.cat_id = c.id
     WHERE p.name ||| $1
     ORDER BY pdb.score(p.id) DESC
     LIMIT $2"
        .execute(&mut conn);
    let mut generic_results = "EXECUTE js_g('electronics', 10)".fetch::<(i32, String)>(&mut conn);
    "DEALLOCATE js_g".execute(&mut conn);

    // All `electronics` matches share identical scores, so the ORDER BY tie is
    // unstable. Compare the *set* of rows by sorting first — what matters is
    // that JoinScan produces a correct top-10 in both modes.
    custom_results.sort_by_key(|r| r.0);
    generic_results.sort_by_key(|r| r.0);
    assert_eq!(
        custom_results, generic_results,
        "JoinScan with param LIMIT must return the same set of rows in both modes"
    );
    assert!(!custom_results.is_empty(), "should have join results");
    assert_eq!(custom_results.len(), 10, "LIMIT $2=10 must be respected");

    // GENERIC plan must actually use JoinScan (not fall back to NestedLoop).
    "SET plan_cache_mode = force_generic_plan".execute(&mut conn);
    "PREPARE js_g_explain(text, int) AS
     SELECT p.id, p.name FROM js_prods p
     JOIN js_cats c ON p.cat_id = c.id
     WHERE p.name ||| $1
     ORDER BY pdb.score(p.id) DESC
     LIMIT $2"
        .execute(&mut conn);
    let (plan,) = "EXPLAIN (FORMAT JSON) EXECUTE js_g_explain('electronics', 10)"
        .fetch_one::<(Value,)>(&mut conn);
    let plan_text = format!("{plan:#}");
    assert!(
        plan_text.contains("ParadeDB Join Scan"),
        "GENERIC mode with param LIMIT must keep JoinScan: {plan_text}"
    );
    "DEALLOCATE js_g_explain".execute(&mut conn);

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

    assert_eq!(custom, baseline, "CUSTOM with param tags must match baseline");
    assert_eq!(generic, baseline, "GENERIC with param tags must match baseline");

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
    // instead of crashing the backend.
    "SET plan_cache_mode = force_generic_plan".execute(&mut conn);
    "PREPARE agg_g(jsonb) AS
     SELECT pdb.agg($1)
     FROM agg_param_test
     WHERE content ||| 'document'"
        .execute(&mut conn);

    // Repeat enough times to land on the GENERIC plan (>= 6th execute).
    let mut last_result: Result<(), sqlx::Error> = Ok(());
    for _ in 0..7 {
        last_result = "EXECUTE agg_g('{\"terms\":{\"field\":\"category\"}}')"
            .execute_result(&mut conn);
    }

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
