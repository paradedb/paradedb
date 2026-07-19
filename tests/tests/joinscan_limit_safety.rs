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

//! JoinScan pushes the query's LIMIT into its scan. That is only safe when
//! nothing above the join needs more rows than the LIMIT keeps (issue #5561:
//! `count(*) OVER ()` returned the LIMIT instead of the true match count).
//! These tests pin the decline cases and the cases that must keep using
//! the JoinScan.

use rstest::*;
use sqlx::PgConnection;
use tests::fixtures::*;

const JOIN_SCAN: &str = "Custom Scan (ParadeDB Join Scan)";

// parent: ids 1..2000, two thirds are 'manga'. child: exactly two children
// per parent 1..1000. The manga join therefore matches 2 * 667 = 1334 rows.
fn setup(conn: &mut PgConnection) {
    r#"
    SET paradedb.enable_custom_scan = on;
    SET paradedb.enable_join_custom_scan = on;
    SET max_parallel_workers_per_gather = 0;
    SET enable_hashjoin = off;
    SET enable_mergejoin = off;
    SET enable_nestloop = off;

    DROP TABLE IF EXISTS ls_parent;
    DROP TABLE IF EXISTS ls_child;
    CREATE TABLE ls_parent (id int PRIMARY KEY, kind text);
    CREATE TABLE ls_child  (id bigint PRIMARY KEY, parent_id bigint);

    INSERT INTO ls_parent
    SELECT g, CASE WHEN g % 3 = 0 THEN 'novel' ELSE 'manga' END
    FROM generate_series(1, 2000) g;
    INSERT INTO ls_child SELECT g, ((g - 1) % 1000) + 1 FROM generate_series(1, 2000) g;

    CREATE INDEX ls_parent_bm25 ON ls_parent USING bm25 (id, kind) WITH (key_field = 'id');
    CREATE INDEX ls_child_bm25  ON ls_child  USING bm25 (id, parent_id) WITH (key_field = 'id');
    ANALYZE ls_parent;
    ANALYZE ls_child;
    "#
    .execute(conn);
}

fn explain(conn: &mut PgConnection, query: &str) -> String {
    let lines: Vec<String> = format!("EXPLAIN (COSTS OFF) {query}").fetch_scalar(conn);
    lines.join("\n")
}

#[derive(Clone, Copy)]
enum LimitSafetyCase {
    WindowCount,
    RowReducingSrf,
    GroupBy,
    PlainPagination,
    ParameterizedLimit,
    WindowAboveSubquery,
    PlainDistinct,
}

#[rstest]
#[case::window_count(LimitSafetyCase::WindowCount)]
#[case::row_reducing_srf(LimitSafetyCase::RowReducingSrf)]
#[case::group_by(LimitSafetyCase::GroupBy)]
#[case::plain_pagination(LimitSafetyCase::PlainPagination)]
#[case::parameterized_limit(LimitSafetyCase::ParameterizedLimit)]
#[case::window_above_subquery(LimitSafetyCase::WindowAboveSubquery)]
#[case::plain_distinct(LimitSafetyCase::PlainDistinct)]
fn limit_pushdown_safety(
    #[case] case: LimitSafetyCase,
    mut conn: PgConnection,
) -> Result<(), sqlx::Error> {
    setup(&mut conn);

    match case {
        LimitSafetyCase::WindowCount => {
            // count(*) OVER () must count all 1334 joined rows, so the LIMIT cannot
            // be pushed below it and the JoinScan must decline.
            let query = r#"
                SELECT p.id, count(*) OVER () AS total
                FROM ls_parent p JOIN ls_child c ON c.parent_id = p.id
                WHERE p.kind @@@ pdb.term('manga') AND c.id @@@ pdb.all()
                ORDER BY p.id, c.id
                LIMIT 5
            "#;

            assert!(!explain(&mut conn, query).contains(JOIN_SCAN));

            let rows = query.fetch_result::<(i32, i64)>(&mut conn)?;
            assert_eq!(rows.len(), 5);
            assert!(rows.iter().all(|(_, total)| *total == 1334), "{rows:?}");
        }
        LimitSafetyCase::RowReducingSrf => {
            // The SRF deletes odd ids (empty array), so filling LIMIT 5 needs more
            // than 5 join rows; a pushed LIMIT would come up short.
            let query = r#"
                SELECT p.id, unnest(CASE WHEN p.id % 2 = 0 THEN ARRAY[1] ELSE '{}'::int[] END) AS u
                FROM ls_parent p JOIN ls_child c ON c.parent_id = p.id
                WHERE p.kind @@@ pdb.term('manga') AND c.id @@@ pdb.all()
                ORDER BY p.id, c.id
                LIMIT 5
            "#;

            assert!(!explain(&mut conn, query).contains(JOIN_SCAN));

            let rows = query.fetch_result::<(i32, i32)>(&mut conn)?;
            assert_eq!(
                rows.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
                vec![2, 2, 4, 4, 8]
            );
        }
        LimitSafetyCase::GroupBy => {
            // With the aggregate scan disabled, the JoinScan is the only custom
            // candidate; it must decline rather than cap the rows feeding the Group
            // node (each parent has two children, so 5 groups need 10 join rows).
            "SET paradedb.enable_aggregate_custom_scan = off;".execute(&mut conn);

            let query = r#"
                SELECT p.id
                FROM ls_parent p JOIN ls_child c ON c.parent_id = p.id
                WHERE p.kind @@@ pdb.term('manga') AND c.id @@@ pdb.all()
                GROUP BY p.id
                ORDER BY p.id
                LIMIT 5
            "#;

            assert!(!explain(&mut conn, query).contains(JOIN_SCAN));

            let rows = query.fetch_result::<(i32,)>(&mut conn)?;
            assert_eq!(rows, vec![(1,), (2,), (4,), (5,), (7,)]);
        }
        LimitSafetyCase::PlainPagination => {
            // The no-regression guard: without a row-consuming node above the join,
            // the LIMIT push stays legal and the JoinScan must keep engaging.
            let query = r#"
                SELECT p.id
                FROM ls_parent p JOIN ls_child c ON c.parent_id = p.id
                WHERE p.kind @@@ pdb.term('manga') AND c.id @@@ pdb.all()
                ORDER BY p.id
                LIMIT 5
            "#;

            assert!(explain(&mut conn, query).contains(JOIN_SCAN));

            let rows = query.fetch_result::<(i32,)>(&mut conn)?;
            assert_eq!(rows, vec![(1,), (1,), (2,), (2,), (4,)]);
        }
        LimitSafetyCase::ParameterizedLimit => {
            // A generic plan leaves the LIMIT as a Param and PG reports
            // limit_tuples == -1; that alone must not disable the JoinScan.
            r#"
            SET plan_cache_mode = force_generic_plan;
            PREPARE ls_page AS
                SELECT p.id
                FROM ls_parent p JOIN ls_child c ON c.parent_id = p.id
                WHERE p.kind @@@ pdb.term('manga') AND c.id @@@ pdb.all()
                ORDER BY p.id
                LIMIT $1;
            "#
            .execute(&mut conn);

            let plan: Vec<String> =
                "EXPLAIN (COSTS OFF) EXECUTE ls_page(5)".fetch_scalar(&mut conn);
            assert!(plan.join("\n").contains(JOIN_SCAN), "{}", plan.join("\n"));
        }
        LimitSafetyCase::WindowAboveSubquery => {
            // The gate is per query level: the window function lives in the OUTER
            // query, while the LIMIT the JoinScan pushes belongs to the subquery.
            // Counting after the inner LIMIT is correct SQL, so the inner JoinScan
            // must keep engaging and the window total must equal the inner limit.
            let query = r#"
                SELECT sub.id, count(*) OVER () AS total
                FROM (
                    SELECT p.id
                    FROM ls_parent p JOIN ls_child c ON c.parent_id = p.id
                    WHERE p.kind @@@ pdb.term('manga') AND c.id @@@ pdb.all()
                    ORDER BY p.id, c.id
                    LIMIT 10
                ) sub
                LIMIT 5
            "#;

            assert!(explain(&mut conn, query).contains(JOIN_SCAN));

            let rows = query.fetch_result::<(i32, i64)>(&mut conn)?;
            assert_eq!(rows.len(), 5);
            assert!(rows.iter().all(|(_, total)| *total == 10), "{rows:?}");
        }
        LimitSafetyCase::PlainDistinct => {
            // Plain DISTINCT dedups on the whole target list, which the JoinScan
            // absorbs and applies before its limit, so the pushdown stays safe.
            let query = r#"
                SELECT DISTINCT p.id
                FROM ls_parent p JOIN ls_child c ON c.parent_id = p.id
                WHERE p.kind @@@ pdb.term('manga') AND c.id @@@ pdb.all()
                ORDER BY p.id
                LIMIT 5
            "#;

            assert!(explain(&mut conn, query).contains(JOIN_SCAN));

            let rows = query.fetch_result::<(i32,)>(&mut conn)?;
            assert_eq!(rows, vec![(1,), (2,), (4,), (5,), (7,)]);
        }
    }

    Ok(())
}
