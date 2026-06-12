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
use pretty_assertions::assert_eq;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;

#[derive(Clone, Copy)]
struct PlanCase {
    name: &'static str,
    query: &'static str,
    expected_workers: Option<i64>,
}

fn explain(conn: &mut PgConnection, query: &str) -> Value {
    let (plan,) = format!("EXPLAIN (FORMAT JSON) {query}").fetch_one::<(Value,)>(conn);
    plan
}

fn root_plan(plan: &Value) -> &Value {
    plan.pointer("/0/Plan")
        .unwrap_or_else(|| panic!("EXPLAIN JSON should have /0/Plan: {plan:#?}"))
}

fn max_workers_planned(node: &Value) -> Option<i64> {
    let here = node.get("Workers Planned").and_then(Value::as_i64);
    node.get("Plans")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(max_workers_planned)
        .fold(here, |max, workers| Some(max.unwrap_or(0).max(workers)))
}

fn contains_topk_exec(node: &Value) -> bool {
    node.get("Exec Method").and_then(Value::as_str) == Some("TopKScanExecState")
        || node
            .get("Plans")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .any(contains_topk_exec)
}

fn assert_plan_case(conn: &mut PgConnection, case: PlanCase) {
    let plan = explain(conn, case.query);
    let root = root_plan(&plan);
    assert!(
        contains_topk_exec(root),
        "{} should use TopKScanExecState:\n{plan:#?}",
        case.name
    );
    assert_eq!(
        max_workers_planned(root),
        case.expected_workers,
        "{} worker count mismatch:\n{plan:#?}",
        case.name
    );
}

fn set_cost_defaults(conn: &mut PgConnection) {
    // The expense cost model parallelizes a score-DESC TopK only when
    // `matches * per_match * comparison_cost` clears PostgreSQL's Gather overhead.
    // The fixtures below are deliberately small (a few thousand matches) to keep
    // CI fast, so we scale the parallel knobs down — a cheap `parallel_setup_cost`
    // and an inflated `cpu_operator_cost` — to make that threshold crossable at
    // small scale. The model's *relative* decisions (single-term DESC serial,
    // unions/ASC/phrase parallel) are what we assert; the absolute crossover
    // point is a function of these costs.
    r#"
    SET max_parallel_workers_per_gather = 2;
    SET max_parallel_workers = 8;
    SET parallel_setup_cost = 100;
    SET parallel_tuple_cost = 0.1;
    SET cpu_tuple_cost = 0.01;
    SET cpu_operator_cost = 0.05;
    SET min_parallel_table_scan_size = 0;
    SET paradedb.global_mutable_segment_rows = 0;
    SET paradedb.min_rows_per_worker = 300000;
    "#
    .execute(conn);
}

fn setup_topk_desc_large(conn: &mut PgConnection) {
    r#"
    DROP TABLE IF EXISTS topk_desc_large CASCADE;
    CREATE TABLE topk_desc_large (
        id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
        body TEXT NOT NULL
    );
    CREATE INDEX topk_desc_large_idx ON topk_desc_large USING bm25 (id, body)
        WITH (
            key_field = 'id',
            target_segment_count = 4,
            mutable_segment_rows = 5000,
            layer_sizes = '10TB',
            background_layer_sizes = '10TB'
        );
    INSERT INTO topk_desc_large (body)
    SELECT CASE WHEN g % 3 = 0 THEN 'alpha beta' ELSE 'gamma delta' END
    FROM generate_series(1, 5000) g;
    INSERT INTO topk_desc_large (body)
    SELECT CASE WHEN g % 3 = 0 THEN 'alpha beta' ELSE 'gamma delta' END
    FROM generate_series(1, 5000) g;
    INSERT INTO topk_desc_large (body)
    SELECT CASE WHEN g % 3 = 0 THEN 'alpha beta' ELSE 'gamma delta' END
    FROM generate_series(1, 5000) g;
    INSERT INTO topk_desc_large (body)
    SELECT CASE WHEN g % 3 = 0 THEN 'alpha beta' ELSE 'gamma delta' END
    FROM generate_series(1, 5000) g;
    ANALYZE topk_desc_large;
    "#
    .execute(conn);
}

fn setup_many_segments(conn: &mut PgConnection) {
    r#"
    DROP TABLE IF EXISTS topk_desc_many_segs CASCADE;
    CREATE TABLE topk_desc_many_segs (
        id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
        body TEXT NOT NULL
    );
    CREATE INDEX topk_desc_many_segs_idx ON topk_desc_many_segs USING bm25 (id, body)
        WITH (
            key_field = 'id',
            target_segment_count = 32,
            mutable_segment_rows = 1000,
            layer_sizes = '10TB',
            background_layer_sizes = '10TB'
        );
    "#
    .execute(conn);

    for _ in 0..32 {
        r#"
        INSERT INTO topk_desc_many_segs (body)
        SELECT CASE WHEN g = 1 THEN 'rare token' ELSE 'common other' END
        FROM generate_series(1, 1000) g;
        "#
        .execute(conn);
    }

    "ANALYZE topk_desc_many_segs;".execute(conn);
}

fn setup_topk_asc_small(conn: &mut PgConnection) {
    r#"
    DROP TABLE IF EXISTS topk_asc_small CASCADE;
    CREATE TABLE topk_asc_small (
        id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
        body TEXT NOT NULL
    );
    CREATE INDEX topk_asc_small_idx ON topk_asc_small USING bm25 (id, body)
        WITH (
            key_field = 'id',
            target_segment_count = 4,
            mutable_segment_rows = 1000,
            layer_sizes = '10TB',
            background_layer_sizes = '10TB'
        );
    INSERT INTO topk_asc_small (body)
    SELECT 'common word' FROM generate_series(1, 1000);
    INSERT INTO topk_asc_small (body)
    SELECT 'special token' FROM generate_series(1, 5);
    ANALYZE topk_asc_small;
    "#
    .execute(conn);
}

fn setup_multi_term(conn: &mut PgConnection) {
    r#"
    DROP TABLE IF EXISTS topk_desc_multi_term CASCADE;
    CREATE TABLE topk_desc_multi_term (
        id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
        body TEXT NOT NULL
    );
    CREATE INDEX topk_desc_multi_term_idx ON topk_desc_multi_term USING bm25 (id, body)
        WITH (
            key_field = 'id',
            target_segment_count = 4,
            mutable_segment_rows = 5000,
            layer_sizes = '10TB',
            background_layer_sizes = '10TB'
        );
    INSERT INTO topk_desc_multi_term (body)
    SELECT 'alpha beta gamma delta epsilon' FROM generate_series(1, 5000);
    INSERT INTO topk_desc_multi_term (body)
    SELECT 'alpha beta gamma delta epsilon' FROM generate_series(1, 5000);
    INSERT INTO topk_desc_multi_term (body)
    SELECT 'alpha beta gamma delta epsilon' FROM generate_series(1, 5000);
    INSERT INTO topk_desc_multi_term (body)
    SELECT 'alpha beta gamma delta epsilon' FROM generate_series(1, 5000);
    ANALYZE topk_desc_multi_term;
    "#
    .execute(conn);
}

fn setup_unanalyzed(conn: &mut PgConnection) {
    r#"
    DROP TABLE IF EXISTS topk_unanalyzed CASCADE;
    CREATE TABLE topk_unanalyzed (
        id INTEGER GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
        body TEXT NOT NULL
    );
    CREATE INDEX topk_unanalyzed_idx ON topk_unanalyzed USING bm25 (id, body)
        WITH (
            key_field = 'id',
            target_segment_count = 4,
            mutable_segment_rows = 100,
            layer_sizes = '10TB',
            background_layer_sizes = '10TB'
        );
    INSERT INTO topk_unanalyzed (body)
    SELECT 'alpha beta' FROM generate_series(1, 100);
    INSERT INTO topk_unanalyzed (body)
    SELECT 'alpha beta' FROM generate_series(1, 100);
    "#
    .execute(conn);
}

#[rstest]
fn cost_based_topk_plan_shapes(mut conn: PgConnection) {
    set_cost_defaults(&mut conn);
    setup_topk_desc_large(&mut conn);
    setup_many_segments(&mut conn);
    setup_topk_asc_small(&mut conn);
    setup_unanalyzed(&mut conn);

    for case in [
        PlanCase {
            name: "score_desc_large_match_small_limit_is_serial",
            query: "SELECT id FROM topk_desc_large WHERE body @@@ 'alpha'
                    ORDER BY paradedb.score(id) DESC LIMIT 10",
            expected_workers: None,
        },
        PlanCase {
            name: "score_desc_score_projection_preserves_serial_choice",
            query: "SELECT id, paradedb.score(id) FROM topk_desc_large
                    WHERE body @@@ 'alpha'
                    ORDER BY paradedb.score(id) DESC LIMIT 10",
            expected_workers: None,
        },
        PlanCase {
            name: "score_desc_many_segments_rare_match_is_serial_by_default",
            query: "SELECT id FROM topk_desc_many_segs WHERE body @@@ 'rare'
                    ORDER BY paradedb.score(id) DESC LIMIT 20",
            expected_workers: None,
        },
        // The headline #4664 fix: a single-term score-DESC TopK stays serial
        // even at a large static LIMIT. Block-WAND prunes a single posting list,
        // so `per_match` is 0 and there is no scan work to split across workers —
        // parallelizing only adds Gather-Merge overhead. This is exactly the
        // shape that was being wrongly parallelized before.
        PlanCase {
            name: "score_desc_single_term_large_limit_stays_serial",
            query: "SELECT id FROM topk_desc_many_segs WHERE body @@@ 'common'
                    ORDER BY paradedb.score(id) DESC LIMIT 1000",
            expected_workers: None,
        },
        PlanCase {
            name: "score_asc_large_match_uses_general_parallel_path",
            query: "SELECT id FROM topk_desc_large WHERE body @@@ 'gamma'
                    ORDER BY paradedb.score(id) ASC LIMIT 10",
            expected_workers: Some(2),
        },
        // Score-ASC can't prune (Block-WAND finds the highest scores, not the
        // lowest), so it must score every match — but with only ~5 matches the
        // scan work is far below the Gather threshold, so serial wins.
        PlanCase {
            name: "score_asc_tiny_match_is_serial",
            query: "SELECT id FROM topk_asc_small WHERE body @@@ 'special'
                    ORDER BY paradedb.score(id) ASC LIMIT 10",
            expected_workers: None,
        },
        PlanCase {
            name: "field_sort_large_match_uses_general_parallel_path",
            query: "SELECT id FROM topk_desc_large WHERE body @@@ 'gamma'
                    ORDER BY id ASC LIMIT 10",
            expected_workers: Some(2),
        },
        // Field-sorted TopK still has to scan every match before sorting, but
        // ~5 matches is well below the Gather threshold, so serial wins.
        PlanCase {
            name: "field_sort_tiny_match_is_serial",
            query: "SELECT id FROM topk_asc_small WHERE body @@@ 'special'
                    ORDER BY id ASC LIMIT 10",
            expected_workers: None,
        },
        PlanCase {
            name: "unordered_topk_small_limit_is_serial",
            query: "SELECT id FROM topk_desc_large WHERE body @@@ 'gamma' LIMIT 10",
            expected_workers: None,
        },
        // Window aggregates genuinely bail to the general path (`compute_nworkers`),
        // not the cost model. With #4457 reverted, the general path applies the
        // #3055 row cap to sorted output, so on this ~20K-row fixture (below the
        // 300K `min_rows_per_worker`) it is serial.
        PlanCase {
            name: "window_aggregate_routes_to_general_path_serial_on_small_data",
            query: "SELECT id,
                           paradedb.score(id),
                           pdb.agg('{\"terms\": {\"field\": \"body\", \"size\": 5}}', false)
                             OVER () AS body_facets
                    FROM topk_desc_large
                    WHERE body @@@ 'alpha'
                    ORDER BY paradedb.score(id) DESC
                    LIMIT 10",
            expected_workers: None,
        },
        // Same, but with the window aggregate nested inside a function call: the
        // planner hook's replacement pass descends into FuncExpr arguments, so
        // the window_agg() placeholder ends up nested inside jsonb_pretty()
        // rather than at the top of the target entry. Only a recursive
        // target-list walk detects it, and it must route to the general path
        // exactly like the top-level form above.
        PlanCase {
            name: "nested_window_aggregate_routes_to_general_path_serial_on_small_data",
            query: "SELECT id,
                           paradedb.score(id),
                           jsonb_pretty(
                               pdb.agg('{\"terms\": {\"field\": \"body\", \"size\": 5}}', false)
                                 OVER ()
                           ) AS body_facets
                    FROM topk_desc_large
                    WHERE body @@@ 'alpha'
                    ORDER BY paradedb.score(id) DESC
                    LIMIT 10",
            expected_workers: None,
        },
        PlanCase {
            name: "unanalyzed_small_limit_is_serial",
            query: "SELECT id FROM topk_unanalyzed WHERE body @@@ 'alpha'
                    ORDER BY paradedb.score(id) DESC LIMIT 10",
            expected_workers: None,
        },
        PlanCase {
            name: "secondary_sort_key_routes_to_general_parallel_path",
            query: "SELECT id FROM topk_desc_large WHERE body @@@ 'alpha'
                    ORDER BY paradedb.score(id) DESC, id ASC LIMIT 10",
            expected_workers: Some(2),
        },
        PlanCase {
            name: "phrase_query_routes_to_general_parallel_path",
            query: "SELECT id FROM topk_desc_large
                    WHERE body @@@ pdb.phrase(ARRAY['alpha', 'beta'])
                    ORDER BY paradedb.score(id) DESC LIMIT 10",
            expected_workers: Some(2),
        },
        PlanCase {
            name: "must_conjunction_routes_to_general_parallel_path",
            query: "SELECT id FROM topk_desc_large
                    WHERE id @@@ paradedb.boolean(
                        must => ARRAY[
                            paradedb.term('body', 'alpha'),
                            paradedb.term('body', 'beta')
                        ]
                    )
                    ORDER BY paradedb.score(id) DESC LIMIT 10",
            expected_workers: Some(2),
        },
    ] {
        assert_plan_case(&mut conn, case);
    }

    "SET plan_cache_mode = force_generic_plan;".execute(&mut conn);
    r#"
    PREPARE topk_desc_param_limit(int) AS
    SELECT id FROM topk_desc_large WHERE body @@@ 'alpha'
    ORDER BY paradedb.score(id) DESC LIMIT $1;
    "#
    .execute(&mut conn);
    // Parameterized LIMIT bails to the general path (`compute_nworkers`), not the
    // cost model. With #4457 reverted, the general path applies the #3055 row cap to
    // sorted output, so on this ~20K-row fixture it is serial.
    assert_plan_case(
        &mut conn,
        PlanCase {
            name: "parameterized_limit_routes_to_general_path_serial_on_small_data",
            query: "EXECUTE topk_desc_param_limit(10)",
            expected_workers: None,
        },
    );
}

#[rstest]
fn dense_multi_term_query_accounts_for_term_union_traversal(mut conn: PgConnection) {
    set_cost_defaults(&mut conn);
    setup_multi_term(&mut conn);

    let matches = r#"
    SELECT count(*) AS matches
    FROM topk_desc_multi_term
    WHERE body ||| 'alpha'
    UNION ALL
    SELECT count(*) AS matches
    FROM topk_desc_multi_term
    WHERE body ||| 'alpha beta gamma delta epsilon'
    ORDER BY matches
    "#
    .fetch::<(i64,)>(&mut conn);
    assert_eq!(matches.len(), 2);
    let (single_term_matches,) = matches[0];
    let (term_union_matches,) = matches[1];
    assert_eq!(single_term_matches, term_union_matches);

    r#"
    SET cpu_operator_cost = 0.05;
    SET parallel_setup_cost = 100;
    "#
    .execute(&mut conn);

    for case in [
        PlanCase {
            name: "dense_single_term_stays_serial",
            query: "SELECT id FROM topk_desc_multi_term WHERE body ||| 'alpha'
                    ORDER BY paradedb.score(id) DESC LIMIT 10",
            expected_workers: None,
        },
        PlanCase {
            name: "dense_multi_term_term_union_chooses_parallel",
            query: "SELECT id FROM topk_desc_multi_term
                    WHERE body ||| 'alpha beta gamma delta epsilon'
                    ORDER BY paradedb.score(id) DESC LIMIT 10",
            expected_workers: Some(2),
        },
    ] {
        assert_plan_case(&mut conn, case);
    }
}
