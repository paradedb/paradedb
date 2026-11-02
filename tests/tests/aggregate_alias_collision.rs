// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use bigdecimal::BigDecimal;
use rstest::rstest;
use serde_json::Value;
use sqlx::PgConnection;
use tests::fixtures::*;

#[rstest]
fn aggregate_scan_handles_internal_alias_collisions(mut conn: PgConnection) {
    r#"
        SET paradedb.enable_aggregate_custom_scan TO on;
        SET max_parallel_workers_per_gather TO 0;
        CREATE TABLE alias_items (
            id INTEGER PRIMARY KEY,
            agg_0 INTEGER,
            ctid_0 INTEGER,
            _ctid_0 INTEGER,
            __ctid_0 INTEGER,
            __pdb_k0 INTEGER,
            __pdb_m0 INTEGER,
            n NUMERIC(10, 2)
        );
        INSERT INTO alias_items
            SELECT g, g % 3, g % 3, g % 3, g % 3, g % 3, g % 3, g
            FROM generate_series(1, 60) AS g;
        CREATE INDEX alias_items_idx ON alias_items
            USING paradedb (id, agg_0, ctid_0, _ctid_0, __ctid_0, __pdb_k0, __pdb_m0, n);
    "#
    .execute(&mut conn);

    for column in ["agg_0", "ctid_0", "_ctid_0", "__ctid_0"] {
        let query = format!(
            "SELECT {column}, SUM(n) FROM alias_items WHERE id @@@ pdb.all() \
             GROUP BY {column} ORDER BY {column}"
        );
        let (plan,) = format!("EXPLAIN (FORMAT JSON) {query}").fetch_one::<(Value,)>(&mut conn);
        let plan = plan.to_string();
        assert!(plan.contains("ParadeDB Aggregate Scan"), "{column}: {plan}");
        assert!(
            plan.contains("DataFusion Physical Plan"),
            "{column}: {plan}"
        );

        let rows: Vec<(i32, BigDecimal)> = query.fetch(&mut conn);
        assert_eq!(
            rows,
            vec![
                (0, BigDecimal::from(630)),
                (1, BigDecimal::from(590)),
                (2, BigDecimal::from(610)),
            ],
            "incorrect results for {column}"
        );
    }

    let query = r#"
        SELECT __pdb_k0, SUM(n), pdb.agg('{"terms": {"field": "__pdb_m0"}}')
        FROM alias_items WHERE id @@@ pdb.all()
        GROUP BY __pdb_k0 ORDER BY __pdb_k0
    "#;
    let (plan,) = format!("EXPLAIN (FORMAT JSON) {query}").fetch_one::<(Value,)>(&mut conn);
    let plan = plan.to_string();
    assert!(plan.contains("ParadeDB Aggregate Scan"), "{plan}");
    assert!(plan.contains("DataFusion Physical Plan"), "{plan}");

    let rows: Vec<(i32, BigDecimal, Value)> = query.fetch(&mut conn);
    assert_eq!(rows.len(), 3);
    for ((key, sum, aggregate), (expected_key, expected_sum)) in
        rows.iter().zip([(0, 630), (1, 590), (2, 610)])
    {
        assert_eq!(*key, expected_key);
        assert_eq!(*sum, BigDecimal::from(expected_sum));
        let buckets = aggregate["buckets"].as_array().expect("terms buckets");
        assert_eq!(buckets.len(), 1);
        assert_eq!(buckets[0]["key"].as_i64(), Some(i64::from(*key)));
        assert_eq!(buckets[0]["doc_count"].as_u64(), Some(20));
    }
}

