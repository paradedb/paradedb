// Copyright (c) 2023-2024 Retake, Inc.
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

//! Tests for ParadeDB's Custom Scan implementation

mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn attribute_1_of_table_has_wrong_type(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let (id,) = "SELECT id, description FROM paradedb.bm25_search WHERE description @@@ 'keyboard' OR id = 1 ORDER BY id LIMIT 1"
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
    let plan = plan
        .get(0)
        .unwrap()
        .as_object()
        .unwrap()
        .get("Plan")
        .unwrap()
        .as_object()
        .unwrap();
    eprintln!("{plan:#?}");
    assert_eq!(
        plan.get("Custom Plan Provider"),
        Some(&Value::String(String::from("ParadeDB Scan")))
    );
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

    let (id,) =
        "SELECT id FROM paradedb.bm25_search WHERE bm25_search @@@ 'description:keyboard' ORDER BY id ASC"
            .fetch_one::<(i32,)>(&mut conn);
    assert_eq!(id, 1);
}

#[rstest]
fn scores_project(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let (id, score) =
        "SELECT id, paradedb.score(bm25_search) FROM paradedb.bm25_search WHERE description @@@ 'keyboard' ORDER BY paradedb.score(bm25_search) DESC LIMIT 1"
            .fetch_one::<(i32, f32)>(&mut conn);
    assert_eq!(id, 2);
    assert_eq!(score, 3.2668595);
}

#[rstest]
fn snippets_project(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let (id, snippet) =
        "SELECT id, paradedb.snippet(bm25_search, 'description') FROM paradedb.bm25_search WHERE description @@@ 'keyboard' ORDER BY paradedb.score(bm25_search) DESC LIMIT 1"
            .fetch_one::<(i32, String)>(&mut conn);
    assert_eq!(id, 2);
    assert_eq!(snippet, String::from("Plastic <b>Keyboard</b>"));
}

#[rstest]
fn scores_and_snippets_project(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let (id, score, snippet) =
        "SELECT id, paradedb.score(bm25_search), paradedb.snippet(bm25_search, 'description') FROM paradedb.bm25_search WHERE description @@@ 'keyboard' ORDER BY paradedb.score(bm25_search) DESC LIMIT 1"
            .fetch_one::<(i32, f32, String)>(&mut conn);
    assert_eq!(id, 2);
    assert_eq!(score, 3.2668595);
    assert_eq!(snippet, String::from("Plastic <b>Keyboard</b>"));
}

#[rstest]
fn mingets(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let (id, snippet) =
        "SELECT id, paradedb.snippet(bm25_search, 'description', '<MING>', '</MING>') FROM paradedb.bm25_search WHERE description @@@ 'teddy bear'"
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
    paradedb.score(bm25_search), 
    rating, 
    paradedb.score(bm25_search) * rating    /* testing this, specifically */
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
fn simple_join_with_scores_and_both_sides(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    let result = r#"
select a.id, 
    a.score, 
    b.id, 
    b.score
from (select paradedb.score(bm25_search), * from paradedb.bm25_search) a
inner join (select paradedb.score(bm25_search), * from paradedb.bm25_search) b on a.id = b.id
where a.description @@@ 'bear' AND b.description @@@ 'teddy bear';"#
        .fetch_one::<(i32, f32, i32, f32)>(&mut conn);
    assert_eq!(result, (40, 3.3322046, 40, 6.664409));
}

#[rstest]
fn simple_join_with_scores_or_both_sides(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // this one doesn't plan a custom scan at all, so scores come back as NaN
    let (a_id, a_score, b_id, b_score) = r#"
select a.id, 
    a.score, 
    b.id, 
    b.score
from (select paradedb.score(bm25_search), * from paradedb.bm25_search) a
inner join (select paradedb.score(bm25_search), * from paradedb.bm25_search) b on a.id = b.id
where a.description @@@ 'bear' OR b.description @@@ 'teddy bear';"#
        .fetch_one::<(i32, f32, i32, f32)>(&mut conn);
    assert!(a_score.is_nan());
    assert!(b_score.is_nan());
    assert_eq!((a_id, b_id), (40, 40));
}
