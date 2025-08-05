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

use crate::fixtures::querygen::groupbygen::arb_group_by;
use crate::fixtures::querygen::joingen::JoinType;
use crate::fixtures::querygen::pagegen::arb_paging_exprs;
use crate::fixtures::querygen::wheregen::arb_wheres;
use crate::fixtures::querygen::{arb_joins_and_wheres, compare, PgGucs};

use fixtures::*;

use futures::executor::block_on;
use lockfree_object_pool::MutexObjectPool;
use proptest::prelude::*;
use rstest::*;
use sqlx::{PgConnection, Row};

#[derive(Debug, Clone)]
enum ColumnDef {
    Id(i64),
    Uuid(&'static str),
    Name(&'static str),
    Color(&'static str),
    Age(&'static str),
    Balance(f64),
    Subscribed(bool),
}

impl ColumnDef {
    fn column_name(&self) -> &'static str {
        match self {
            ColumnDef::Id(_) => "id",
            ColumnDef::Uuid(_) => "uuid",
            ColumnDef::Name(_) => "name",
            ColumnDef::Color(_) => "color",
            ColumnDef::Age(_) => "age",
            ColumnDef::Balance(_) => "balance",
            ColumnDef::Subscribed(_) => "subscribed",
        }
    }

    fn sql_type(&self) -> &'static str {
        match self {
            ColumnDef::Id(_) => "SERIAL8",
            ColumnDef::Uuid(_) => "UUID",
            ColumnDef::Name(_) => "TEXT",
            ColumnDef::Color(_) => "VARCHAR",
            ColumnDef::Age(_) => "VARCHAR",
            ColumnDef::Balance(_) => "FLOAT8",
            ColumnDef::Subscribed(_) => "BOOLEAN",
        }
    }

    fn is_groupable(&self) -> bool {
        match self {
            ColumnDef::Id(_) => false,
            ColumnDef::Uuid(_) => false,
            ColumnDef::Name(_) => true,
            ColumnDef::Color(_) => true,
            ColumnDef::Age(_) => true,
            ColumnDef::Balance(_) => true,
            ColumnDef::Subscribed(_) => true,
        }
    }

    fn value_as_string(&self) -> String {
        match self {
            ColumnDef::Id(val) => val.to_string(),
            ColumnDef::Uuid(val) => val.to_string(),
            ColumnDef::Name(val) => val.to_string(),
            ColumnDef::Color(val) => val.to_string(),
            ColumnDef::Age(val) => val.to_string(),
            ColumnDef::Balance(val) => val.to_string(),
            ColumnDef::Subscribed(val) => val.to_string(),
        }
    }
}

// Usage:
const COLUMNS: &[ColumnDef] = &[
    ColumnDef::Id(3),
    ColumnDef::Uuid("550e8400-e29b-41d4-a716-446655440000"),
    ColumnDef::Name("bob"),
    ColumnDef::Color("blue"),
    ColumnDef::Age("20"),
    ColumnDef::Balance(456.78),
    ColumnDef::Subscribed(true),
];

fn generated_queries_setup(
    conn: &mut PgConnection,
    tables: &[(&str, usize)],
    columns_def: &[ColumnDef],
) -> String {
    "CREATE EXTENSION pg_search;".execute(conn);
    "SET log_error_verbosity TO VERBOSE;".execute(conn);
    "SET log_min_duration_statement TO 1000;".execute(conn);

    let mut setup_sql = String::new();
    let column_definitions = columns_def
        .iter()
        .map(|col| {
            if col.column_name() == "id" {
                return format!(
                    "{} {} NOT NULL PRIMARY KEY",
                    col.column_name(),
                    col.sql_type()
                );
            }
            format!("{} {}", col.column_name(), col.sql_type())
        })
        .collect::<Vec<_>>()
        .join(", \n");

    for (tname, row_count) in tables {
        let sql = format!(
            r#"
CREATE TABLE {tname}
(
        {column_definitions}
);

-- Note: Create the index before inserting rows to encourage multiple segments being created.
CREATE INDEX idx{tname} ON {tname} USING bm25 (id, uuid, name, color, age, balance, subscribed)
WITH (
key_field = 'id',
text_fields = '
            {{
                "uuid": {{ "tokenizer": {{ "type": "keyword" }}, "fast": true }},
                "name": {{ "tokenizer": {{ "type": "keyword" }}, "fast": true }},
                "color": {{ "tokenizer": {{ "type": "keyword" }}, "fast": true }},
                "age": {{ "tokenizer": {{ "type": "keyword" }}, "fast": true }},
                "balance": {{ "tokenizer": {{ "type": "keyword" }}, "fast": true }},
                "subscribed": {{ "tokenizer": {{ "type": "keyword" }}, "fast": true }}
            }}'
);

INSERT into {tname} (uuid, name, color, age, balance, subscribed)
VALUES (gen_random_uuid(), 'bob', 'blue', 20, 40.56, 'true');

INSERT into {tname} (uuid, name, color, age)
SELECT
      gen_random_uuid(),
      (ARRAY ['alice','bob','cloe', 'sally','brandy','brisket','anchovy']::text[])[(floor(random() * 7) + 1)::int],
      (ARRAY ['red','green','blue', 'orange','purple','pink','yellow']::text[])[(floor(random() * 7) + 1)::int],
      (floor(random() * 100) + 1)::int::text
FROM generate_series(1, {row_count});

CREATE INDEX idx{tname}_uuid ON {tname} (uuid);
CREATE INDEX idx{tname}_name ON {tname} (name);
CREATE INDEX idx{tname}_color ON {tname} (color);
CREATE INDEX idx{tname}_age ON {tname} (age);
CREATE INDEX idx{tname}_balance ON {tname} (balance);
CREATE INDEX idx{tname}_subscribed ON {tname} (subscribed);
ANALYZE;
"#
        );

        (&sql).execute(conn);
        setup_sql.push_str(&sql);
    }

    setup_sql
}

///
/// Tests all JoinTypes against small tables (which are particularly important for joins which
/// result in e.g. the cartesian product).
///
#[rstest]
#[tokio::test]
async fn generated_joins_small(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );

    let tables_and_sizes = [("users", 10), ("products", 10), ("orders", 10)];
    let tables = tables_and_sizes
        .iter()
        .map(|(table, _)| table)
        .collect::<Vec<_>>();
    let setup_sql = generated_queries_setup(&mut pool.pull(), &tables_and_sizes, COLUMNS);
    eprintln!("{setup_sql}");

    proptest!(|(
        (join, where_expr) in arb_joins_and_wheres(
            any::<JoinType>(),
            tables,
            vec![("id", "3"), ("name", "bob"), ("color", "blue"), ("age", "20")]
        ),
        gucs in any::<PgGucs>(),
    )| {
        let join_clause = join.to_sql();

        let from = format!("SELECT COUNT(*) {join_clause} ");

        compare(
            format!("{from} WHERE {}", where_expr.to_sql(" = ")),
            format!("{from} WHERE {}", where_expr.to_sql("@@@")),
            gucs,
            &mut pool.pull(),
            |query, conn| query.fetch_one::<(i64,)>(conn).0,
        )?;
    });
}

///
/// Tests only the smallest JoinType against larger tables, with a target list, and a limit.
///
/// TODO: This test is currently ignored because it occasionally generates nested loop joins which
/// run in exponential time: https://github.com/paradedb/paradedb/issues/2733
///
#[ignore]
#[rstest]
#[tokio::test]
async fn generated_joins_large_limit(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );

    let tables_and_sizes = [("users", 10000), ("products", 10000), ("orders", 10000)];
    let tables = tables_and_sizes
        .iter()
        .map(|(table, _)| table)
        .collect::<Vec<_>>();
    let setup_sql = generated_queries_setup(&mut pool.pull(), &tables_and_sizes, COLUMNS);
    eprintln!("{setup_sql}");

    proptest!(|(
        (join, where_expr) in arb_joins_and_wheres(
            Just(JoinType::Inner),
            tables,
            vec![("id", "3"), ("name", "bob"), ("color", "blue"), ("age", "20")]
        ),
        target_list in proptest::sample::subsequence(vec!["id", "name", "color", "age"], 1..=4),
        gucs in any::<PgGucs>(),
    )| {
        let join_clause = join.to_sql();
        let used_tables = join.used_tables();

        let target_list =
            target_list
                .into_iter()
                .map(|column| format!("{}.{column}", used_tables[0]))
                .collect::<Vec<_>>()
                .join(", ");

        let from = format!("SELECT {target_list} {join_clause} ");

        compare(
            format!("{from} WHERE {} LIMIT 10;", where_expr.to_sql(" = ")),
            format!("{from} WHERE {} LIMIT 10;", where_expr.to_sql("@@@")),
            gucs,
            &mut pool.pull(),
            |query, conn| query.fetch_dynamic(conn).len(),
        )?;
    });
}

#[rstest]
#[tokio::test]
async fn generated_single_relation(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );

    let table_name = "users";
    let setup_sql = generated_queries_setup(&mut pool.pull(), &[(table_name, 10)], COLUMNS);
    eprintln!("{setup_sql}");

    proptest!(|(
        where_expr in arb_wheres(
            vec![table_name],
            vec![("name", "bob"), ("color", "blue"), ("age", "20")]
        ),
        gucs in any::<PgGucs>(),
        target in prop_oneof![Just("COUNT(*)"), Just("id")],
    )| {
        compare(
            format!("SELECT {target} FROM {table_name} WHERE {}", where_expr.to_sql(" = ")),
            format!("SELECT {target} FROM {table_name} WHERE {}", where_expr.to_sql("@@@")),
            gucs,
            &mut pool.pull(),
            |query, conn| {
                let mut rows = query.fetch::<(i64,)>(conn);
                rows.sort();
                rows
            }
        )?;
    });
}

///
/// Property test for GROUP BY aggregates - ensures equivalence between PostgreSQL and bm25 behavior
///
#[rstest]
#[tokio::test]
async fn generated_group_by_aggregates(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );

    let table_name = "users";
    let setup_sql = generated_queries_setup(&mut pool.pull(), &[(table_name, 50)], COLUMNS);
    eprintln!("{setup_sql}");

    // Columns that can be used for grouping (must have fast: true in index)
    // TODO(#2903): Add support for more data types (other than text)
    let grouping_columns: Vec<_> = COLUMNS
        .iter()
        .filter(|col| col.is_groupable())
        .map(|col| col.column_name())
        .collect();

    let column_data: Vec<(&str, String)> = COLUMNS
        .iter()
        .filter(|col| col.is_groupable())
        .map(|col| (col.column_name(), col.value_as_string()))
        .collect();

    proptest!(|(
        where_expr in arb_wheres(
            vec![table_name],
            column_data
        ),
        group_by_expr in arb_group_by(grouping_columns.to_vec(), vec!["COUNT(*)"]),
        gucs in any::<PgGucs>(),
    )| {
        let select_list = group_by_expr.to_select_list();
        let group_by_clause = group_by_expr.to_sql();

        let pg_query = format!(
            "SELECT {} FROM {} WHERE {} {}",
            select_list,
            table_name,
            where_expr.to_sql(" = "),
            group_by_clause
        );

        let bm25_query = format!(
            "SELECT {} FROM {} WHERE {} {}",
            select_list,
            table_name,
            where_expr.to_sql("@@@"),
            group_by_clause
        );

        // Custom result comparator for GROUP BY results
        let compare_results = |query: &str, conn: &mut PgConnection| -> Vec<String> {
            // Fetch all rows as dynamic results and convert to string representation
            let rows = query.fetch_dynamic(conn);
            let mut string_rows: Vec<String> = rows
                .into_iter()
                .map(|row| {
                    // Convert entire row to a string representation for comparison
                    let mut row_string = String::new();
                    for i in 0..row.len() {
                        if i > 0 {
                            row_string.push('|');
                        }

                        // Try to get value as different types, converting to string
                        let value_str = if let Ok(val) = row.try_get::<i64, _>(i) {
                            val.to_string()
                        } else if let Ok(val) = row.try_get::<i32, _>(i) {
                            val.to_string()
                        } else if let Ok(val) = row.try_get::<String, _>(i) {
                            val
                        } else {
                            "NULL".to_string()
                        };

                        row_string.push_str(&value_str);
                    }
                    row_string
                })
                .collect();

            // Sort for consistent comparison
            string_rows.sort();
            string_rows
        };

        compare(pg_query, bm25_query, gucs, &mut pool.pull(), compare_results)?;
    });
}

#[rstest]
#[tokio::test]
async fn generated_paging_small(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );

    let table_name = "users";
    let setup_sql = generated_queries_setup(&mut pool.pull(), &[(table_name, 1000)], COLUMNS);
    eprintln!("{setup_sql}");

    proptest!(|(
        where_expr in arb_wheres(vec![table_name], vec![("name", "bob")]),
        // TODO: Until https://github.com/paradedb/paradedb/issues/2642 is resolved, we do not
        // tiebreak appropriately for compound columns, and so we do not pass any non-tiebreak
        // columns here.
        paging_exprs in arb_paging_exprs(table_name, vec![], vec!["id", "uuid"]),
        gucs in any::<PgGucs>(),
    )| {
        compare(
            format!("SELECT id FROM {table_name} WHERE {} {paging_exprs}", where_expr.to_sql(" = ")),
            format!("SELECT id FROM {table_name} WHERE {} {paging_exprs}", where_expr.to_sql("@@@")),
            gucs,
            &mut pool.pull(),
            |query, conn| query.fetch::<(i64,)>(conn),
        )?;
    });
}

/// Generates paging expressions on a large table, which was necessary to reproduce
/// https://github.com/paradedb/tantivy/pull/51
///
/// TODO: Explore whether this could use https://github.com/paradedb/paradedb/pull/2681
/// to use a large segment count rather than a large table size.
#[rstest]
#[tokio::test]
async fn generated_paging_large(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );

    let table_name = "users";
    let setup_sql = generated_queries_setup(&mut pool.pull(), &[(table_name, 100000)], COLUMNS);
    eprintln!("{setup_sql}");

    proptest!(|(
        paging_exprs in arb_paging_exprs(table_name, vec![], vec!["uuid"]),
        gucs in any::<PgGucs>(),
    )| {
        compare(
            format!("SELECT uuid::text FROM {table_name} WHERE name  =  'bob' {paging_exprs}"),
            format!("SELECT uuid::text FROM {table_name} WHERE name @@@ 'bob' {paging_exprs}"),
            gucs,
            &mut pool.pull(),
            |query, conn| query.fetch::<(String,)>(conn),
        )?;
    });
}

#[rstest]
#[tokio::test]
async fn generated_subquery(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );

    let outer_table_name = "products";
    let inner_table_name = "orders";
    let setup_sql = generated_queries_setup(
        &mut pool.pull(),
        &[(outer_table_name, 10), (inner_table_name, 10)],
        COLUMNS,
    );
    eprintln!("{setup_sql}");

    proptest!(|(
        outer_where_expr in arb_wheres(
            vec![outer_table_name],
            vec![("name", "bob"), ("color", "blue"), ("age", "20")]
        ),
        inner_where_expr in arb_wheres(
            vec![inner_table_name],
            vec![("name", "bob"), ("color", "blue"), ("age", "20")]
        ),
        subquery_column in proptest::sample::select(&["name", "color", "age"]),
        // TODO: Until https://github.com/paradedb/paradedb/issues/2642 is resolved, we do not
        // tiebreak appropriately for compound columns, and so we do not pass any non-tiebreak
        // columns here.
        paging_exprs in arb_paging_exprs(inner_table_name, vec![], vec!["id", "uuid"]),
        gucs in any::<PgGucs>(),
    )| {
        let pg = format!(
            "SELECT COUNT(*) FROM {outer_table_name} \
            WHERE {outer_table_name}.{subquery_column} IN (\
                SELECT {subquery_column} FROM {inner_table_name} WHERE {} {paging_exprs}\
            ) AND {}",
            inner_where_expr.to_sql(" = "),
            outer_where_expr.to_sql(" = "),
        );
        let bm25 = format!(
            "SELECT COUNT(*) FROM {outer_table_name} \
            WHERE {outer_table_name}.{subquery_column} IN (\
                SELECT {subquery_column} FROM {inner_table_name} WHERE {} {paging_exprs}\
            ) AND {}",
            inner_where_expr.to_sql("@@@"),
            outer_where_expr.to_sql("@@@"),
        );

        compare(
            pg,
            bm25,
            gucs,
            &mut pool.pull(),
            |query, conn| query.fetch_one::<(i64,)>(conn),
        )?;
    });
}
