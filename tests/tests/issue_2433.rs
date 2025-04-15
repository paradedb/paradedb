// Copyright (c) 2023-2025 ParadeDB, Inc.
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

/// Test for issue #2433: Pushdown `bool_field IS true|false`
/// Verifies that the SQL IS operator for boolean fields is properly
/// pushed down to the ParadeDB scan operator.
#[rstest]
fn test_bool_is_operator_pushdown(mut conn: PgConnection) {
    r#"
    DROP TABLE IF EXISTS is_true;
    CREATE TABLE is_true (
        id serial8 not null primary key,
        bool_field boolean,
        message text
    );

    CREATE INDEX idxis_true ON is_true USING bm25 (id, bool_field, message) WITH (key_field = 'id');

    INSERT INTO is_true (bool_field, message) VALUES (true, 'beer');
    INSERT INTO is_true (bool_field, message) VALUES (false, 'beer');
    "#
    .execute(&mut conn);

    // Test with IS true
    {
        let sql = r#"
            EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
            SELECT *, paradedb.score(id) FROM is_true 
            WHERE bool_field IS true AND message @@@ 'beer';
        "#;

        eprintln!("{sql}");
        let (plan,) = sql.fetch_one::<(Value,)>(&mut conn);
        eprintln!("{plan:#?}");

        // Verify we're using a custom scan (ParadeDB Scan)
        let plan_node = plan
            .pointer("/0/Plan/Plans/0")
            .unwrap()
            .as_object()
            .unwrap();
        let node_type = plan_node.get("Node Type").unwrap().as_str().unwrap();

        // The node type should be either "Custom Scan" directly or "Gather" with a Custom Scan child
        // Depending on PostgreSQL's decision to use parallelism
        if node_type == "Custom Scan" {
            assert_eq!("Custom Scan", node_type);
        } else {
            assert_eq!("Gather", node_type);
            let child_node = plan_node
                .get("Plans")
                .unwrap()
                .as_array()
                .unwrap()
                .get(0)
                .unwrap()
                .as_object()
                .unwrap();
            assert_eq!(
                "Custom Scan",
                child_node.get("Node Type").unwrap().as_str().unwrap()
            );
        }

        // Actually query with IS true to verify results
        let results: Vec<(i64, bool, String, f32)> = r#"
            SELECT id, bool_field, message, paradedb.score(id)
            FROM is_true 
            WHERE bool_field IS true AND message @@@ 'beer'
            ORDER BY id;
        "#
        .fetch(&mut conn);

        assert_eq!(1, results.len());
        assert_eq!(1, results[0].0); // id
        assert_eq!(true, results[0].1); // bool_field
        assert_eq!("beer", results[0].2); // message
    }

    // Test with IS false
    {
        let sql = r#"
            EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
            SELECT *, paradedb.score(id) FROM is_true 
            WHERE bool_field IS false AND message @@@ 'beer';
        "#;

        eprintln!("{sql}");
        let (plan,) = sql.fetch_one::<(Value,)>(&mut conn);
        eprintln!("{plan:#?}");

        // Verify we're using a custom scan (ParadeDB Scan)
        let plan_node = plan
            .pointer("/0/Plan/Plans/0")
            .unwrap()
            .as_object()
            .unwrap();
        let node_type = plan_node.get("Node Type").unwrap().as_str().unwrap();

        if node_type == "Custom Scan" {
            assert_eq!("Custom Scan", node_type);
        } else {
            assert_eq!("Gather", node_type);
            let child_node = plan_node
                .get("Plans")
                .unwrap()
                .as_array()
                .unwrap()
                .get(0)
                .unwrap()
                .as_object()
                .unwrap();
            assert_eq!(
                "Custom Scan",
                child_node.get("Node Type").unwrap().as_str().unwrap()
            );
        }

        // Actually query with IS false to verify results
        let results: Vec<(i64, bool, String, f32)> = r#"
            SELECT id, bool_field, message, paradedb.score(id)
            FROM is_true 
            WHERE bool_field IS false AND message @@@ 'beer'
            ORDER BY id;
        "#
        .fetch(&mut conn);

        assert_eq!(1, results.len());
        assert_eq!(2, results[0].0); // id
        assert_eq!(false, results[0].1); // bool_field
        assert_eq!("beer", results[0].2); // message
    }

    // Test with IS NOT TRUE
    {
        let sql = r#"
            EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
            SELECT *, paradedb.score(id) FROM is_true 
            WHERE bool_field IS NOT true AND message @@@ 'beer';
        "#;

        eprintln!("{sql}");
        let (plan,) = sql.fetch_one::<(Value,)>(&mut conn);
        eprintln!("{plan:#?}");

        // Verify we're using a custom scan (ParadeDB Scan)
        let plan_node = plan
            .pointer("/0/Plan/Plans/0")
            .unwrap()
            .as_object()
            .unwrap();
        let node_type = plan_node.get("Node Type").unwrap().as_str().unwrap();

        if node_type == "Custom Scan" {
            assert_eq!("Custom Scan", node_type);
        } else {
            assert_eq!("Gather", node_type);
            let child_node = plan_node
                .get("Plans")
                .unwrap()
                .as_array()
                .unwrap()
                .get(0)
                .unwrap()
                .as_object()
                .unwrap();
            assert_eq!(
                "Custom Scan",
                child_node.get("Node Type").unwrap().as_str().unwrap()
            );
        }

        // Actually query with IS NOT TRUE to verify results
        let results: Vec<(i64, bool, String, f32)> = r#"
            SELECT id, bool_field, message, paradedb.score(id)
            FROM is_true 
            WHERE bool_field IS NOT true AND message @@@ 'beer'
            ORDER BY id;
        "#
        .fetch(&mut conn);

        assert_eq!(1, results.len());
        assert_eq!(2, results[0].0); // id
        assert_eq!(false, results[0].1); // bool_field
        assert_eq!("beer", results[0].2); // message
    }

    // Test with IS NOT FALSE
    {
        let sql = r#"
            EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
            SELECT *, paradedb.score(id) FROM is_true 
            WHERE bool_field IS NOT false AND message @@@ 'beer';
        "#;

        eprintln!("{sql}");
        let (plan,) = sql.fetch_one::<(Value,)>(&mut conn);
        eprintln!("{plan:#?}");

        // Verify we're using a custom scan (ParadeDB Scan)
        let plan_node = plan
            .pointer("/0/Plan/Plans/0")
            .unwrap()
            .as_object()
            .unwrap();
        let node_type = plan_node.get("Node Type").unwrap().as_str().unwrap();

        if node_type == "Custom Scan" {
            assert_eq!("Custom Scan", node_type);
        } else {
            assert_eq!("Gather", node_type);
            let child_node = plan_node
                .get("Plans")
                .unwrap()
                .as_array()
                .unwrap()
                .get(0)
                .unwrap()
                .as_object()
                .unwrap();
            assert_eq!(
                "Custom Scan",
                child_node.get("Node Type").unwrap().as_str().unwrap()
            );
        }

        // Actually query with IS NOT FALSE to verify results
        let results: Vec<(i64, bool, String, f32)> = r#"
            SELECT id, bool_field, message, paradedb.score(id)
            FROM is_true 
            WHERE bool_field IS NOT false AND message @@@ 'beer'
            ORDER BY id;
        "#
        .fetch(&mut conn);

        assert_eq!(1, results.len());
        assert_eq!(1, results[0].0); // id
        assert_eq!(true, results[0].1); // bool_field
        assert_eq!("beer", results[0].2); // message
    }
}
