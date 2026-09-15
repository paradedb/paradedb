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

// Tokenizer expression fields cannot yet be read back by pdb.agg() over joins.
use super::*;

/// Property test for `pdb.agg()` over joins: the buckets and metrics the DataFusion backend
/// assembles must equal the rows of the equivalent SQL `GROUP BY` run by PostgreSQL, which is
/// the oracle since `pdb.agg()` itself has no native fallback. Covers nested `terms`, a
/// `size` cut under the default count order, NULL buckets, NUMERIC metrics, `cardinality`,
/// a SQL `GROUP BY` beside the call, and MPP when the parallel GUCs are on.
/// TODO: Consider merging this property test with other "aggregate over join" tests
/// (such as `generated_aggregate_join` and `generated_join_aggregates`) in the future.
#[rstest]
#[tokio::test]
async fn generated_pdb_agg_join(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );

    let tables_and_sizes = [("users", 50), ("products", 50), ("orders", 50)];
    let all_tables: Vec<String> = tables_and_sizes
        .iter()
        .map(|(table, _)| table.to_string())
        .collect();
    let mut setup_sql = generated_queries_setup(&pool, &tables_and_sizes, COLUMNS);
    let mut conn = pool.pull();
    for (table, _) in tables_and_sizes {
        let start = setup_sql
            .sql
            .find(&format!("CREATE INDEX idx{table} "))
            .unwrap();
        let end = start + setup_sql.sql[start..].find(';').unwrap() + 1;
        let mut ddl = setup_sql.sql[start..end].to_string();
        for column in ["name", "color", "tags"] {
            ddl = ddl.replace(&format!("({column}::pdb.literal)"), column);
        }
        ddl = ddl.replace(
            "WITH (",
            r#"WITH (text_fields = '{
            "name": {"tokenizer": {"type": "keyword"}, "fast": true},
            "color": {"tokenizer": {"type": "keyword"}, "fast": true},
            "tags": {"tokenizer": {"type": "keyword"}, "fast": true}
        }',"#,
        );
        format!("DROP INDEX idx{table}; {ddl}").execute(&mut conn);
        setup_sql.sql.replace_range(start..end, &ddl);
    }
    drop(conn);

    let where_columns = columns_named(vec!["name", "color"]);
    let join_key_columns = columns_named(vec!["id", "age"]);

    proptest!(qgen_proptest_config(), |(
        (join_expr, agg, wheres) in arb_pdb_agg_join(all_tables.clone(), &join_key_columns, &where_columns),
        mut gucs in any::<PgGucs>(),
    )| {
        let join_clause = join_expr.to_sql();
        let pg_query = agg.pg_query(&join_clause, &wheres.pg_where());
        let bm25_query = agg.pdb_query(&join_clause, &wheres.bm25_where());

        // `pdb.agg()` over a join only runs on the DataFusion backend.
        gucs.aggregate_custom_scan = true;
        gucs.join_custom_scan = true;
        gucs.custom_scan = true;

        if !agg.outer_aggs.is_empty() {
            let pg_outer_query = agg.pg_outer_query(&join_clause, &wheres.pg_where());
            qgen_oracle!("qgen: generated_pdb_agg_join - outer aggregates match PostgreSQL", compare_outcome_retrying(
                &pg_outer_query,
                &bm25_query,
                &gucs,
                &pool,
                &setup_sql,
                |query, side, conn| {
                    "SET work_mem TO '64MB';".execute_result(conn)?;
                    let rows = query.fetch_dynamic_result(conn)?;
                    let mut rows = agg.outer_rows(rows, side.is_candidate())?;
                    rows.sort();
                    Ok(rows)
                },
            ))?;
        }

        qgen_oracle!("qgen: generated_pdb_agg_join - pdb.agg() buckets match PostgreSQL GROUP BY", compare_outcome_retrying(
            &pg_query,
            &bm25_query,
            &gucs,
            &pool,
            &setup_sql,
            |query, _, conn| {
                // A keyless join under three bucket keys makes tens of thousands of
                // buckets, and the DataFusion aggregate cannot spill past `work_mem`.
                "SET work_mem TO '64MB';".execute_result(conn)?;
                let rows = query.fetch_dynamic_result(conn)?;
                let mut rows = agg.rows(rows)?;
                rows.sort();
                Ok(rows)
            },
        ))?;
    });
}
