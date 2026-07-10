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

use pretty_assertions::assert_eq;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;
use tests::fixtures::*;

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

fn total_cost(plan: &Value) -> f64 {
    root_plan(plan)
        .get("Total Cost")
        .and_then(Value::as_f64)
        .unwrap_or_else(|| panic!("EXPLAIN JSON should have a Total Cost: {plan:#?}"))
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

/// Structural parallel knobs only, leaving PostgreSQL's cost GUCs (`parallel_setup_cost`,
/// `cpu_operator_cost`, ...) at their defaults. Worker decisions asserted under this regime
/// are exactly the plans a real user gets. `min_parallel_table_scan_size = 0` keeps a
/// parallel path on the table so the model's *serial* choice is what's tested — not
/// PostgreSQL declining to consider parallelism at all on a tiny table.
fn set_default_costs(conn: &mut PgConnection) {
    r#"
    SET max_parallel_workers_per_gather = 2;
    SET max_parallel_workers = 8;
    SET min_parallel_table_scan_size = 0;
    SET paradedb.global_mutable_segment_rows = 0;
    SET paradedb.min_rows_per_worker = 300000;
    "#
    .execute(conn);
}

/// Same structural knobs as `set_default_costs`, but with the parallel cost GUCs scaled
/// down — a cheap `parallel_setup_cost` and an inflated `cpu_operator_cost` — so the cost
/// model's *parallel* branch is reachable on a CI-sized fixture. At real costs the #4664
/// repro needed millions of docs to clear Gather overhead; here we assert only the model's
/// *relative* choice (these shapes parallelize, the serial cases above do not). The
/// absolute default-cost crossover is validated separately by `topk_parallel_bench.sh`
/// (4M rows, real costs).
fn set_scaled_costs(conn: &mut PgConnection) {
    set_default_costs(conn);
    // The TopK work term is `Query::cost * cpu_index_tuple_cost`; inflate that per-doc
    // constant (and lower the Gather floor) to reach the parallel branch on a small fixture.
    r#"
    SET parallel_setup_cost = 100;
    SET cpu_index_tuple_cost = 0.05;
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
    setup_topk_desc_large(&mut conn);
    setup_many_segments(&mut conn);
    setup_topk_asc_small(&mut conn);
    setup_unanalyzed(&mut conn);

    // ---- Serial decisions, asserted at PostgreSQL's DEFAULT cost GUCs, so these are the
    // plans a real user gets. The #4664 fix lives here: a single-term score-DESC TopK stays
    // serial because Block-WAND prunes the lone posting list (work ~ 0, independent of cost
    // knobs). The general-path shapes (window agg, unordered, parameterized LIMIT) are serial
    // via the #3055 row cap / LIMIT cap on this CI-sized fixture — none depend on the scaled
    // knobs the parallel cases below use.
    set_default_costs(&mut conn);
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
        // Score-ASC can't prune (Block-WAND finds the highest scores, not the
        // lowest), so it must score every match — but with only ~5 matches the
        // scan work is far below the Gather threshold, so serial wins.
        PlanCase {
            name: "score_asc_tiny_match_is_serial",
            query: "SELECT id FROM topk_asc_small WHERE body @@@ 'special'
                    ORDER BY paradedb.score(id) ASC LIMIT 10",
            expected_workers: None,
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
        // The headline #4664 fix: a single-term score-DESC TopK stays serial at any LIMIT,
        // because Block-WAND prunes the single posting list and keeps serial scoring cheap.
        // Today every score-sorted scan parallelizes; on a 4M-row corpus that costs this
        // shape up to ~2x, since serial beats parallel at every LIMIT measured.
        PlanCase {
            name: "score_desc_single_term_large_limit_stays_serial",
            query: "SELECT id FROM topk_desc_many_segs WHERE body @@@ 'common'
                    ORDER BY paradedb.score(id) DESC LIMIT 1000",
            expected_workers: None,
        },
        PlanCase {
            name: "secondary_sort_key_is_serial",
            query: "SELECT id FROM topk_desc_large WHERE body @@@ 'alpha'
                    ORDER BY paradedb.score(id) DESC, id ASC LIMIT 10",
            expected_workers: None,
        },
    ] {
        assert_plan_case(&mut conn, case);
    }

    // The runtime-bound forms below need a generic plan so the Param survives to planning.
    "SET plan_cache_mode = force_generic_plan;".execute(&mut conn);

    // Parameterized LIMIT: not a prunable candidate (the LIMIT is a runtime Param), so it goes
    // through the cost model with an estimated LIMIT. On this small fixture the scan work is below
    // the Gather threshold, so it is serial.
    r#"
    PREPARE topk_desc_param_limit(int) AS
    SELECT id FROM topk_desc_large WHERE body @@@ 'alpha'
    ORDER BY paradedb.score(id) DESC LIMIT $1;
    "#
    .execute(&mut conn);
    assert_plan_case(
        &mut conn,
        PlanCase {
            name: "parameterized_limit_is_serial_on_small_data",
            query: "EXECUTE topk_desc_param_limit(10)",
            expected_workers: None,
        },
    );

    // Runtime-bound WHERE predicate (`@@@ $1`), UNSORTED: the predicate has no plan-time value to
    // open a scorer for, so the cost model can't run. An unsorted scan can skip segments once the
    // LIMIT is met, so it defers to the row-based heuristic, which is serial on this small fixture.
    r#"
    PREPARE unsorted_param_pred(text) AS
    SELECT id FROM topk_desc_large WHERE body @@@ $1 LIMIT 10;
    "#
    .execute(&mut conn);
    assert_plan_case(
        &mut conn,
        PlanCase {
            name: "runtime_bound_predicate_unsorted_defers_to_row_heuristic_serial",
            query: "EXECUTE unsorted_param_pred('alpha')",
            expected_workers: None,
        },
    );
    "RESET plan_cache_mode;".execute(&mut conn);

    // ---- Parallel decisions. The cost model parallelizes only when scan work clears Gather
    // overhead, which at real costs needs scale. Scale the parallel knobs down to reach that branch
    // on this small fixture; this asserts the model's *relative* choice (these shapes parallelize,
    // the serial ones above do not). They all route through the cost model (`cost_test`), not the
    // row-cap path. See `set_scaled_costs`.
    set_scaled_costs(&mut conn);
    for case in [
        PlanCase {
            name: "score_asc_large_match_parallelizes",
            query: "SELECT id FROM topk_desc_large WHERE body @@@ 'gamma'
                    ORDER BY paradedb.score(id) ASC LIMIT 10",
            expected_workers: Some(2),
        },
        PlanCase {
            name: "field_sort_large_match_parallelizes",
            query: "SELECT id FROM topk_desc_large WHERE body @@@ 'gamma'
                    ORDER BY id ASC LIMIT 10",
            expected_workers: Some(2),
        },
        PlanCase {
            name: "phrase_query_parallelizes",
            query: "SELECT id FROM topk_desc_large
                    WHERE body @@@ pdb.phrase(ARRAY['alpha', 'beta'])
                    ORDER BY paradedb.score(id) DESC LIMIT 10",
            expected_workers: Some(2),
        },
        PlanCase {
            name: "must_conjunction_parallelizes",
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
        // A dense term_set (here alpha ∪ gamma = every row) has no Block-WAND-pruning
        // weight, so it must reach the cost model and parallelize -- regression guard
        // against term_set being mis-classified prunable and forced serial.
        PlanCase {
            name: "term_set_dense_parallelizes",
            query: "SELECT id FROM topk_desc_large
                    WHERE body @@@ paradedb.term_set(terms => ARRAY[
                        paradedb.term('body', 'alpha'),
                        paradedb.term('body', 'gamma')
                    ])
                    ORDER BY paradedb.score(id) DESC LIMIT 10",
            expected_workers: Some(2),
        },
        // UNSORTED scans are cost-modeled too (no ORDER BY). A cheap term serializes (a small
        // LIMIT finds its rows in a sliver of the docset), but an expensive phrase must drive its
        // matches to find a full LIMIT, so its work clears the Gather threshold and it parallelizes.
        PlanCase {
            name: "unsorted_expensive_phrase_parallelizes",
            query: "SELECT id FROM topk_desc_large
                    WHERE body @@@ pdb.phrase(ARRAY['alpha', 'beta']) LIMIT 100",
            expected_workers: Some(2),
        },
    ] {
        assert_plan_case(&mut conn, case);
    }

    // Runtime-bound WHERE predicate (`@@@ $1`), SORTED: can't be costed, but a sorted scan must
    // visit every segment, so workers always help -- it parallelizes via the no-cost fallback
    // (the fix for parameterized score-DESC paging queries that previously serialized).
    "SET plan_cache_mode = force_generic_plan;".execute(&mut conn);
    r#"
    PREPARE sorted_param_pred(text) AS
    SELECT id FROM topk_desc_large WHERE body @@@ $1
    ORDER BY paradedb.score(id) DESC LIMIT 10;
    "#
    .execute(&mut conn);
    assert_plan_case(
        &mut conn,
        PlanCase {
            name: "runtime_bound_predicate_sorted_parallelizes",
            query: "EXECUTE sorted_param_pred('gamma')",
            expected_workers: Some(2),
        },
    );
    "RESET plan_cache_mode;".execute(&mut conn);

    // Unanalyzed table with a NON-prunable shape (score ASC): no match count, so the cost model
    // can't run and it defers to the row-based heuristic. With no row stats the heuristic applies
    // no caps, so it parallelizes by structure alone -- matching the pre-cost-model behavior.
    // (The prunable `unanalyzed_small_limit` case above stays serial via the short-circuit, which
    // runs before this fallback, so this case is what actually exercises the unanalyzed path.)
    assert_plan_case(
        &mut conn,
        PlanCase {
            name: "unanalyzed_nonprunable_defers_to_row_heuristic_parallel",
            query: "SELECT id FROM topk_unanalyzed WHERE body @@@ 'alpha'
                    ORDER BY paradedb.score(id) ASC LIMIT 10",
            expected_workers: Some(2),
        },
    );
}

#[rstest]
fn dense_multi_term_query_accounts_for_term_union_traversal(mut conn: PgConnection) {
    // Both cases share the scaled-cost regime on purpose: the contrast (single term serial,
    // term union parallel) must come from the query *shape*, not from different GUCs.
    set_scaled_costs(&mut conn);
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

#[rstest]
fn no_limit_costed_both_accounts_for_scan_work(mut conn: PgConnection) {
    set_scaled_costs(&mut conn);
    setup_multi_term(&mut conn);
    "SET paradedb.enable_aggregate_custom_scan = false;".execute(&mut conn);

    // Same scan in both cases, so the only thing that differs is how many rows cross the Gather --
    // isolating the transport cost as the serial-vs-parallel driver.
    let count_plan = explain(
        &mut conn,
        "SELECT COUNT(*) FROM topk_desc_multi_term WHERE body @@@ 'alpha'",
    );
    let count_root = root_plan(&count_plan);
    assert_eq!(
        max_workers_planned(count_root),
        Some(2),
        "COUNT(*) should go parallel: the Partial Aggregate collapses rows before the Gather, so the \
         costed scan work splits across workers with almost nothing to transport:\n{count_plan:#?}"
    );

    let select_plan = explain(
        &mut conn,
        "SELECT * FROM topk_desc_multi_term WHERE body @@@ 'alpha'",
    );
    let select_root = root_plan(&select_plan);
    assert_eq!(
        max_workers_planned(select_root),
        None,
        "bulk SELECT should serialize: every matched row crosses the Gather, so transport outweighs \
         the scan-work split:\n{select_plan:#?}"
    );
}

/// A prunable score-DESC TopK costs the scan by output rows, not its full drive cost: Block-WAND
/// prunes it sublinear, so folding `drive_cost` into the path cost would overstate the work. A
/// non-prunable score-ASC TopK over the *same* term and match set is identical except for the
/// excluded `drive_cost`, so it must cost strictly more -- equal cost means the exclusion is gone.
#[rstest]
fn prunable_topk_cost_excludes_drive_work(mut conn: PgConnection) {
    set_scaled_costs(&mut conn);
    // Force serial so the comparison is the bare scan cost, not a Gather plan on either side.
    "SET max_parallel_workers_per_gather = 0;".execute(&mut conn);
    setup_multi_term(&mut conn);

    // Same single term and match set; only the sort direction differs. Block-WAND prunes score DESC
    // (cost excludes drive_cost) but not score ASC (cost includes it).
    let prunable = total_cost(&explain(
        &mut conn,
        "SELECT id FROM topk_desc_multi_term WHERE body @@@ 'alpha'
         ORDER BY paradedb.score(id) DESC LIMIT 10",
    ));
    let non_prunable = total_cost(&explain(
        &mut conn,
        "SELECT id FROM topk_desc_multi_term WHERE body @@@ 'alpha'
         ORDER BY paradedb.score(id) ASC LIMIT 10",
    ));

    assert!(
        prunable < non_prunable,
        "prunable score-DESC TopK ({prunable}) must cost strictly less than the otherwise-identical \
         non-prunable score-ASC ({non_prunable}); equal cost means drive_cost is no longer excluded"
    );
}
