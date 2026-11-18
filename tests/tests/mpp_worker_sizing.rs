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

//! Integration coverage for MPP worker sizing, `parallel_leader_participation`, per-table
//! `parallel_workers` reloptions, `debug_parallel_query`, generic `PREPARE` mode rebaking, and
//! `Gather`-equivalent parallel-mode hazards across `JoinScan`, `AggregateScan`, and `BaseScan`.

use rstest::*;
use sqlx::PgConnection;
use tests::fixtures::*;

#[derive(Clone, Copy, Debug)]
struct Segments(usize);

#[derive(Clone, Copy, Debug)]
struct RowsPerSegment(usize);

#[derive(Clone, Copy, Debug)]
struct PerGather(usize);

#[derive(Clone, Copy, Debug)]
enum LeaderMode {
    Participating,
    CoordinatorOnly,
}

impl LeaderMode {
    fn as_guc(self) -> &'static str {
        match self {
            Self::Participating => "on",
            Self::CoordinatorOnly => "off",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExpectedMpp {
    Serial,
    Workers(u32),
}

impl ExpectedMpp {
    fn is_distributed(self) -> bool {
        matches!(self, Self::Workers(_))
    }

    fn launched_workers(self) -> Option<u32> {
        match self {
            Self::Serial => None,
            Self::Workers(n) => Some(n),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum ExplainMode {
    CostsOff,
    Verbose,
    AnalyzeVerbose,
}

impl ExplainMode {
    fn as_opts(self) -> &'static str {
        match self {
            Self::CostsOff => "COSTS OFF",
            Self::Verbose => "VERBOSE, COSTS OFF",
            Self::AnalyzeVerbose => "ANALYZE, VERBOSE, COSTS OFF, TIMING OFF",
        }
    }
}

fn setup_segmented_pair(
    conn: &mut PgConnection,
    segments: Segments,
    rows_per_segment: RowsPerSegment,
) {
    r#"
    SET paradedb.enable_join_custom_scan TO on;
    SET paradedb.enable_aggregate_custom_scan TO on;
    SET paradedb.mpp_min_rows TO 0;
    SET max_parallel_workers TO 16;
    SET min_parallel_table_scan_size TO 0;
    SET parallel_setup_cost TO 0;
    SET parallel_tuple_cost TO 0;

    CREATE TABLE ws_users (id bigserial PRIMARY KEY, name text, age int);
    CREATE TABLE ws_products (id bigserial PRIMARY KEY, name text, age int);
    CREATE INDEX ws_users_idx ON ws_users USING paradedb (id, (name::pdb.literal), age)
    WITH (layer_sizes = '0', background_layer_sizes = '0');
    CREATE INDEX ws_products_idx ON ws_products USING paradedb (id, name, age)
    WITH (layer_sizes = '0', background_layer_sizes = '0');

    SET paradedb.global_mutable_segment_rows = 0;
    "#
    .execute(conn);

    for seg in 0..segments.0 {
        let start = seg * rows_per_segment.0 + 1;
        let end = (seg + 1) * rows_per_segment.0;
        format!(
            "INSERT INTO ws_users (name, age)
             SELECT (ARRAY['bob', 'alice'])[1 + (g % 2)], g % 25
             FROM generate_series({start}, {end}) g;
             INSERT INTO ws_products (name, age)
             SELECT 'item ' || g, g % 25
             FROM generate_series({start}, {end}) g;"
        )
        .execute(conn);
    }

    "RESET paradedb.global_mutable_segment_rows; ANALYZE ws_users; ANALYZE ws_products;"
        .execute(conn);
}

fn explain(conn: &mut PgConnection, mode: ExplainMode, sql: &str) -> String {
    format!("EXPLAIN ({}) {sql}", mode.as_opts())
        .fetch::<(String,)>(conn)
        .into_iter()
        .map(|(line,)| line)
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_first_u32_after(plan: &str, marker: &str) -> Option<u32> {
    plan.lines().find_map(|line| {
        let (_, rest) = line.split_once(marker)?;
        let digits: String = rest
            .trim_start()
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        digits.parse().ok()
    })
}

fn parse_mpp_launched_workers(plan: &str) -> Option<u32> {
    parse_first_u32_after(plan, "MPP Launch: workers=")
}

fn parse_gather_workers_planned(plan: &str) -> Option<u32> {
    parse_first_u32_after(plan, "Workers Planned:")
}

#[rstest]
#[case::one_segment_never_launches(
    Segments(1),
    PerGather(8),
    LeaderMode::Participating,
    ExpectedMpp::Serial
)]
#[case::zero_per_gather_serial(
    Segments(2),
    PerGather(0),
    LeaderMode::Participating,
    ExpectedMpp::Serial
)]
#[case::one_per_gather_leader_on(
    Segments(2),
    PerGather(1),
    LeaderMode::Participating,
    ExpectedMpp::Workers(1)
)]
#[case::one_per_gather_leader_off_serial(
    Segments(2),
    PerGather(1),
    LeaderMode::CoordinatorOnly,
    ExpectedMpp::Serial
)]
#[case::two_per_gather_leader_off(
    Segments(2),
    PerGather(2),
    LeaderMode::CoordinatorOnly,
    ExpectedMpp::Workers(2)
)]
#[case::oversized_cap_clamps_to_tasks_leader_on(
    Segments(2),
    PerGather(8),
    LeaderMode::Participating,
    ExpectedMpp::Workers(1)
)]
#[case::oversized_cap_clamps_to_tasks_leader_off(
    Segments(2),
    PerGather(8),
    LeaderMode::CoordinatorOnly,
    ExpectedMpp::Workers(2)
)]
#[case::multiplexes_tasks_at_cap_leader_on(
    Segments(4),
    PerGather(2),
    LeaderMode::Participating,
    ExpectedMpp::Workers(2)
)]
#[case::four_segments_clamps_to_three_leader_on(
    Segments(4),
    PerGather(8),
    LeaderMode::Participating,
    ExpectedMpp::Workers(3)
)]
fn worker_sizing_matrix(
    mut serial_conn: SerialConn,
    #[case] segments: Segments,
    #[case] per_gather: PerGather,
    #[case] leader: LeaderMode,
    #[case] expected: ExpectedMpp,
) {
    setup_segmented_pair(&mut serial_conn, segments, RowsPerSegment(60));

    let join_sql = "SELECT u.id FROM ws_users u JOIN ws_products p ON u.age = p.age \
                    WHERE u.name === 'bob' ORDER BY u.id LIMIT 150";
    let agg_sql = "SELECT p.age, count(*)::bigint AS cnt \
                   FROM ws_users u JOIN ws_products p ON u.age = p.age \
                   WHERE u.name === 'bob' GROUP BY p.age ORDER BY p.age LIMIT 20";

    "SET max_parallel_workers_per_gather TO 0; SET parallel_leader_participation TO on;"
        .execute(&mut serial_conn);
    let serial_join = join_sql.fetch::<(i64,)>(&mut serial_conn);
    let serial_agg = agg_sql.fetch::<(i32, i64)>(&mut serial_conn);
    assert!(!serial_join.is_empty());
    assert!(!serial_agg.is_empty());

    format!(
        "SET max_parallel_workers_per_gather TO {}; SET parallel_leader_participation TO {};",
        per_gather.0,
        leader.as_guc()
    )
    .execute(&mut serial_conn);

    for (label, sql) in [("JoinScan", join_sql), ("AggregateScan", agg_sql)] {
        let plain = explain(&mut serial_conn, ExplainMode::Verbose, sql);
        assert_eq!(
            plain.contains("DistributedExec"),
            expected.is_distributed(),
            "{label} plain EXPLAIN mismatch for {segments:?} {per_gather:?} {leader:?}:\n{plain}"
        );
        if !expected.is_distributed() {
            assert!(
                !plain.contains("RoundRobinBatch") && !plain.contains("SortPreservingMergeExec"),
                "{label} serial plain EXPLAIN leaked distributed operators:\n{plain}"
            );
        }

        let analyze = explain(&mut serial_conn, ExplainMode::AnalyzeVerbose, sql);
        assert_eq!(
            parse_mpp_launched_workers(&analyze),
            expected.launched_workers(),
            "{label} EXPLAIN ANALYZE worker count mismatch:\n{analyze}"
        );
        if expected.is_distributed() {
            assert!(
                analyze.contains("output_rows="),
                "{label} EXPLAIN ANALYZE should merge worker metrics:\n{analyze}"
            );
        }
    }

    assert_eq!(join_sql.fetch::<(i64,)>(&mut serial_conn), serial_join);
    assert_eq!(agg_sql.fetch::<(i32, i64)>(&mut serial_conn), serial_agg);
}

#[rstest]
fn reloption_parallel_workers(mut serial_conn: SerialConn) {
    setup_segmented_pair(&mut serial_conn, Segments(4), RowsPerSegment(60));
    // Restore default row gates so we also test reloption overrides of `min_rows_per_worker` and
    // `paradedb.mpp_min_rows`.
    "RESET paradedb.min_rows_per_worker; RESET paradedb.mpp_min_rows; \
     SET max_parallel_workers_per_gather TO 4; SET parallel_leader_participation TO on;"
        .execute(&mut serial_conn);

    // 1. BaseScan: CostModel (`min_rows_per_worker = 0`) and RowHeuristic (`min_rows_per_worker = 300k`).
    let base_cost_sql = "SELECT id, name FROM ws_users WHERE name === 'bob'";
    let base_heur_sql = "SELECT id, name FROM ws_users WHERE name === 'bob' AND id <> (SELECT max(id) + 1 FROM ws_users)";

    "SET paradedb.min_rows_per_worker TO 0;".execute(&mut serial_conn);
    assert_eq!(
        parse_gather_workers_planned(&explain(
            &mut serial_conn,
            ExplainMode::CostsOff,
            base_cost_sql
        )),
        Some(3)
    );

    "ALTER TABLE ws_users SET (parallel_workers = 0);".execute(&mut serial_conn);
    assert_eq!(
        parse_gather_workers_planned(&explain(
            &mut serial_conn,
            ExplainMode::CostsOff,
            base_cost_sql
        )),
        None
    );

    "RESET paradedb.min_rows_per_worker; ALTER TABLE ws_users SET (parallel_workers = 1);"
        .execute(&mut serial_conn);
    assert_eq!(
        parse_gather_workers_planned(&explain(
            &mut serial_conn,
            ExplainMode::CostsOff,
            base_cost_sql
        )),
        Some(1)
    );

    "ALTER TABLE ws_users SET (parallel_workers = 2);".execute(&mut serial_conn);
    assert_eq!(
        parse_gather_workers_planned(&explain(
            &mut serial_conn,
            ExplainMode::CostsOff,
            base_heur_sql
        )),
        Some(2)
    );

    "ALTER TABLE ws_users RESET (parallel_workers);".execute(&mut serial_conn);
    assert_eq!(
        parse_gather_workers_planned(&explain(
            &mut serial_conn,
            ExplainMode::CostsOff,
            base_heur_sql
        )),
        None
    );

    // 2. MPP (`JoinScan` + `AggregateScan`): per-table caps, mixed `(0, 2)` max, `mpp_min_rows`
    //    bypass when `parallel_workers > 0`, and cached generic `PREPARE` invalidation.
    let join_sql = "SELECT u.id FROM ws_users u JOIN ws_products p ON u.age = p.age \
                    WHERE u.name === 'bob' ORDER BY u.id LIMIT 50";
    let agg_sql = "SELECT p.age, count(*)::bigint FROM ws_users u JOIN ws_products p ON u.age = p.age \
                   WHERE u.name === 'bob' GROUP BY p.age ORDER BY p.age LIMIT 10";

    // Unset under default `mpp_min_rows = 500000` (> 240 rows) stays gated off.
    assert_eq!(
        parse_mpp_launched_workers(&explain(
            &mut serial_conn,
            ExplainMode::AnalyzeVerbose,
            agg_sql
        )),
        None
    );

    // Explicit `parallel_workers > 0` bypasses `mpp_min_rows` and caps worker count.
    for (u_pw, p_pw, leader, expected_workers) in [
        ("0", "0", LeaderMode::Participating, None),
        ("1", "1", LeaderMode::Participating, Some(1)),
        ("1", "1", LeaderMode::CoordinatorOnly, None),
        ("0", "2", LeaderMode::Participating, Some(2)),
    ] {
        format!(
            "ALTER TABLE ws_users SET (parallel_workers = {u_pw});
             ALTER TABLE ws_products SET (parallel_workers = {p_pw});
             SET parallel_leader_participation TO {};",
            leader.as_guc()
        )
        .execute(&mut serial_conn);

        for sql in [join_sql, agg_sql] {
            let analyze = explain(&mut serial_conn, ExplainMode::AnalyzeVerbose, sql);
            assert_eq!(
                parse_mpp_launched_workers(&analyze),
                expected_workers,
                "unexpected workers for ({u_pw}, {p_pw}, {leader:?}):\n{analyze}"
            );
        }
    }

    // Generic `PREPARE` invalidates and replans with updated `parallel_workers` across `ALTER TABLE`.
    r#"
    SET paradedb.mpp_min_rows TO 0;
    SET parallel_leader_participation TO on;
    ALTER TABLE ws_users RESET (parallel_workers);
    ALTER TABLE ws_products RESET (parallel_workers);
    SET plan_cache_mode TO force_generic_plan;
    PREPARE ws_relpw_prep(text) AS
        SELECT u.id FROM ws_users u JOIN ws_products p ON u.age = p.age
        WHERE u.name === $1 ORDER BY u.id LIMIT 20;
    "#
    .execute(&mut serial_conn);

    assert_eq!(
        parse_mpp_launched_workers(&explain(
            &mut serial_conn,
            ExplainMode::AnalyzeVerbose,
            "EXECUTE ws_relpw_prep('bob')"
        )),
        Some(3)
    );

    "ALTER TABLE ws_users SET (parallel_workers = 0); \
     ALTER TABLE ws_products SET (parallel_workers = 0);"
        .execute(&mut serial_conn);
    assert_eq!(
        parse_mpp_launched_workers(&explain(
            &mut serial_conn,
            ExplainMode::AnalyzeVerbose,
            "EXECUTE ws_relpw_prep('bob')"
        )),
        None
    );
}

#[derive(Clone, Copy, Debug)]
enum DebugParallelOutcome {
    Serial,
    DistributedWithLaunchLine,
    DistributedWithoutLaunchLine,
}

#[rstest]
#[case::off("off", DebugParallelOutcome::Serial)]
#[case::on("on", DebugParallelOutcome::DistributedWithLaunchLine)]
#[case::regress("regress", DebugParallelOutcome::DistributedWithoutLaunchLine)]
fn debug_parallel_query_modes(
    mut serial_conn: SerialConn,
    #[case] mode: &str,
    #[case] expected: DebugParallelOutcome,
) {
    if pg_major_version(&mut serial_conn) < 16 {
        return;
    }

    setup_segmented_pair(&mut serial_conn, Segments(2), RowsPerSegment(60));
    format!(
        "SET max_parallel_workers_per_gather TO 4;
         SET parallel_leader_participation TO on;
         SET paradedb.mpp_min_rows TO 1000000000;
         SET debug_parallel_query TO {mode};"
    )
    .execute(&mut serial_conn);

    let (expect_distributed, expect_launch_line) = match expected {
        DebugParallelOutcome::Serial => (false, false),
        DebugParallelOutcome::DistributedWithLaunchLine => (true, true),
        DebugParallelOutcome::DistributedWithoutLaunchLine => (true, false),
    };

    for sql in [
        "SELECT u.id FROM ws_users u JOIN ws_products p ON u.age = p.age WHERE u.name === 'bob' ORDER BY u.id LIMIT 10",
        "SELECT p.age, count(*) FROM ws_users u JOIN ws_products p ON u.age = p.age WHERE u.name === 'bob' GROUP BY p.age ORDER BY p.age LIMIT 5",
    ] {
        let plain = explain(&mut serial_conn, ExplainMode::Verbose, sql);
        let analyze = explain(&mut serial_conn, ExplainMode::AnalyzeVerbose, sql);
        assert_eq!(plain.contains("DistributedExec"), expect_distributed);
        assert_eq!(analyze.contains("DistributedExec"), expect_distributed);
        assert_eq!(analyze.contains("MPP Launch:"), expect_launch_line);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlanShape {
    Serial,
    Distributed,
}

struct RebakeCase {
    prepare_gucs: &'static str,
    query_sql: &'static str,
    steps: &'static [(&'static str, PlanShape)],
}

#[rstest]
#[case::join_mpp_first(RebakeCase {
    prepare_gucs: "SET max_parallel_workers_per_gather TO 2; SET parallel_leader_participation TO on;",
    query_sql: "SELECT u.id::int * 1000 + p.id::int FROM ws_users u JOIN ws_products p ON u.id = p.id \
                WHERE u.name === 'bob' AND p.name @@@ 'item' ORDER BY u.id, p.id LIMIT 5",
    steps: &[
        ("SET max_parallel_workers_per_gather TO 2;", PlanShape::Distributed),
        ("SET max_parallel_workers_per_gather TO 0;", PlanShape::Serial),
        ("SET max_parallel_workers_per_gather TO 2;", PlanShape::Distributed),
    ],
})]
#[case::join_serial_first_leader_toggles(RebakeCase {
    prepare_gucs: "SET max_parallel_workers_per_gather TO 1; SET parallel_leader_participation TO off;",
    query_sql: "SELECT u.id::int * 1000 + p.id::int FROM ws_users u JOIN ws_products p ON u.id = p.id \
                WHERE u.name === 'bob' AND p.name @@@ 'item' ORDER BY u.id, p.id LIMIT 5",
    steps: &[
        ("SET max_parallel_workers_per_gather TO 1; SET parallel_leader_participation TO off;", PlanShape::Serial),
        ("SET parallel_leader_participation TO on;", PlanShape::Distributed),
        ("SET max_parallel_workers_per_gather TO 2; SET parallel_leader_participation TO off;", PlanShape::Distributed),
        ("SET max_parallel_workers_per_gather TO 0; SET parallel_leader_participation TO on;", PlanShape::Serial),
    ],
})]
#[case::agg_6950_non_equi_join(RebakeCase {
    prepare_gucs: "SET paradedb.enable_filter_pushdown TO false; SET paradedb.add_doc_count_to_aggs TO true; \
                   SET max_parallel_workers_per_gather TO 2; SET parallel_leader_participation TO off;",
    query_sql: "SELECT (SUM(u.id) + ROUND(AVG(p.id)))::int FROM ws_users u \
                JOIN ws_products p ON u.id = p.id AND u.age <= p.age \
                WHERE NOT (u.name === 'nobody') GROUP BY u.id ORDER BY 1 LIMIT 5",
    steps: &[
        ("SET max_parallel_workers_per_gather TO 2;", PlanShape::Distributed),
        ("SET max_parallel_workers_per_gather TO 0;", PlanShape::Serial),
        ("SET max_parallel_workers_per_gather TO 2;", PlanShape::Distributed),
    ],
})]
fn generic_plan_rebakes_across_mode_toggles(mut serial_conn: SerialConn, #[case] case: RebakeCase) {
    setup_segmented_pair(&mut serial_conn, Segments(2), RowsPerSegment(50));
    "SET max_parallel_workers_per_gather TO 0;".execute(&mut serial_conn);
    let expected = case.query_sql.fetch::<(i32,)>(&mut serial_conn);
    assert_eq!(expected.len(), 5);

    format!(
        "SET plan_cache_mode TO force_generic_plan;
         {}
         PREPARE rebake_q AS {};",
        case.prepare_gucs, case.query_sql
    )
    .execute(&mut serial_conn);

    for (guc_sql, expect_shape) in case.steps {
        guc_sql.execute(&mut serial_conn);
        let rows = "EXECUTE rebake_q".fetch::<(i32,)>(&mut serial_conn);
        assert_eq!(rows, expected, "row mismatch after `{guc_sql}`");
        let analyze = explain(
            &mut serial_conn,
            ExplainMode::AnalyzeVerbose,
            "EXECUTE rebake_q",
        );
        assert_eq!(
            analyze.contains("DistributedExec"),
            *expect_shape == PlanShape::Distributed,
            "unexpected shape {expect_shape:?} after `{guc_sql}`:\n{analyze}"
        );
    }
}

#[rstest]
fn gather_parallel_hazards_and_portal_fetches(mut serial_conn: SerialConn) {
    setup_segmented_pair(&mut serial_conn, Segments(4), RowsPerSegment(60));
    "SET max_parallel_workers_per_gather TO 4; SET parallel_leader_participation TO on;"
        .execute(&mut serial_conn);

    // 1. `INSERT INTO ... SELECT` (`#6157`, static and generic `PREPARE`) keeps custom scan serial.
    r#"
    CREATE TABLE ws_join_out (id bigint);
    CREATE TABLE ws_agg_out (age int, n bigint);
    "#
    .execute(&mut serial_conn);

    let ins_join = "INSERT INTO ws_join_out SELECT u.id FROM ws_users u JOIN ws_products p ON u.age = p.age WHERE u.name === 'bob' ORDER BY u.id LIMIT 10";
    let ins_agg = "INSERT INTO ws_agg_out SELECT p.age, count(*) FROM ws_users u JOIN ws_products p ON u.age = p.age WHERE u.name === 'bob' GROUP BY p.age ORDER BY p.age LIMIT 5";
    for (scan_label, sql) in [
        ("ParadeDB Join Scan", ins_join),
        ("ParadeDB Aggregate Scan", ins_agg),
    ] {
        let plain = explain(&mut serial_conn, ExplainMode::Verbose, sql);
        assert!(plain.contains(scan_label) && !plain.contains("DistributedExec"));
        sql.execute(&mut serial_conn);
    }
    assert_eq!(
        "SELECT count(*)::bigint FROM ws_join_out".fetch::<(i64,)>(&mut serial_conn),
        vec![(10,)]
    );
    assert_eq!(
        "SELECT count(*)::bigint FROM ws_agg_out".fetch::<(i64,)>(&mut serial_conn),
        vec![(5,)]
    );

    r#"
    TRUNCATE ws_join_out;
    SET plan_cache_mode TO force_generic_plan;
    PREPARE ws_prep_ins(text) AS
        INSERT INTO ws_join_out
        SELECT u.id FROM ws_users u JOIN ws_products p ON u.age = p.age
        WHERE u.name === $1 ORDER BY u.id LIMIT 10;
    "#
    .execute(&mut serial_conn);
    let prep_ins_plan = explain(
        &mut serial_conn,
        ExplainMode::AnalyzeVerbose,
        "EXECUTE ws_prep_ins('bob')",
    );
    assert!(
        prep_ins_plan.contains("ParadeDB Join Scan")
            && !prep_ins_plan.contains("DistributedExec")
            && !prep_ins_plan.contains("MPP Launch:")
    );
    "EXECUTE ws_prep_ins('alice'); DEALLOCATE ws_prep_ins; RESET plan_cache_mode;"
        .execute(&mut serial_conn);
    assert_eq!(
        "SELECT count(*)::bigint FROM ws_join_out".fetch::<(i64,)>(&mut serial_conn),
        vec![(20,)]
    );

    // 2. `TEMP` source table falls back to serial custom scan and releases local buffer pins cleanly
    //    so `DROP TABLE` succeeds in the same session.
    r#"
    CREATE TEMP TABLE ws_temp_users (id bigserial PRIMARY KEY, name text, age int);
    CREATE INDEX ws_temp_users_idx ON ws_temp_users USING paradedb (id, (name::pdb.literal), age)
    WITH (layer_sizes = '0', background_layer_sizes = '0');
    SET paradedb.global_mutable_segment_rows = 0;
    INSERT INTO ws_temp_users (name, age)
    SELECT (ARRAY['bob', 'alice'])[1 + (g % 2)], g % 25 FROM generate_series(1, 60) g;
    INSERT INTO ws_temp_users (name, age)
    SELECT (ARRAY['bob', 'alice'])[1 + (g % 2)], g % 25 FROM generate_series(61, 120) g;
    RESET paradedb.global_mutable_segment_rows;
    ANALYZE ws_temp_users;
    "#
    .execute(&mut serial_conn);

    for (scan_label, sql) in [
        (
            "ParadeDB Join Scan",
            "SELECT u.id FROM ws_temp_users u JOIN ws_products p ON u.age = p.age WHERE u.name === 'bob' ORDER BY u.id LIMIT 10",
        ),
        (
            "ParadeDB Aggregate Scan",
            "SELECT p.age, count(*) FROM ws_temp_users u JOIN ws_products p ON u.age = p.age WHERE u.name === 'bob' GROUP BY p.age ORDER BY p.age LIMIT 5",
        ),
    ] {
        let analyze = explain(&mut serial_conn, ExplainMode::AnalyzeVerbose, sql);
        assert!(
            analyze.contains(scan_label)
                && !analyze.contains("DistributedExec")
                && !analyze.contains("MPP Launch:")
        );
    }
    "DROP TABLE ws_temp_users;".execute(&mut serial_conn);

    // 3. `PARALLEL RESTRICTED` functions, `FOR UPDATE`, and correlated `PARAM_EXEC`:
    //    - Leader-only projection (`ResolvedExpr::PgProjected`) and leader-solved correlated
    //      `PARAM_EXEC` (`LATERAL` / scalar subquery) stay MPP-eligible.
    //    - Worker heap filter, `PostgresExpression` predicate with `PARALLEL RESTRICTED` function,
    //      `SELECT DISTINCT`, and `FOR UPDATE` all run serially.
    r#"
    SELECT set_config('probe.leader_pid', pg_backend_pid()::text, false);

    CREATE FUNCTION ws_restricted_id(x int) RETURNS int
    LANGUAGE plpgsql IMMUTABLE PARALLEL RESTRICTED AS $$
    BEGIN
        IF pg_backend_pid()::text <> current_setting('probe.leader_pid') THEN
            RAISE EXCEPTION 'restricted function ran in parallel worker';
        END IF;
        RETURN x;
    END;
    $$;

    CREATE FUNCTION ws_restricted_term(t text) RETURNS pdb.query
    LANGUAGE plpgsql STABLE PARALLEL RESTRICTED AS $$
    BEGIN
        IF pg_backend_pid()::text <> current_setting('probe.leader_pid') THEN
            RAISE EXCEPTION 'restricted function ran in parallel worker';
        END IF;
        RETURN pdb.term(t);
    END;
    $$;
    "#
    .execute(&mut serial_conn);

    let leader_proj_sql = "SELECT ws_restricted_id(u.id::int) FROM ws_users u \
                           JOIN ws_products p ON u.age = p.age \
                           WHERE u.name === 'bob' ORDER BY u.id LIMIT 10";
    let leader_proj_plan = explain(
        &mut serial_conn,
        ExplainMode::AnalyzeVerbose,
        leader_proj_sql,
    );
    assert!(leader_proj_plan.contains("DistributedExec"));
    assert_eq!(parse_mpp_launched_workers(&leader_proj_plan), Some(3));
    assert_eq!(leader_proj_sql.fetch::<(i32,)>(&mut serial_conn).len(), 10);

    for (expected_scan, serial_hazard_sql) in [
        (
            "ParadeDB Join Scan",
            "SELECT u.id FROM ws_users u JOIN ws_products p ON u.age = p.age \
             WHERE u.name === 'bob' AND ws_restricted_id(p.age) >= 0 ORDER BY u.id LIMIT 10",
        ),
        (
            "ParadeDB Join Scan",
            "SELECT u.id FROM ws_users u \
             JOIN ws_products p ON u.age = p.age AND ws_restricted_id(u.age) <= p.id \
             WHERE u.name === 'bob' ORDER BY u.id LIMIT 10",
        ),
        (
            "ParadeDB Join Scan",
            "SELECT u.id FROM ws_users u \
             WHERE u.name === 'bob' AND EXISTS ( \
                 SELECT 1 FROM ws_products p \
                 WHERE p.age = u.age AND ws_restricted_id(u.age) <= p.id \
             ) ORDER BY u.id LIMIT 10",
        ),
        (
            "ParadeDB Join Scan",
            "SELECT u.id FROM ws_users u JOIN ws_products p ON u.age = p.age \
             WHERE u.name @@@ ws_restricted_term('bob') ORDER BY u.id LIMIT 10",
        ),
        (
            "ParadeDB Aggregate Scan",
            "SELECT p.age, count(*) FROM ws_users u JOIN ws_products p ON u.age = p.age \
             WHERE u.name @@@ ws_restricted_term('bob') GROUP BY p.age ORDER BY p.age LIMIT 5",
        ),
        (
            "ParadeDB Aggregate Scan",
            "SELECT count(*) FROM ws_users u \
             JOIN ws_products p ON u.age = p.age AND ws_restricted_id(u.age) <= p.id \
             WHERE u.name === 'bob'",
        ),
        (
            "ParadeDB Aggregate Scan",
            "SELECT count(*) FROM ws_users u \
             WHERE u.name === 'bob' AND EXISTS ( \
                 SELECT 1 FROM ws_products p \
                 WHERE p.age = u.age AND ws_restricted_id(u.age) <= p.id \
             )",
        ),
        (
            "ParadeDB Join Scan",
            "SELECT DISTINCT ws_restricted_id(u.id::int) FROM ws_users u \
             JOIN ws_products p ON u.age = p.age WHERE u.name === 'bob' AND u.id <= 10 LIMIT 10",
        ),
        (
            "ParadeDB Aggregate Scan",
            "SELECT l.id, (SELECT count(*) FROM ws_users u JOIN ws_products p ON u.age = p.age \
                           WHERE u.name === 'bob') \
             FROM ws_join_out l ORDER BY l.id LIMIT 1 FOR UPDATE",
        ),
    ] {
        let analyze = explain(
            &mut serial_conn,
            ExplainMode::AnalyzeVerbose,
            serial_hazard_sql,
        );
        assert!(
            analyze.contains(expected_scan)
                && !analyze.contains("DistributedExec")
                && !analyze.contains("MPP Launch:"),
            "expected serial `{expected_scan}` execution for `{serial_hazard_sql}`:\n{analyze}"
        );
    }

    let corr_agg_sql = "SELECT v.term, (SELECT count(*)::bigint FROM ws_users u \
                                        JOIN ws_products p ON u.age = p.age \
                                        WHERE u.name @@@ pdb.term(v.term)) \
                        FROM (VALUES ('bob'), ('nobody')) AS v(term)";
    let corr_agg_plan = explain(&mut serial_conn, ExplainMode::AnalyzeVerbose, corr_agg_sql);
    assert!(
        corr_agg_plan.contains("ParadeDB Aggregate Scan")
            && corr_agg_plan.contains("DistributedExec"),
        "expected MPP AggregateScan for correlated PARAM_EXEC:\n{corr_agg_plan}"
    );
    let corr_rows = corr_agg_sql.fetch::<(String, i64)>(&mut serial_conn);
    assert_eq!(corr_rows.len(), 2);
    assert_eq!(corr_rows[0].0, "bob");
    assert!(corr_rows[0].1 > 0);
    assert_eq!(corr_rows[1], ("nobody".to_string(), 0));

    // 4. Partial portal fetches (`!es_use_parallel_mode`): explicit cursor `FETCH` and PL/pgSQL
    //    `FOR` loops (> 50-row SPI portal chunks) must run serially without restarting from row 0.
    r#"
    CREATE FUNCTION ws_verify_partial_portals() RETURNS void LANGUAGE plpgsql AS $$
    DECLARE
        cur refcursor;
        rec record;
        serial_join_sum bigint;
        serial_agg_sum bigint;
        cur_cnt int := 0;
        join_cnt int := 0;
        join_sum bigint := 0;
        agg_cnt int := 0;
        agg_sum bigint := 0;
    BEGIN
        PERFORM set_config('max_parallel_workers_per_gather', '0', true);
        SELECT coalesce(sum(u_id * 1000 + p_id), 0) INTO serial_join_sum FROM (
            SELECT u.id AS u_id, p.id AS p_id
            FROM ws_users u JOIN ws_products p ON u.age = p.age
            WHERE u.name === 'bob' ORDER BY u.id, p.id LIMIT 120
        ) t;
        SELECT coalesce(sum(id * 1000 + cnt), 0) INTO serial_agg_sum FROM (
            SELECT u.id, count(*) AS cnt
            FROM ws_users u JOIN ws_products p ON u.age = p.age
            WHERE u.name === 'bob' GROUP BY u.id ORDER BY u.id LIMIT 120
        ) t;

        PERFORM set_config('max_parallel_workers_per_gather', '4', true);

        OPEN cur NO SCROLL FOR
            SELECT u.id AS u_id, p.id AS p_id
            FROM ws_users u JOIN ws_products p ON u.age = p.age
            WHERE u.name === 'bob' ORDER BY u.id, p.id LIMIT 12;
        LOOP
            FETCH cur INTO rec;
            EXIT WHEN NOT FOUND;
            cur_cnt := cur_cnt + 1;
        END LOOP;
        CLOSE cur;
        IF cur_cnt <> 12 THEN
            RAISE EXCEPTION 'cursor count mismatch: %', cur_cnt;
        END IF;

        FOR rec IN
            SELECT u.id AS u_id, p.id AS p_id
            FROM ws_users u JOIN ws_products p ON u.age = p.age
            WHERE u.name === 'bob' ORDER BY u.id, p.id LIMIT 120
        LOOP
            join_cnt := join_cnt + 1;
            join_sum := join_sum + rec.u_id * 1000 + rec.p_id;
        END LOOP;
        IF join_cnt <> 120 OR join_sum <> serial_join_sum THEN
            RAISE EXCEPTION 'join FOR loop mismatch: cnt=% sum=% expected=%',
                join_cnt, join_sum, serial_join_sum;
        END IF;

        FOR rec IN
            SELECT u.id, count(*) AS cnt
            FROM ws_users u JOIN ws_products p ON u.age = p.age
            WHERE u.name === 'bob' GROUP BY u.id ORDER BY u.id LIMIT 120
        LOOP
            agg_cnt := agg_cnt + 1;
            agg_sum := agg_sum + rec.id * 1000 + rec.cnt;
        END LOOP;
        IF agg_cnt <> 120 OR agg_sum <> serial_agg_sum THEN
            RAISE EXCEPTION 'agg FOR loop mismatch: cnt=% sum=% expected=%',
                agg_cnt, agg_sum, serial_agg_sum;
        END IF;
    END;
    $$;
    SELECT ws_verify_partial_portals();
    "#
    .execute(&mut serial_conn);
}
