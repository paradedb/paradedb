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
struct BM25Options {
    /// "text_fields" or "numeric_fields"
    field_type: &'static str,
    /// The JSON config for this field, e.g. `{ "tokenizer": { "type": "keyword" } }`
    config_json: &'static str,
}

#[derive(Debug, Clone)]
struct Column {
    name: &'static str,
    sql_type: &'static str,
    sample_value: &'static str,
    is_primary_key: bool,
    is_groupable: bool,
    is_indexed: bool,
    bm25_options: Option<BM25Options>,
    random_generator_sql: &'static str,
}

impl Column {
    const fn new(name: &'static str, sql_type: &'static str, sample_value: &'static str) -> Self {
        Self {
            name,
            sql_type,
            sample_value,
            is_primary_key: false,
            is_groupable: true,
            is_indexed: true,
            bm25_options: None,
            random_generator_sql: "NULL",
        }
    }

    const fn primary_key(mut self) -> Self {
        self.is_primary_key = true;
        self
    }

    const fn groupable(mut self, is_groupable: bool) -> Self {
        self.is_groupable = is_groupable;
        self
    }

    #[allow(dead_code)]
    const fn indexed(mut self, is_indexed: bool) -> Self {
        self.is_indexed = is_indexed;
        self
    }

    const fn bm25_text_field(mut self, config_json: &'static str) -> Self {
        self.bm25_options = Some(BM25Options {
            field_type: "text_fields",
            config_json,
        });
        self
    }

    const fn bm25_numeric_field(mut self, config_json: &'static str) -> Self {
        self.bm25_options = Some(BM25Options {
            field_type: "numeric_fields",
            config_json,
        });
        self
    }

    const fn random_generator_sql(mut self, random_generator_sql: &'static str) -> Self {
        self.random_generator_sql = random_generator_sql;
        self
    }

    fn raw_sample_value(&self) -> &str {
        self.sample_value.trim_matches('\'')
    }
}

const COLUMNS: &[Column] = &[
    Column::new("id", "SERIAL8", "4")
        .primary_key()
        .groupable({
            // TODO: Grouping on id/uuid causes:
            // ```
            // Var in target list not found in grouping columns
            // ```
            false
        }),
    Column::new("uuid", "UUID", "'550e8400-e29b-41d4-a716-446655440000'")
        .groupable({
            // TODO: Grouping on id/uuid causes:
            // ```
            // Var in target list not found in grouping columns
            // ```
            false
        })
        .bm25_text_field(r#""uuid": { "tokenizer": { "type": "keyword" } , "fast": true }"#)
        .random_generator_sql("gen_random_uuid()"),
    Column::new("name", "TEXT", "'bob'")
        .bm25_text_field(r#""name": { "tokenizer": { "type": "keyword" }, "fast": true }"#)
        .random_generator_sql(
            "(ARRAY ['alice','bob','cloe', 'sally','brandy','brisket','anchovy']::text[])[(floor(random() * 7) + 1)::int]"
        ),
    Column::new("color", "VARCHAR", "'blue'")
        .bm25_text_field(r#""color": { "tokenizer": { "type": "keyword" }, "fast": true }"#)
        .random_generator_sql(
            "(ARRAY ['red','green','blue', 'orange','purple','pink','yellow']::text[])[(floor(random() * 7) + 1)::int]"
        ),
    Column::new("age", "INTEGER", "20")
        .bm25_numeric_field(r#""age": { "fast": true }"#)
        .random_generator_sql("(floor(random() * 100) + 1)::int"),
    Column::new("price", "NUMERIC(10,2)", "99.99")
        .groupable({
            // TODO: Grouping on a float fails to ORDER BY (even in cases without an ORDER BY):
            // ```
            // Cannot ORDER BY OrderByInfo
            // ```
            false
        })
        .bm25_numeric_field(r#""price": { "fast": true }"#)
        .random_generator_sql("(random() * 1000 + 10)::numeric(10,2)"),
    Column::new("rating", "INTEGER", "4")
        .bm25_numeric_field(r#""rating": { "fast": true }"#)
        .random_generator_sql("(floor(random() * 5) + 1)::int"),
];

fn generated_queries_setup(
    conn: &mut PgConnection,
    tables: &[(&str, usize)],
    columns_def: &[Column],
) -> String {
    "CREATE EXTENSION pg_search;".execute(conn);
    "SET log_error_verbosity TO VERBOSE;".execute(conn);
    "SET log_min_duration_statement TO 1000;".execute(conn);

    let mut setup_sql = String::new();
    let column_definitions = columns_def
        .iter()
        .map(|col| {
            if col.is_primary_key {
                format!("{} {} NOT NULL PRIMARY KEY", col.name, col.sql_type)
            } else {
                format!("{} {}", col.name, col.sql_type)
            }
        })
        .collect::<Vec<_>>()
        .join(", \n");

    // For bm25 index
    let bm25_columns = columns_def
        .iter()
        .map(|c| c.name)
        .collect::<Vec<_>>()
        .join(", ");
    let key_field = columns_def
        .iter()
        .find(|c| c.is_primary_key)
        .map(|c| c.name)
        .expect("At least one column must be a primary key");

    let text_fields = columns_def
        .iter()
        .filter_map(|c| c.bm25_options.as_ref())
        .filter(|o| o.field_type == "text_fields")
        .map(|o| o.config_json)
        .collect::<Vec<_>>()
        .join(",\n");

    let numeric_fields = columns_def
        .iter()
        .filter_map(|c| c.bm25_options.as_ref())
        .filter(|o| o.field_type == "numeric_fields")
        .map(|o| o.config_json)
        .collect::<Vec<_>>()
        .join(",\n");

    // For INSERT statements
    let insert_columns = columns_def
        .iter()
        .filter(|c| !c.is_primary_key)
        .map(|c| c.name)
        .collect::<Vec<_>>()
        .join(", ");

    let sample_values = columns_def
        .iter()
        .filter(|c| !c.is_primary_key)
        .map(|c| c.sample_value)
        .collect::<Vec<_>>()
        .join(", ");

    let random_generators = columns_def
        .iter()
        .filter(|c| !c.is_primary_key)
        .map(|c| c.random_generator_sql)
        .collect::<Vec<_>>()
        .join(",\n      ");

    for (tname, row_count) in tables {
        let sql = format!(
            r#"
CREATE TABLE {tname} (
    {column_definitions}
);
-- Note: Create the index before inserting rows to encourage multiple segments being created.
CREATE INDEX idx{tname} ON {tname} USING bm25 ({bm25_columns}) WITH (
    key_field = '{key_field}',
    text_fields = '{{ {text_fields} }}',
    numeric_fields = '{{ {numeric_fields} }}'
);

INSERT into {tname} ({insert_columns}) VALUES ({sample_values});

INSERT into {tname} ({insert_columns}) SELECT {random_generators} FROM generate_series(1, {row_count});

{b_tree_indexes}

ANALYZE;
"#,
            b_tree_indexes = columns_def
                .iter()
                .filter(|c| c.is_indexed)
                .map(|c| format!(
                    "CREATE INDEX idx{tname}_{name} ON {tname} ({name});",
                    name = c.name
                ))
                .collect::<Vec<_>>()
                .join("\n")
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
        move || {
            block_on(async {
                {
                    database.connection().await
                }
            })
        },
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
            &format!("{from} WHERE {}", where_expr.to_sql(" = ")),
            &format!("{from} WHERE {}", where_expr.to_sql("@@@")),
            &gucs,
            &mut pool.pull(),
            &setup_sql,
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
        move || {
            block_on(async {
                {
                    database.connection().await
                }
            })
        },
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
            &format!("{from} WHERE {} LIMIT 10;", where_expr.to_sql(" = ")),
            &format!("{from} WHERE {} LIMIT 10;", where_expr.to_sql("@@@")),
            &gucs,
            &mut pool.pull(),
            &setup_sql,
            |query, conn| query.fetch_dynamic(conn).len(),
        )?;
    });
}

#[rstest]
#[tokio::test]
async fn generated_single_relation(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || {
            block_on(async {
                {
                    database.connection().await
                }
            })
        },
        |_| {},
    );

    let table_name = "users";
    let setup_sql = generated_queries_setup(&mut pool.pull(), &[(table_name, 10)], COLUMNS);
    eprintln!("{setup_sql}");

    let where_columns: Vec<_> = COLUMNS
        .iter()
        .map(|c| (c.name, c.raw_sample_value()))
        .collect();

    proptest!(|(
        where_expr in arb_wheres(
            vec![table_name],
            where_columns,
        ),
        gucs in any::<PgGucs>(),
        target in prop_oneof![Just("COUNT(*)"), Just("id")],
    )| {
        compare(
            &format!("SELECT {target} FROM {table_name} WHERE {}", where_expr.to_sql(" = ")),
            &format!("SELECT {target} FROM {table_name} WHERE {}", where_expr.to_sql("@@@")),
            &gucs,
            &mut pool.pull(),
            &setup_sql,
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
        move || {
            block_on(async {
                {
                    database.connection().await
                }
            })
        },
        |_| {},
    );

    let table_name = "users";
    let setup_sql = generated_queries_setup(&mut pool.pull(), &[(table_name, 50)], COLUMNS);
    eprintln!("{setup_sql}");

    // Columns that can be used for grouping (must have fast: true in index)
    let grouping_columns: Vec<_> = COLUMNS
        .iter()
        .filter(|col| col.is_groupable)
        .map(|col| col.name)
        .collect();

    let where_columns: Vec<_> = COLUMNS
        .iter()
        .filter(|col| col.is_groupable)
        .map(|col| (col.name, col.raw_sample_value()))
        .collect();

    proptest!(|(
        text_where_expr in arb_wheres(
            vec![table_name],
            where_columns,
        ),
        numeric_where_expr in arb_wheres(
            vec![table_name],
            vec![("age", "20"), ("price", "99.99"), ("rating", "4")]
        ),
        group_by_expr in arb_group_by(grouping_columns.to_vec(), vec!["COUNT(*)", "SUM(price)", "AVG(price)", "MIN(rating)", "MAX(rating)", "SUM(age)", "AVG(age)"]),
        gucs in any::<PgGucs>(),
    )| {
        let select_list = group_by_expr.to_select_list();
        let group_by_clause = group_by_expr.to_sql();

        // Create combined WHERE clause for PostgreSQL using = operator
        let pg_where_clause = format!(
            "({}) AND ({})",
            text_where_expr.to_sql(" = "),
            numeric_where_expr.to_sql(" < ")
        );

        // Create combined WHERE clause for BM25 using appropriate operators
        let bm25_where_clause = format!(
            "({}) AND ({})",
            text_where_expr.to_sql("@@@"),
            numeric_where_expr.to_sql(" < ")
        );

        let pg_query = format!(
            "SELECT {select_list} FROM {table_name} WHERE {pg_where_clause} {group_by_clause}",
        );

        let bm25_query = format!(
            "SELECT {select_list} FROM {table_name} WHERE {bm25_where_clause} {group_by_clause}",
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

        compare(&pg_query, &bm25_query, &gucs, &mut pool.pull(), &setup_sql, compare_results)?;
    });
}

#[rstest]
#[tokio::test]
async fn generated_paging_small(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || {
            block_on(async {
                {
                    database.connection().await
                }
            })
        },
        |_| {},
    );

    let table_name = "users";
    let setup_sql = generated_queries_setup(&mut pool.pull(), &[(table_name, 1000)], COLUMNS);
    eprintln!("{setup_sql}");

    proptest!(|(
        where_expr in arb_wheres(vec![table_name], vec![("name", "bob")]),
        paging_exprs in arb_paging_exprs(table_name, vec!["name", "color", "age"], vec!["id", "uuid"]),
        gucs in any::<PgGucs>(),
    )| {
        compare(
            &format!("SELECT id FROM {table_name} WHERE {} {paging_exprs}", where_expr.to_sql(" = ")),
            &format!("SELECT id FROM {table_name} WHERE {} {paging_exprs}", where_expr.to_sql("@@@")),
            &gucs,
            &mut pool.pull(),
            &setup_sql,
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
        move || {
            block_on(async {
                {
                    database.connection().await
                }
            })
        },
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
            &format!("SELECT uuid::text FROM {table_name} WHERE name  =  'bob' {paging_exprs}"),
            &format!("SELECT uuid::text FROM {table_name} WHERE name @@@ 'bob' {paging_exprs}"),
            &gucs,
            &mut pool.pull(),
            &setup_sql,
            |query, conn| query.fetch::<(String,)>(conn),
        )?;
    });
}

#[rstest]
#[tokio::test]
async fn generated_subquery(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || {
            block_on(async {
                {
                    database.connection().await
                }
            })
        },
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
        paging_exprs in arb_paging_exprs(inner_table_name, vec!["name", "color", "age"], vec!["id", "uuid"]),
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
            &pg,
            &bm25,
            &gucs,
            &mut pool.pull(),
            &setup_sql,
            |query, conn| query.fetch_one::<(i64,)>(conn),
        )?;
    });
}
