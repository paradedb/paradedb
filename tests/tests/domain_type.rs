#![allow(dead_code)]
// Copyright (c) 2023-2025 Retake, Inc.
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
use sqlx::PgConnection;

#[fixture]
fn setup_test_table(mut conn: PgConnection) -> PgConnection {
    "CREATE DOMAIN employee_salary_range AS int4range;".execute(&mut conn);
    "CREATE DOMAIN employee_status AS TEXT CHECK (VALUE IN ('active', 'inactive', 'on_leave'));"
        .execute(&mut conn);

    // array of domain type
    "CREATE DOMAIN rating AS INTEGER CHECK (VALUE BETWEEN 1 AND 5);".execute(&mut conn);
    "CREATE DOMAIN rating_history AS rating[];".execute(&mut conn);

    let sql = r#"
        CREATE TABLE employees (
            id SERIAL PRIMARY KEY,
            salary_range employee_salary_range,
            status_history employee_status[],
            ratings rating_history
        );
    "#;
    sql.execute(&mut conn);

    let sql = r#"
        CREATE INDEX idx_employees ON employees USING bm25 (id, salary_range, status_history, ratings)
        WITH (
            key_field='id',
            range_fields='{
                "salary_range": {"fast": true}
            }',
            text_fields='{
                "status_history": {"fast": true}
            }',
            numeric_fields='{
                "ratings": {"fast": true}
            }'
        );
    "#;
    sql.execute(&mut conn);

    "INSERT INTO employees (salary_range, status_history, ratings)
    VALUES
        ('[10000, 50000)', ARRAY['active', 'on_leave'], ARRAY[3, 4]::rating_history),
        ('[50000, 100000)', ARRAY['inactive', 'active'], ARRAY[5, 1]::rating_history),
        ('[20000, 80000)', ARRAY['on_leave', 'inactive'], ARRAY[2, 2, 5]::rating_history);"
        .execute(&mut conn);

    "SET enable_indexscan TO off;".execute(&mut conn);
    "SET enable_bitmapscan TO off;".execute(&mut conn);
    "SET max_parallel_workers TO 0;".execute(&mut conn);

    conn
}

mod domain_types {
    use super::*;

    #[rstest]
    fn verify_index_schema(#[from(setup_test_table)] mut conn: PgConnection) {
        let rows: Vec<(String, String)> =
            "SELECT name, field_type FROM paradedb.schema('idx_employees')".fetch(&mut conn);

        assert_eq!(rows[0], ("ctid".into(), "U64".into()));
        assert_eq!(rows[1], ("id".into(), "I64".into()));
        assert_eq!(rows[2], ("ratings".into(), "I64".into()));
        assert_eq!(rows[3], ("salary_range".into(), "JsonObject".into()));
        assert_eq!(rows[4], ("status_history".into(), "Str".into()));
    }

    #[rstest]
    fn with_range(#[from(setup_test_table)] mut conn: PgConnection) {
        let res: Vec<(i32, i32, i32, String, String)> = r#"
            select id, lower(salary_range), upper(salary_range), status_history::TEXT, ratings::TEXT
            from employees
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            and lower(salary_range) > 15000;
        "#
        .fetch(&mut conn);
        assert_eq!(
            res,
            vec![
                (2, 50000, 100000, "{inactive,active}".into(), "{5,1}".into()),
                (
                    3,
                    20000,
                    80000,
                    "{on_leave,inactive}".into(),
                    "{2,2,5}".into()
                )
            ]
        );

        let count = r#"
            select count(*)
            from employees
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            and lower(salary_range) > 15000;
        "#
        .fetch::<(i64,)>(&mut conn);
        assert_eq!(count, vec![(2,)]);
    }

    #[rstest]
    fn with_array_filter(#[from(setup_test_table)] mut conn: PgConnection) {
        let res: Vec<(i32, i32, i32, String, String)> = r#"
            select id, lower(salary_range), upper(salary_range), status_history::TEXT, ratings::TEXT
            from employees
            where id @@@ paradedb.range(field=> 'id', range=> '[1, 5]'::int8range)
            and 'active' = ANY(status_history);
        "#
        .fetch(&mut conn);
        assert_eq!(
            res,
            vec![
                (1, 10000, 50000, "{active,on_leave}".into(), "{3,4}".into()),
                (2, 50000, 100000, "{inactive,active}".into(), "{5,1}".into())
            ]
        );
    }

    #[rstest]
    fn with_domain_wrapped_array(#[from(setup_test_table)] mut conn: PgConnection) {
        let res: Vec<(i32, String)> = r#"
            SELECT id, ratings::TEXT
            FROM employees
            WHERE 5 = ANY(ratings);
        "#
        .fetch(&mut conn);

        assert_eq!(res, vec![(2, "{5,1}".into()), (3, "{2,2,5}".into())]);
    }

    #[rstest]
    fn reject_invalid_domain_values(#[from(setup_test_table)] mut conn: PgConnection) {
        let result = "INSERT INTO employees (status_history)
                      VALUES (ARRAY['invalid']::status_array);"
            .execute_result(&mut conn);
        assert!(result.is_err());
    }
}
