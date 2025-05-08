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

// Tests for ParadeDB's Custom Scan implementation
mod fixtures;

use fixtures::*;
use futures::executor::block_on;
use lockfree_object_pool::MutexObjectPool;
use parking_lot::Mutex;
use rayon::prelude::*;
use rstest::*;
use sqlgen::joingen::JoinGenerator;
use sqlgen::sqlgen::QueryGenerator;
use sqlx::PgConnection;
use std::collections::HashMap;

#[rstest]
#[tokio::test]
async fn generated_queries(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );

    "CREATE EXTENSION pg_search;".execute(&mut pool.pull());

    let mut setup_sql = String::new();

    for tname in ["users", "products", "orders"] {
        let sql = format!(
            r#"
CREATE TABLE {tname}
(
    id    serial8 not null primary key,
    name  text,
    color varchar,
    age   varchar
);

INSERT into {tname} (name, color, age)
VALUES ('bob', 'blue', 20);

INSERT into {tname} (name, color, age)
SELECT(ARRAY ['alice','bob','cloe', 'sally','brandy','brisket','anchovy']::text[])[(floor(random() * 7) + 1)::int],
      (ARRAY ['red','green','blue', 'orange','purple','pink','yellow']::text[])[(floor(random() * 7) + 1)::int],
      (floor(random() * 100) + 1)::int::text
FROM generate_series(1, 1000);

CREATE INDEX idx{tname} ON {tname} USING bm25 (id, name, color, age)
WITH (
key_field = 'id',
text_fields = '
            {{
                "name": {{ "tokenizer": {{ "type": "keyword" }} }},
                "color": {{ "tokenizer": {{ "type": "keyword" }} }},
                "age": {{ "tokenizer": {{ "type": "keyword" }} }}
            }}'
);
CREATE INDEX idx{tname}_name ON {tname} (name);
CREATE INDEX idx{tname}_color ON {tname} (color);
CREATE INDEX idx{tname}_age ON {tname} (age);
"#,
            tname = tname
        );

        (&sql).execute(&mut pool.pull());
        setup_sql.push_str(&sql);
    }

    "ANALYZE;".execute(&mut pool.pull());

    let want = |table_name: &str| {
        vec![
            (format!("{table_name}.name"), "bob"),
            (format!("{table_name}.color"), "blue"),
            (format!("{table_name}.age"), "20"),
        ]
    };

    let pg_generators = {
        let mut generators = HashMap::<&str, QueryGenerator<&str>>::default();
        generators.insert("users", QueryGenerator::new(" = ", want("users")));
        generators.insert("orders", QueryGenerator::new(" = ", want("orders")));
        generators.insert("products", QueryGenerator::new(" = ", want("products")));
        generators
    };
    let bm25_generators = {
        let mut generators = HashMap::<&str, QueryGenerator<&str>>::default();
        generators.insert("users", QueryGenerator::new("@@@", want("users")));
        generators.insert("orders", QueryGenerator::new("@@@", want("orders")));
        generators.insert("products", QueryGenerator::new("@@@", want("products")));
        generators
    };

    let generators = Mutex::new((pg_generators, bm25_generators));
    let errors = Mutex::new(String::new());

    for connector in ["AND", "OR", "AND NOT"] {
        println!("connector={connector}");

        JoinGenerator::new(vec![
            ("users", vec!["name", "color", "age"]),
            ("orders", vec!["name", "color", "age"]),
            ("products", vec!["name", "color", "age"]),
        ])
        .take(100)
        .enumerate()
        .par_bridge()
        .for_each(|(idx, (join_clause, used_tables))| {
            let from = format!("SELECT COUNT(*) {join_clause} ");

            let mut pg_where_clauses = Vec::with_capacity(used_tables.len() * 1);
            let mut bm25_where_clauses = Vec::with_capacity(used_tables.len() * 1);

            // populate the where clauses with what should be matching where clauses from the two different generators
            {
                let mut generators = generators.lock();

                for table_name in &used_tables {
                    pg_where_clauses
                        .extend(generators.0.get_mut(table_name.as_str()).unwrap().take(1));
                }

                for table_name in used_tables {
                    bm25_where_clauses
                        .extend(generators.1.get_mut(table_name.as_str()).unwrap().take(1));
                }
            }

            let pg = format!(
                "{from} WHERE {}",
                pg_where_clauses.join(&format!(" {connector} "))
            );
            let bm25 = format!(
                "{from} WHERE {}",
                bm25_where_clauses.join(&format!(" {connector} ")),
            );

            let (pg_count,) = (&pg).fetch_one::<(i64,)>(&mut pool.pull());
            let (bm25_count,) = (&bm25).fetch_one::<(i64,)>(&mut pool.pull());

            if pg_count != bm25_count {
                let mut errors = errors.lock();
                errors.push_str(&format!("---- connector={connector} ----\n"));
                errors.push_str(&format!("-- pg={pg_count}, bm25={bm25_count}\n"));
                errors.push_str(&format!("{pg}\n"));
                errors.push_str(&format!("{bm25}\n"));
                errors.push('\n');
            }
        });
    }

    let errors = errors.into_inner();
    if !errors.is_empty() {
        eprintln!("{setup_sql}");
        eprintln!("{errors}");
    }

    // QueryGenerator::new("=", want)
    //     .take(1000000)
    //     .enumerate()
    //     .par_bridge()
    //     .for_each(|(idx, where_clause)| {
    //         let pg = format!("SELECT count(*) FROM qgen WHERE {where_clause}");
    //         let bm25 = format!(
    //             "SELECT count(*) FROM qgen WHERE ({where_clause}) AND id @@@ paradedb.all()"
    //         ); // force a pushdown
    //
    //         let mut conn = pool.pull();
    //         let (pg_cnt,) = (&pg).fetch_one::<(i64,)>(&mut conn);
    //         let (bm25_cnt,) = (&bm25).fetch_one::<(i64,)>(&mut conn);
    //
    //         eprintln!("#{idx}:  {pg}");
    //         eprintln!("#{idx}:  {bm25}");
    //
    //         assert_eq!(pg_cnt, bm25_cnt, "{pg} / {bm25}");
    //     });
}
