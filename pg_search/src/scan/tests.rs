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

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use crate::index::fast_fields_helper::{FFHelper, WhichFastField};
    use crate::index::mvcc::MvccSatisfies;
    use crate::index::reader::index::SearchIndexReader;
    use crate::postgres::heap::VisibilityChecker as HeapVisibilityChecker;
    use crate::postgres::rel::PgSearchRelation;
    use crate::query::SearchQueryInput;
    use crate::scan::execution_plan::SegmentPlan;
    use crate::scan::Scanner;
    use crate::schema::SearchFieldType;
    use datafusion::execution::TaskContext;
    use datafusion::physical_plan::ExecutionPlan;
    use futures::StreamExt;
    use pgrx::prelude::*;
    use std::sync::Arc;

    fn get_relation_oids() -> (pg_sys::Oid, pg_sys::Oid) {
        Spi::run("SET client_min_messages = 'debug1';").unwrap();
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run("INSERT INTO t (data) SELECT 'test ' || i FROM generate_series(1, 100) i;")
            .unwrap();
        Spi::run(
            "CREATE INDEX t_idx ON t USING bm25(id, (data::pdb.simple)) WITH (key_field = 'id')",
        )
        .unwrap();

        let heap_oid = Spi::get_one::<pg_sys::Oid>(
            "SELECT oid FROM pg_class WHERE relname = 't' AND relkind = 'r';",
        )
        .expect("spi")
        .unwrap();

        let index_oid = Spi::get_one::<pg_sys::Oid>(
            "SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';",
        )
        .expect("spi")
        .unwrap();

        (heap_oid, index_oid)
    }

    #[pg_test]
    fn test_datafusion_scan() {
        let (heap_oid, index_oid) = get_relation_oids();
        let heap_rel = PgSearchRelation::open(heap_oid);
        let index_rel = PgSearchRelation::open(index_oid);

        // Open search reader
        let reader = SearchIndexReader::open(
            &index_rel,
            SearchQueryInput::All, // Scan all docs
            false,                 // need_scores
            MvccSatisfies::Snapshot,
        )
        .unwrap();

        let search_results = reader.search();

        // Define fields to scan
        let fields = vec![
            WhichFastField::Ctid,
            WhichFastField::Named("id".to_string(), SearchFieldType::I64(pg_sys::INT4OID)),
        ];

        let ffhelper = FFHelper::with_fields(&reader, &fields);

        // Ensure current transaction changes are visible
        unsafe {
            pg_sys::CommandCounterIncrement();
            let snap = pg_sys::GetTransactionSnapshot();
            pg_sys::PushActiveSnapshot(snap);
        }
        let snapshot = unsafe { pg_sys::GetActiveSnapshot() };
        let visibility = HeapVisibilityChecker::with_rel_and_snap(&heap_rel, snapshot);

        let scanner = Scanner::new(search_results, None, fields, heap_oid.into());

        let plan = SegmentPlan::new(
            scanner,
            ffhelper.into(),
            Box::new(visibility),
            SearchQueryInput::All,
        );

        let task_ctx = Arc::new(TaskContext::default());
        let mut stream = plan.execute(0, task_ctx).unwrap();

        let mut row_count = 0;

        // Use a runtime to block on the stream
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        runtime.block_on(async {
            while let Some(batch) = stream.next().await {
                let batch = batch.unwrap();
                row_count += batch.num_rows();
                // Basic verification
                assert_eq!(batch.num_columns(), 2); // ctid and id
            }
        });

        assert_eq!(row_count, 100);
    }

    // ==================== Filter Pushdown Test Helpers ====================

    /// Standard test fields for filter pushdown tests: ctid, id (i64), price (f64), quantity (i64)
    fn test_fields() -> Vec<WhichFastField> {
        vec![
            WhichFastField::Ctid,
            WhichFastField::Named("id".to_string(), SearchFieldType::I64(pg_sys::InvalidOid)),
            WhichFastField::Named(
                "price".to_string(),
                SearchFieldType::F64(pg_sys::InvalidOid),
            ),
            WhichFastField::Named(
                "quantity".to_string(),
                SearchFieldType::I64(pg_sys::InvalidOid),
            ),
        ]
    }

    /// Push an active snapshot so transaction changes are visible
    fn push_active_snapshot() {
        unsafe {
            pg_sys::CommandCounterIncrement();
            let snap = pg_sys::GetTransactionSnapshot();
            pg_sys::PushActiveSnapshot(snap);
        }
    }

    /// Create a test table with 100 rows for filter pushdown tests.
    /// Returns (heap_oid, index_oid).
    fn create_filter_pushdown_test_table() -> (pg_sys::Oid, pg_sys::Oid) {
        Spi::run("SET client_min_messages = 'debug1';").unwrap();
        Spi::run(
            "CREATE TABLE filter_test (
                id SERIAL PRIMARY KEY,
                price DOUBLE PRECISION,
                quantity INTEGER
            );",
        )
        .unwrap();

        // 100 rows: price = 10.0, 20.0, ..., 1000.0; quantity = 1, 2, ..., 100
        Spi::run(
            "INSERT INTO filter_test (price, quantity)
             SELECT (i * 10)::double precision, i
             FROM generate_series(1, 100) i;",
        )
        .unwrap();

        Spi::run(
            "CREATE INDEX filter_test_idx ON filter_test
             USING bm25(id, price, quantity)
             WITH (
                 key_field = 'id',
                 numeric_fields = '{\"price\": {\"fast\": true}, \"quantity\": {\"fast\": true}}'
             );",
        )
        .unwrap();

        let heap_oid = Spi::get_one::<pg_sys::Oid>(
            "SELECT oid FROM pg_class WHERE relname = 'filter_test' AND relkind = 'r';",
        )
        .expect("spi")
        .unwrap();

        let index_oid = Spi::get_one::<pg_sys::Oid>(
            "SELECT oid FROM pg_class WHERE relname = 'filter_test_idx' AND relkind = 'i';",
        )
        .expect("spi")
        .unwrap();

        (heap_oid, index_oid)
    }

    // ==================== FilterAnalyzer Test Helpers ====================

    mod filter_analyzer_helpers {
        use crate::scan::filter_pushdown::FilterAnalyzer;
        use datafusion::logical_expr::Expr;

        /// Assert that the filter is supported
        pub fn assert_exact(analyzer: &FilterAnalyzer, filter: &Expr, desc: &str) {
            assert!(analyzer.supports(filter), "{}: expected supported", desc);
        }

        /// Assert that the filter is not supported
        pub fn assert_unsupported(analyzer: &FilterAnalyzer, filter: &Expr, desc: &str) {
            assert!(!analyzer.supports(filter), "{}: expected unsupported", desc);
        }
    }

    #[pg_test]
    fn test_filter_pushdown_analysis() {
        use crate::scan::filter_pushdown::FilterAnalyzer;
        use datafusion::logical_expr::{col, lit, Expr};
        use filter_analyzer_helpers::{assert_exact, assert_unsupported};

        let fields = test_fields();
        let analyzer = FilterAnalyzer::new(&fields, pgrx::pg_sys::InvalidOid);

        // Equality
        assert_exact(&analyzer, &col("id").eq(lit(1i64)), "id = 1");

        // Range filters
        assert_exact(&analyzer, &col("price").gt(lit(100.0f64)), "price > 100.0");
        assert_exact(
            &analyzer,
            &col("quantity").lt_eq(lit(50i64)),
            "quantity <= 50",
        );

        // Boolean combinations
        assert_exact(
            &analyzer,
            &col("price")
                .gt(lit(100.0f64))
                .and(col("quantity").lt(lit(50i64))),
            "price > 100 AND quantity < 50",
        );
        assert_exact(
            &analyzer,
            &col("id").eq(lit(1i64)).or(col("id").eq(lit(2i64))),
            "id = 1 OR id = 2",
        );

        // NULL checks
        assert_exact(
            &analyzer,
            &Expr::IsNull(Box::new(col("price"))),
            "price IS NULL",
        );
        assert_exact(
            &analyzer,
            &Expr::IsNotNull(Box::new(col("price"))),
            "price IS NOT NULL",
        );

        // IN list
        assert_exact(
            &analyzer,
            &col("id").in_list(vec![lit(1i64), lit(2i64), lit(3i64)], false),
            "id IN (1, 2, 3)",
        );

        // NOT
        assert_exact(
            &analyzer,
            &Expr::Not(Box::new(col("id").eq(lit(1i64)))),
            "NOT id = 1",
        );

        // Unknown column -> Unsupported
        assert_unsupported(
            &analyzer,
            &col("unknown_column").eq(lit(1i64)),
            "unknown_column = 1",
        );

        pgrx::warning!("All filter pushdown analysis tests passed!");
    }

    // ==================== TableProvider Pushdown Test Helpers ====================

    mod table_provider_helpers {
        use super::*;
        use crate::scan::info::ScanInfo;
        use crate::scan::table_provider::PgSearchTableProvider;
        use datafusion::catalog::TableProvider;
        use datafusion::logical_expr::{Expr, TableProviderFilterPushDown};

        /// Create a PgSearchTableProvider for testing
        pub fn create_provider(
            heap_oid: pg_sys::Oid,
            index_oid: pg_sys::Oid,
            fields: Vec<WhichFastField>,
        ) -> Arc<PgSearchTableProvider> {
            let mut scan_info = ScanInfo::new()
                .with_heaprelid(heap_oid)
                .with_indexrelid(index_oid)
                .with_heap_rti(1);

            for (i, field) in fields.iter().enumerate() {
                scan_info.add_field(i as pg_sys::AttrNumber, field.clone());
            }

            Arc::new(PgSearchTableProvider::new(scan_info, fields, None, false))
        }

        /// Assert all filters get Exact pushdown
        pub fn assert_all_exact(provider: &PgSearchTableProvider, filters: &[&Expr], desc: &str) {
            let results = provider.supports_filters_pushdown(filters).unwrap();
            assert_eq!(results.len(), filters.len(), "{}: length mismatch", desc);
            for (i, result) in results.iter().enumerate() {
                assert!(
                    matches!(result, TableProviderFilterPushDown::Exact),
                    "{}: filter {} expected Exact, got {:?}",
                    desc,
                    i,
                    result
                );
            }
            pgrx::warning!("{} -> all Exact", desc);
        }

        /// Assert filter gets Unsupported
        pub fn assert_unsupported(provider: &PgSearchTableProvider, filter: &Expr, desc: &str) {
            let results = provider.supports_filters_pushdown(&[filter]).unwrap();
            assert_eq!(results.len(), 1);
            assert!(
                matches!(results[0], TableProviderFilterPushDown::Unsupported),
                "{}: expected Unsupported, got {:?}",
                desc,
                results[0]
            );
            pgrx::warning!("{} -> Unsupported", desc);
        }
    }

    // ==================== DataFusion Query Test Helpers ====================

    mod datafusion_query_helpers {
        use datafusion::dataframe::DataFrame;
        use datafusion::logical_expr::Expr;
        use datafusion::prelude::SessionContext;

        /// Count rows from a DataFrame
        pub async fn count_rows(df: DataFrame) -> usize {
            df.collect()
                .await
                .unwrap()
                .iter()
                .map(|b| b.num_rows())
                .sum()
        }

        /// Execute query with optional filter and assert row count
        pub async fn assert_query_count(
            ctx: &SessionContext,
            table: &str,
            filter: Option<Expr>,
            expected: usize,
            desc: &str,
        ) {
            let df = ctx.table(table).await.unwrap();
            let df = match filter {
                Some(f) => df.filter(f).unwrap(),
                None => df,
            };
            let count = count_rows(df).await;
            assert_eq!(
                count, expected,
                "{}: expected {} rows, got {}",
                desc, expected, count
            );
            pgrx::warning!("{}: {} rows", desc, count);
        }
    }

    #[pg_test]
    fn test_datafusion_filter_pushdown_end_to_end() {
        use datafusion::logical_expr::{col, lit};
        use datafusion::prelude::SessionContext;
        use datafusion_query_helpers::assert_query_count;
        use table_provider_helpers::{assert_all_exact, assert_unsupported, create_provider};

        let (heap_oid, index_oid) = create_filter_pushdown_test_table();
        push_active_snapshot();

        let fields = test_fields();
        let provider = create_provider(heap_oid, index_oid, fields);

        // Test supports_filters_pushdown API
        let quantity_gt_50 = col("quantity").gt(lit(50i64));
        let price_lt_500 = col("price").lt(lit(500.0f64));

        assert_all_exact(&provider, &[&quantity_gt_50], "quantity > 50");
        assert_all_exact(
            &provider,
            &[&quantity_gt_50, &price_lt_500],
            "quantity > 50, price < 500",
        );
        assert_unsupported(
            &provider,
            &col("unknown_col").eq(lit(1i64)),
            "unknown column",
        );

        // Test full DataFusion flow
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();

        runtime.block_on(async {
            let ctx = SessionContext::new();
            ctx.register_table("filter_test", provider.clone()).unwrap();

            // No filter: all 100 rows
            assert_query_count(&ctx, "filter_test", None, 100, "no filter").await;

            // quantity > 50: rows 51-100 = 50 rows
            assert_query_count(
                &ctx,
                "filter_test",
                Some(col("quantity").gt(lit(50i64))),
                50,
                "quantity > 50",
            )
            .await;

            // quantity > 50 AND price < 800: rows 51-79 = 29 rows
            assert_query_count(
                &ctx,
                "filter_test",
                Some(
                    col("quantity")
                        .gt(lit(50i64))
                        .and(col("price").lt(lit(800.0f64))),
                ),
                29,
                "quantity > 50 AND price < 800",
            )
            .await;

            // quantity = 25: 1 row
            assert_query_count(
                &ctx,
                "filter_test",
                Some(col("quantity").eq(lit(25i64))),
                1,
                "quantity = 25",
            )
            .await;

            // quantity IN (10, 20, 30): 3 rows
            assert_query_count(
                &ctx,
                "filter_test",
                Some(col("quantity").in_list(vec![lit(10i64), lit(20i64), lit(30i64)], false)),
                3,
                "quantity IN (10, 20, 30)",
            )
            .await;
        });

        pgrx::warning!("All DataFusion filter pushdown end-to-end tests passed!");
    }
}
