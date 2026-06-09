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

//! Integration tests for the RTI offset fix (issue #5266) and TEXT-type
//! constant support in `translate_const` (TEXTOID / VARCHAROID / BPCHAROID /
//! NAMEOID).
//!
//! When an AggregateScan on a JOIN is used as a scalar subquery inside a
//! larger outer query, PostgreSQL's `setrefs` pass rewrites the Var nodes in
//! `custom_scan_tlist` with outer-context RTIs.  The fix builds a
//! planning-time `tlist_col_map` so that `AggregateIndexVarMapper` can
//! resolve INDEX_VAR references without any RTI arithmetic.
//!
//! Each test below asserts **parity**: the count returned by the `@@@`
//! aggregate-scan variant must equal the count returned by an equivalent
//! plain-SQL query.

mod fixtures;

use fixtures::db::Query;
use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

// ---------------------------------------------------------------------------
// Setup helpers
// ---------------------------------------------------------------------------

fn setup_text_tables(conn: &mut PgConnection) {
    r#"
    CREATE TABLE rti_p (id INT PRIMARY KEY, name TEXT NOT NULL);
    CREATE TABLE rti_o (id INT PRIMARY KEY, name TEXT NOT NULL);

    INSERT INTO rti_p VALUES (1, 'bob'), (2, 'alice'), (3, 'charlie');
    INSERT INTO rti_o VALUES (1, 'bob'), (2, 'alice'), (3, 'charlie');

    CREATE INDEX rti_p_idx ON rti_p
        USING bm25 (id, name)
        WITH (key_field='id', text_fields='{"name": {"fast": true}}');
    CREATE INDEX rti_o_idx ON rti_o
        USING bm25 (id, name)
        WITH (key_field='id', text_fields='{"name": {"fast": true}}');

    SET paradedb.enable_aggregate_custom_scan TO on;
    "#
    .execute(conn);
}

fn setup_varchar_tables(conn: &mut PgConnection) {
    r#"
    CREATE TABLE vp (id INT PRIMARY KEY, name VARCHAR(50) NOT NULL);
    CREATE TABLE vo (id INT PRIMARY KEY, name VARCHAR(50) NOT NULL);

    INSERT INTO vp VALUES (1, 'bob'), (2, 'alice'), (3, 'charlie');
    INSERT INTO vo VALUES (1, 'bob'), (2, 'alice'), (3, 'charlie');

    CREATE INDEX vp_idx ON vp
        USING bm25 (id, name)
        WITH (key_field='id', text_fields='{"name": {"fast": true}}');
    CREATE INDEX vo_idx ON vo
        USING bm25 (id, name)
        WITH (key_field='id', text_fields='{"name": {"fast": true}}');

    SET paradedb.enable_aggregate_custom_scan TO on;
    "#
    .execute(conn);
}

/// Run `pdb_query` (uses @@@) and `sql_query` (plain SQL) and assert they
/// return the same count. Both queries must return a single (i64,) row.
fn assert_parity(conn: &mut PgConnection, pdb_query: &str, sql_query: &str, label: &str) {
    let (pdb,) = pdb_query.fetch_one::<(i64,)>(conn);
    let (sql,) = sql_query.fetch_one::<(i64,)>(conn);
    assert_eq!(
        pdb, sql,
        "{label}: @@@ result {pdb} != native SQL result {sql}"
    );
}

// ---------------------------------------------------------------------------
// Section 1 – Baseline: no RTI offset
//
// Direct aggregate-on-join (no outer scalar subquery).  The inner RTIs are
// the same as the outer RTIs (no setrefs rewrite), so tlist_col_map lookups
// must resolve cleanly with index 0 for the first table.
// ---------------------------------------------------------------------------

#[rstest]
fn test_rti_offset_baseline_cross_table_or(mut conn: PgConnection) {
    setup_text_tables(&mut conn);

    // Cross-table OR: (o.id = 1 OR p.id = 2) cannot be pushed to either
    // individual scan → becomes a custom_expr in the AggregateScan.
    // Expected: id=1 (o.id=1 ✓) + id=2 (p.id=2 ✓) = 2
    assert_parity(
        &mut conn,
        "SELECT COUNT(*) FROM rti_p p JOIN rti_o o ON p.id = o.id
         WHERE p.name @@@ 'bob OR alice' AND (o.id = 1 OR p.id = 2)",
        "SELECT COUNT(*) FROM rti_p p JOIN rti_o o ON p.id = o.id
         WHERE p.name IN ('bob', 'alice') AND (o.id = 1 OR p.id = 2)",
        "baseline cross-table OR",
    );
}

// ---------------------------------------------------------------------------
// Section 2 – RTI offset = 1
//
// A single-table scalar subquery precedes the agg-on-join subquery.
// `setrefs` rewrites the join subquery's tlist Vars from inner RTIs (1, 2)
// to outer RTIs (2, 3).  The tlist_col_map fix must absorb this shift.
// ---------------------------------------------------------------------------

#[rstest]
fn test_rti_offset_one_extra_table(mut conn: PgConnection) {
    setup_text_tables(&mut conn);

    let (c1, c2) = r#"
        SELECT
          (SELECT COUNT(*) FROM rti_p WHERE name = 'bob'),
          (SELECT COUNT(*) FROM rti_p p JOIN rti_o o ON p.id = o.id
           WHERE p.name @@@ 'bob OR alice' AND (o.id = 1 OR p.id = 2))
    "#
    .fetch_one::<(i64, i64)>(&mut conn);

    assert_eq!(c1, 1, "first subquery: 1 bob");
    assert_eq!(c2, 2, "second subquery with RTI offset 1: 2 rows");
}

#[rstest]
fn test_rti_offset_one_extra_table_parity(mut conn: PgConnection) {
    setup_text_tables(&mut conn);

    assert_parity(
        &mut conn,
        "SELECT (SELECT COUNT(*) FROM rti_p p JOIN rti_o o ON p.id = o.id
                 WHERE p.name @@@ 'bob OR alice' AND (o.id = 1 OR p.id = 2))
         FROM (SELECT COUNT(*) FROM rti_p WHERE name = 'bob') AS sentinel",
        "SELECT (SELECT COUNT(*) FROM rti_p p JOIN rti_o o ON p.id = o.id
                 WHERE p.name IN ('bob', 'alice') AND (o.id = 1 OR p.id = 2))
         FROM (SELECT COUNT(*) FROM rti_p WHERE name = 'bob') AS sentinel",
        "RTI offset 1 parity",
    );
}

// ---------------------------------------------------------------------------
// Section 3 – RTI offset = 2
//
// The first scalar subquery is itself a join (contributes 2 RTI slots).
// The second subquery's inner RTIs (1, 2) become outer RTIs (3, 4).
// ---------------------------------------------------------------------------

#[rstest]
fn test_rti_offset_two_join_outer(mut conn: PgConnection) {
    setup_text_tables(&mut conn);

    let (c1, c2) = r#"
        SELECT
          (SELECT COUNT(*) FROM rti_p p JOIN rti_o o ON p.id = o.id WHERE p.name = 'bob'),
          (SELECT COUNT(*) FROM rti_p p JOIN rti_o o ON p.id = o.id
           WHERE p.name @@@ 'bob OR alice' AND (o.id = 1 OR p.id = 2))
    "#
    .fetch_one::<(i64, i64)>(&mut conn);

    assert_eq!(c1, 1, "first join: 1 bob");
    assert_eq!(c2, 2, "second join with RTI offset 2: 2 rows");
}

// ---------------------------------------------------------------------------
// Section 4 – Exact reproduction of issue #5266
//
// Two scalar subqueries: the first uses plain equality, the second uses @@@
// with a cross-table OR.  Before the fix the second subquery would return 0
// (wrong) because the tlist_col_map resolved p and o to each other's
// columns after the RTI offset.
// ---------------------------------------------------------------------------

#[rstest]
fn test_rti_offset_issue5266_exact(mut conn: PgConnection) {
    r#"
    CREATE TABLE i5266_products (id serial8 NOT NULL PRIMARY KEY, name TEXT);
    CREATE TABLE i5266_orders   (id serial8 NOT NULL PRIMARY KEY, name TEXT);

    INSERT INTO i5266_products (id, name) VALUES (1,'alice'),(2,'bob'),(3,'charlie');
    INSERT INTO i5266_orders   (id, name) VALUES (1,'alice'),(2,'bob'),(3,'charlie');

    CREATE INDEX i5266_pidx ON i5266_products
        USING bm25 (id, name)
        WITH (key_field='id', text_fields='{"name":{"tokenizer":{"type":"keyword"}}}');
    CREATE INDEX i5266_oidx ON i5266_orders
        USING bm25 (id, name)
        WITH (key_field='id', text_fields='{"name":{"tokenizer":{"type":"keyword"}}}');

    SET paradedb.enable_aggregate_custom_scan TO on;
    "#
    .execute(&mut conn);

    // Join on name: 3 matching pairs (alice, bob, charlie).
    // Filter NOT(id=3) OR (name='bob' AND orders.id=3):
    //   id=1 (alice): NOT(1=3)=T → included
    //   id=2 (bob):   NOT(2=3)=T → included
    //   id=3 (charlie): NOT(3=3)=F, (charlie='bob')=F → excluded
    // Expected: 2
    let (plain_sql, pdb) = r#"
        SELECT
          (SELECT COUNT(*) FROM i5266_products JOIN i5266_orders
           ON i5266_products.name = i5266_orders.name
           WHERE (NOT (i5266_products.id = '3'))
              OR ((i5266_products.name = 'bob') AND (i5266_orders.id = '3'))),
          (SELECT COUNT(*) FROM i5266_products JOIN i5266_orders
           ON i5266_products.name = i5266_orders.name
           WHERE (NOT (i5266_products.id @@@ '3'))
              OR ((i5266_products.name @@@ 'bob') AND (i5266_orders.id @@@ '3')))
    "#
    .fetch_one::<(i64, i64)>(&mut conn);

    assert_eq!(plain_sql, 2, "plain SQL must return 2");
    assert_eq!(
        pdb, plain_sql,
        "@@@ result must match plain SQL (issue #5266)"
    );
}

// ---------------------------------------------------------------------------
// Section 5 – TEXT literal constants in cross-table predicates
//
// Tests `translate_const` for TEXTOID values: (o.name = 'alice' OR
// p.name = 'bob') contains two text string constants that must be mapped to
// ScalarValue::Utf8 by the DataFusion translator.
// ---------------------------------------------------------------------------

#[rstest]
fn test_translate_const_text_literals(mut conn: PgConnection) {
    setup_text_tables(&mut conn);

    // Outer single-table subquery adds RTI offset 1.
    // Cross-table OR with text literals: o.name='alice' (right side) or p.name='bob' (left side).
    // Matching rows: id=1 (p.name='bob' ✓) + id=2 (o.name='alice' ✓) = 2
    let (c1, c2) = r#"
        SELECT
          (SELECT COUNT(*) FROM rti_p WHERE name = 'bob'),
          (SELECT COUNT(*) FROM rti_p p JOIN rti_o o ON p.id = o.id
           WHERE p.name @@@ 'bob OR alice'
             AND (o.name = 'alice' OR p.name = 'bob'))
    "#
    .fetch_one::<(i64, i64)>(&mut conn);

    assert_eq!(c1, 1, "sentinel subquery");
    assert_eq!(c2, 2, "text literal cross-table OR: 2 rows");
}

// ---------------------------------------------------------------------------
// Section 6 – VARCHAR columns (triggers RelabelType in unwrap_to_var)
//
// When a JOIN column is VARCHAR the planner may wrap the tlist Var in a
// RelabelType (binary-compatible cast varchar→text).  `unwrap_to_var` must
// peel this cast to reach the underlying Var and compute the correct
// DataFusion column name.
// ---------------------------------------------------------------------------

#[rstest]
fn test_varchar_columns_rti_offset(mut conn: PgConnection) {
    setup_varchar_tables(&mut conn);

    // RTI offset scenario with VARCHAR columns.
    let (c1, c2) = r#"
        SELECT
          (SELECT COUNT(*) FROM vp WHERE name = 'bob'),
          (SELECT COUNT(*) FROM vp p JOIN vo o ON p.id = o.id
           WHERE p.name @@@ 'bob OR alice' AND (o.id = 1 OR p.id = 2))
    "#
    .fetch_one::<(i64, i64)>(&mut conn);

    assert_eq!(c1, 1, "sentinel");
    assert_eq!(c2, 2, "varchar columns with RTI offset 1");
}

#[rstest]
fn test_varchar_text_literal_predicate(mut conn: PgConnection) {
    setup_varchar_tables(&mut conn);

    // VARCHAR column compared to a string literal — exercises VARCHAROID
    // handling in translate_const AND RelabelType stripping in unwrap_to_var.
    let (c1, c2) = r#"
        SELECT
          (SELECT COUNT(*) FROM vp WHERE name = 'bob'),
          (SELECT COUNT(*) FROM vp p JOIN vo o ON p.id = o.id
           WHERE p.name @@@ 'bob OR alice'
             AND (o.name = 'alice' OR p.name = 'bob'))
    "#
    .fetch_one::<(i64, i64)>(&mut conn);

    assert_eq!(c1, 1, "sentinel");
    assert_eq!(c2, 2, "varchar text-literal cross-table OR: 2 rows");
}

// ---------------------------------------------------------------------------
// Section 7 – BPCHAR constant in a cross-table predicate (translate_const)
//
// commit bd6f73c added BPCHAROID / VARCHAROID / NAMEOID handling to
// translate_const.  BM25 cannot index BPCHAR columns, but a cross-table
// predicate can still contain a BPCHAR *constant* when the comparison
// column is TEXT and the literal is explicitly cast with ::bpchar.
//
// PostgreSQL's implicit-cast rules convert the BPCHAR literal to TEXT for
// the = operator, but only AFTER type-checking; the Const node that lands
// in the DataFusion custom_expr still carries consttype = BPCHAROID in some
// planner paths.  We verify correctness by asserting parity with plain SQL.
// ---------------------------------------------------------------------------

#[rstest]
fn test_bpchar_constant_cross_table_predicate(mut conn: PgConnection) {
    setup_text_tables(&mut conn);

    // The predicate (o.name = 'alice'::bpchar OR p.name = 'bob'::bpchar)
    // may produce BPCHAR Const nodes depending on the planner's type
    // resolution.  Either way the result must match the plain-SQL baseline.
    assert_parity(
        &mut conn,
        "SELECT COUNT(*) FROM rti_p p JOIN rti_o o ON p.id = o.id
         WHERE p.name @@@ 'bob OR alice'
           AND (o.name = 'alice'::bpchar OR p.name = 'bob'::bpchar)",
        "SELECT COUNT(*) FROM rti_p p JOIN rti_o o ON p.id = o.id
         WHERE p.name IN ('bob', 'alice')
           AND (o.name = 'alice' OR p.name = 'bob')",
        "bpchar constant cross-table predicate",
    );
}

#[rstest]
fn test_bpchar_constant_with_rti_offset(mut conn: PgConnection) {
    setup_text_tables(&mut conn);

    // RTI offset 1 + BPCHAR constant: ensures translate_const handles
    // BPCHAROID correctly under an RTI-shifted INDEX_VAR mapping.
    let (c1, c2) = r#"
        SELECT
          (SELECT COUNT(*) FROM rti_p WHERE name = 'bob'),
          (SELECT COUNT(*) FROM rti_p p JOIN rti_o o ON p.id = o.id
           WHERE p.name @@@ 'bob OR alice'
             AND (o.name = 'alice'::bpchar OR p.name = 'bob'::bpchar))
    "#
    .fetch_one::<(i64, i64)>(&mut conn);

    assert_eq!(c1, 1, "sentinel");
    assert_eq!(c2, 2, "bpchar constant with RTI offset 1: 2 rows");
}

// ---------------------------------------------------------------------------
// Section 8 – Agg-scan on/off parity
//
// The RTI offset fix must not change results for queries that were already
// handled correctly.  Run each scenario with agg-scan on and off and assert
// the counts agree.
// ---------------------------------------------------------------------------

#[rstest]
fn test_agg_scan_on_off_agree(mut conn: PgConnection) {
    setup_text_tables(&mut conn);

    let pdb_q = "SELECT COUNT(*) FROM rti_p p JOIN rti_o o ON p.id = o.id
                 WHERE p.name @@@ 'bob OR alice' AND (o.id = 1 OR p.id = 2)";
    let native_q = "SELECT COUNT(*) FROM rti_p p JOIN rti_o o ON p.id = o.id
                    WHERE p.name IN ('bob', 'alice') AND (o.id = 1 OR p.id = 2)";

    "SET paradedb.enable_aggregate_custom_scan TO on;".execute(&mut conn);
    let (on,) = pdb_q.fetch_one::<(i64,)>(&mut conn);

    "SET paradedb.enable_aggregate_custom_scan TO off;".execute(&mut conn);
    let (native,) = native_q.fetch_one::<(i64,)>(&mut conn);

    assert_eq!(on, native, "agg-scan on/off must agree: {on} vs {native}");
}

// ---------------------------------------------------------------------------
// Section 9 – Multiple sequential scalar subqueries (RTI offset ≥ 2)
//
// Two single-table subqueries before the join subquery each add one RTI
// slot, so the join's inner RTIs (1,2) become outer RTIs (3,4).
// ---------------------------------------------------------------------------

#[rstest]
fn test_rti_offset_three_subqueries(mut conn: PgConnection) {
    setup_text_tables(&mut conn);

    let (c1, c2, c3) = r#"
        SELECT
          (SELECT COUNT(*) FROM rti_p WHERE name = 'bob'),
          (SELECT COUNT(*) FROM rti_o WHERE name = 'alice'),
          (SELECT COUNT(*) FROM rti_p p JOIN rti_o o ON p.id = o.id
           WHERE p.name @@@ 'bob OR alice' AND (o.id = 1 OR p.id = 2))
    "#
    .fetch_one::<(i64, i64, i64)>(&mut conn);

    assert_eq!(c1, 1, "first sentinel: bob");
    assert_eq!(c2, 1, "second sentinel: alice");
    assert_eq!(c3, 2, "join with RTI offset 2: 2 rows");
}

// ---------------------------------------------------------------------------
// Section 10 – Self-join: same table on both sides of the join
//
// `build_tlist_col_map` must distinguish the two instances of the same
// relation by RTI, not just by relation OID — both sources share a
// `heaprelid` but have distinct execution aliases.  An OID-only lookup
// always picks plan position 0, silently resolving `b.id` to `a`'s column.
//
// Data is asymmetric so the wrong mapping yields a different count:
//   rti_s: (1,'red'), (2,'red'), (3,'red'), (4,'blue')
//   self-join on name → 9 'red' pairs + 1 'blue' pair
//   (b.id = 1 OR a.id = 2): {(1,1),(2,1),(3,1)} ∪ {(2,1),(2,2),(2,3)} = 5
//   misresolved (a.id = 1 OR a.id = 2): 3 + 3 = 6
//
// The join is deliberately on `name` while the OR predicate is on `id`;
// joining on `id` would force a.id = b.id and mask the misresolution.
// ---------------------------------------------------------------------------

fn setup_self_join_table(conn: &mut PgConnection) {
    r#"
    CREATE TABLE rti_s (id INT PRIMARY KEY, name TEXT NOT NULL);

    INSERT INTO rti_s VALUES (1, 'red'), (2, 'red'), (3, 'red'), (4, 'blue');

    CREATE INDEX rti_s_idx ON rti_s
        USING bm25 (id, name)
        WITH (key_field='id', text_fields='{"name": {"fast": true}}');

    SET paradedb.enable_aggregate_custom_scan TO on;
    "#
    .execute(conn);
}

#[rstest]
fn test_self_join_cross_table_or(mut conn: PgConnection) {
    setup_self_join_table(&mut conn);

    let pdb_q = "SELECT COUNT(*) FROM rti_s a JOIN rti_s b ON a.name = b.name
                 WHERE a.name @@@ 'red OR blue' AND (b.id = 1 OR a.id = 2)";

    // Guard: the @@@ variant must actually go through the aggregate custom
    // scan, otherwise this test exercises nothing.
    let (plan,) =
        format!("EXPLAIN (FORMAT JSON) {pdb_q}").fetch_one::<(serde_json::Value,)>(&mut conn);
    assert!(
        plan.to_string().contains("ParadeDB Aggregate Scan"),
        "self-join @@@ query must use the aggregate custom scan; plan: {plan:#?}"
    );

    let (count,) = pdb_q.fetch_one::<(i64,)>(&mut conn);
    assert_eq!(count, 5, "self-join cross-table OR: exact count");

    assert_parity(
        &mut conn,
        pdb_q,
        "SELECT COUNT(*) FROM rti_s a JOIN rti_s b ON a.name = b.name
         WHERE a.name IN ('red', 'blue') AND (b.id = 1 OR a.id = 2)",
        "self-join cross-table OR",
    );
}

#[rstest]
fn test_self_join_with_rti_shift(mut conn: PgConnection) {
    setup_self_join_table(&mut conn);

    // A preceding scalar subquery shifts the self-join subquery's outer
    // RTIs, combining both failure modes (RTI shift + same-OID sources).
    let (c1, c2) = r#"
        SELECT
          (SELECT COUNT(*) FROM rti_s WHERE name = 'blue'),
          (SELECT COUNT(*) FROM rti_s a JOIN rti_s b ON a.name = b.name
           WHERE a.name @@@ 'red OR blue' AND (b.id = 1 OR a.id = 2))
    "#
    .fetch_one::<(i64, i64)>(&mut conn);

    assert_eq!(c1, 1, "sentinel: blue");
    assert_eq!(c2, 5, "self-join under RTI shift: exact count");
}
