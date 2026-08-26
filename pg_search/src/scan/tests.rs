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
    use crate::index::fast_fields_helper::{FFHelper, WhichFastField, build_arrow_schema};
    use crate::index::mvcc::MvccSatisfies;
    use crate::index::reader::index::SearchIndexReader;
    use crate::postgres::heap::VisibilityChecker as HeapVisibilityChecker;
    use crate::postgres::rel::PgSearchRelation;
    use crate::query::SearchQueryInput;
    use crate::scan::execution_plan::PgSearchScanPlan;
    use crate::schema::SearchFieldType;
    use datafusion::common::stats::Precision;
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
            "CREATE INDEX t_idx ON t USING paradedb (id, (data::pdb.simple)) WITH (key_field = 'id')",
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

        let partition = crate::scan::execution_plan::ScanState {
            source_idx: None,
            planner_estimated_rows: 0,
            scanner_config: crate::scan::execution_plan::ScannerConfig {
                which_fast_fields: fields.clone(),
                heap_relid: heap_oid.into(),
                batch_size_hint: None,
                score_needed: false,
                scan_mode: crate::scan::ScanMode::all(),
            },
            ffhelper: ffhelper.into(),
            visibility: Box::new(visibility),
            reader: reader.clone(),
        };

        let plan = PgSearchScanPlan::new(
            Some(partition),
            build_arrow_schema(&fields),
            SearchQueryInput::All,
            None,
            Vec::new(),
            None,
            0,
            None,
            1,
            None,
            None,
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
             USING paradedb (id, price, quantity)
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
        use datafusion::logical_expr::{Expr, col, lit};
        use filter_analyzer_helpers::{assert_exact, assert_unsupported};

        let fields = test_fields();
        let analyzer = FilterAnalyzer::new(&fields);

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
            let mut scan_info = ScanInfo::new(1, heap_oid, index_oid, crate::scan::ScanMode::all());

            for (i, field) in fields.iter().enumerate() {
                scan_info.add_field(i as pg_sys::AttrNumber, field.clone());
            }

            Arc::new(PgSearchTableProvider::new(scan_info, fields, None))
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

    #[pg_test]
    fn test_range_partitioning_sample_build() {
        use crate::api::FieldName;
        use crate::postgres::pdb_owned_value::PdbOwnedValue;
        use crate::scan::range_partitioning::RangePartitioningSample;

        let sample = RangePartitioningSample {
            partition_by: FieldName::from("id"),
            persisted_points: vec![],
            sample_points: vec![
                PdbOwnedValue::I64(10),
                PdbOwnedValue::I64(20),
                PdbOwnedValue::I64(30),
            ],
        };

        // Down-sample: target partitions (2) < sample size (4)
        // 2 partitions requires 1 split point.
        // i=1: (1 * 3) / 2 = 1. sample_points[1] is 20.
        let build_2 = sample.build(2);
        assert_eq!(build_2.split_points.len(), 1);
        assert_eq!(build_2.split_points[0], PdbOwnedValue::I64(20));

        // Exact match: target partitions (4) == sample size (4)
        // 4 partitions requires 3 split points.
        let build_4 = sample.build(4);
        assert_eq!(build_4.split_points.len(), 3);
        assert_eq!(build_4.split_points, sample.sample_points);

        // Up-sample / Pad: target partitions (6) > sample size (4)
        // Since we cap at sample_points.len() + 1, it will generate 3 split points (4 partitions).
        // The remaining 2 partitions will yield empty streams at execution time.
        let build_6 = sample.build(6);
        assert_eq!(build_6.split_points.len(), 3);
        assert_eq!(build_6.split_points[0], PdbOwnedValue::I64(10));
        assert_eq!(build_6.split_points[1], PdbOwnedValue::I64(20));
        assert_eq!(build_6.split_points[2], PdbOwnedValue::I64(30));

        // Single partition (no splits)
        let build_1 = sample.build(1);
        assert_eq!(build_1.split_points.len(), 0);
    }

    #[pg_test]
    fn test_range_partitioning_sample_nulls() {
        use crate::api::FieldName;
        use crate::postgres::pdb_owned_value::PdbOwnedValue;
        use crate::query::SearchQueryInput;
        use crate::scan::range_partitioning::RangePartitioningSample;

        let sample = RangePartitioningSample {
            partition_by: FieldName::from("id"),
            persisted_points: vec![],
            sample_points: vec![
                PdbOwnedValue::Null,
                PdbOwnedValue::Null,
                PdbOwnedValue::I64(10),
            ],
        };

        // Down-sample to 4 partitions: 3 split points
        let build = sample.build(4);
        assert_eq!(build.split_points.len(), 3);
        assert_eq!(build.split_points[0], PdbOwnedValue::Null);
        assert_eq!(build.split_points[1], PdbOwnedValue::Null);
        assert_eq!(build.split_points[2], PdbOwnedValue::I64(10));

        // partition 0: upper is Null -> is_empty_range -> returns Boolean(NOT Exists)
        let p0 = build.partition_bounds(0);
        assert!(matches!(p0, SearchQueryInput::Boolean { .. }));

        // partition 1: lower is Null (Unbounded), upper is Null (Empty) -> Empty
        let p1 = build.partition_bounds(1);
        assert!(matches!(p1, SearchQueryInput::Empty));

        // partition 2: lower is Null (Unbounded), upper is Excluded(10) -> Range
        let p2 = build.partition_bounds(2);
        assert!(matches!(p2, SearchQueryInput::FieldedQuery { .. }));

        // partition 3: lower is Included(10), upper is Unbounded -> Range
        let p3 = build.partition_bounds(3);
        assert!(matches!(p3, SearchQueryInput::FieldedQuery { .. }));
    }

    #[pg_test]
    fn test_range_partitioning_sample_all_nulls() {
        use crate::api::FieldName;
        use crate::postgres::pdb_owned_value::PdbOwnedValue;
        use crate::query::SearchQueryInput;
        use crate::scan::range_partitioning::RangePartitioningSample;

        let sample = RangePartitioningSample {
            partition_by: FieldName::from("id"),
            persisted_points: vec![],
            sample_points: vec![PdbOwnedValue::Null, PdbOwnedValue::Null],
        };

        let build = sample.build(3);
        assert_eq!(build.split_points.len(), 2);
        assert_eq!(build.split_points[0], PdbOwnedValue::Null);
        assert_eq!(build.split_points[1], PdbOwnedValue::Null);

        // partition 0: upper is Null -> is_empty_range -> returns Boolean(NOT Exists)
        let p0 = build.partition_bounds(0);
        assert!(matches!(p0, SearchQueryInput::Boolean { .. }));

        // partition 1: lower is Null (Unbounded), upper is Null (Empty) -> Empty
        let p1 = build.partition_bounds(1);
        assert!(matches!(p1, SearchQueryInput::Empty));

        // partition 2: lower is Null (Unbounded), upper is Unbounded -> Range
        let p2 = build.partition_bounds(2);
        assert!(matches!(p2, SearchQueryInput::FieldedQuery { .. }));
    }

    #[pg_test]
    fn test_range_partitioning_sample_identical_values() {
        use crate::api::FieldName;
        use crate::postgres::pdb_owned_value::PdbOwnedValue;
        use crate::query::SearchQueryInput;
        use crate::scan::range_partitioning::RangePartitioningSample;

        let sample = RangePartitioningSample {
            partition_by: FieldName::from("id"),
            persisted_points: vec![],
            sample_points: vec![
                PdbOwnedValue::I64(10),
                PdbOwnedValue::I64(10),
                PdbOwnedValue::I64(10),
            ],
        };

        let build = sample.build(4);
        assert_eq!(build.split_points.len(), 3);
        assert_eq!(build.split_points[0], PdbOwnedValue::I64(10));
        assert_eq!(build.split_points[1], PdbOwnedValue::I64(10));
        assert_eq!(build.split_points[2], PdbOwnedValue::I64(10));

        // partition 0: upper is 10 -> Range OR Boolean(NOT Exists)
        let p0 = build.partition_bounds(0);
        assert!(matches!(p0, SearchQueryInput::Boolean { .. }));

        // partition 1: lower is 10, upper is 10 -> Range
        let p1 = build.partition_bounds(1);
        assert!(matches!(p1, SearchQueryInput::FieldedQuery { .. }));

        // partition 2: lower is 10, upper is 10 -> Range
        let p2 = build.partition_bounds(2);
        assert!(matches!(p2, SearchQueryInput::FieldedQuery { .. }));

        // partition 3: lower is 10, upper is Unbounded -> Range
        let p3 = build.partition_bounds(3);
        assert!(matches!(p3, SearchQueryInput::FieldedQuery { .. }));
    }

    #[pg_test]
    #[allow(deprecated)] // Exercises PgSearchScanPlan's DataFusion partition-statistics contract.
    fn test_range_partitioning_repartition() {
        let (heap_oid, index_oid) = get_relation_oids();
        let heap_rel = PgSearchRelation::open(heap_oid);
        let index_rel = PgSearchRelation::open(index_oid);

        let reader = SearchIndexReader::open(
            &index_rel,
            SearchQueryInput::All,
            false,
            MvccSatisfies::Snapshot,
        )
        .unwrap();

        let fields = vec![
            WhichFastField::Ctid,
            WhichFastField::Named("id".to_string(), SearchFieldType::I64(pg_sys::INT4OID)),
        ];
        let ffhelper = FFHelper::with_fields(&reader, &fields);

        unsafe {
            pg_sys::CommandCounterIncrement();
            let snap = pg_sys::GetTransactionSnapshot();
            pg_sys::PushActiveSnapshot(snap);
        }
        let snapshot = unsafe { pg_sys::GetActiveSnapshot() };
        let visibility = HeapVisibilityChecker::with_rel_and_snap(&heap_rel, snapshot);

        let partition = crate::scan::execution_plan::ScanState {
            source_idx: None,
            planner_estimated_rows: 100,
            scanner_config: crate::scan::execution_plan::ScannerConfig {
                which_fast_fields: fields.clone(),
                heap_relid: heap_oid.into(),
                batch_size_hint: None,
                score_needed: false,
                scan_mode: crate::scan::ScanMode::all(),
            },
            ffhelper: ffhelper.into(),
            visibility: Box::new(visibility),
            reader: reader.clone(),
        };

        let sample = crate::scan::range_partitioning::RangePartitioningSample {
            partition_by: crate::api::FieldName::from("id"),
            persisted_points: vec![],
            sample_points: vec![
                crate::postgres::pdb_owned_value::PdbOwnedValue::I64(10),
                crate::postgres::pdb_owned_value::PdbOwnedValue::I64(20),
                crate::postgres::pdb_owned_value::PdbOwnedValue::I64(30),
                crate::postgres::pdb_owned_value::PdbOwnedValue::I64(40),
            ],
        };

        let plan = PgSearchScanPlan::new(
            Some(partition),
            build_arrow_schema(&fields),
            SearchQueryInput::All,
            None,
            Vec::new(),
            None,
            index_oid.into(),
            None,
            5,
            None,
            Some(sample),
        );

        use datafusion::physical_plan::Partitioning;

        // The boundaries cover the requested count exactly, so the plan declares
        // `Partitioning::Range` and DataFusion can co-partition against it.
        assert_eq!(plan.properties().output_partitioning().partition_count(), 5);
        assert!(matches!(
            plan.properties().output_partitioning(),
            Partitioning::Range(_)
        ));

        let plan_2 = plan.repartition(2).unwrap();
        assert_eq!(
            plan_2.properties().output_partitioning().partition_count(),
            2
        );
        assert!(matches!(
            plan_2.properties().output_partitioning(),
            Partitioning::Range(_)
        ));

        // 10 partitions exceed what the 4-point sample can bound: the plan keeps
        // the requested count as UnknownPartitioning and the extra partitions
        // execute as empty streams.
        let plan_10 = plan.repartition(10).unwrap();
        assert_eq!(
            plan_10.properties().output_partitioning().partition_count(),
            10
        );
        assert!(matches!(
            plan_10.properties().output_partitioning(),
            Partitioning::UnknownPartitioning(_)
        ));
        assert_eq!(
            plan_10.partition_statistics(Some(0)).unwrap().num_rows,
            Precision::Inexact(20)
        );
        assert_eq!(
            plan_10.partition_statistics(Some(9)).unwrap().num_rows,
            Precision::Inexact(0)
        );

        let empty_variant = plan_10
            .downcast_ref::<PgSearchScanPlan>()
            .unwrap()
            .with_assigned_partition(9);
        assert_eq!(
            empty_variant.partition_statistics(None).unwrap().num_rows,
            Precision::Inexact(0)
        );
        assert_eq!(
            empty_variant
                .partition_statistics(Some(0))
                .unwrap()
                .num_rows,
            Precision::Inexact(0)
        );
    }

    #[pg_test]
    fn test_range_partitioning_to_datafusion() {
        use crate::api::FieldName;
        use crate::postgres::pdb_owned_value::PdbOwnedValue;
        use crate::scan::range_partitioning::RangePartitioning;
        use arrow_schema::{DataType, Field, Schema};
        use datafusion::common::ScalarValue;
        use datafusion::physical_plan::Partitioning;

        let schema = Arc::new(Schema::new(vec![
            Field::new("ctid", DataType::UInt64, true),
            Field::new("id", DataType::Int64, true),
        ]));

        let boundaries = RangePartitioning {
            partition_by: FieldName::from("id"),
            split_points: vec![PdbOwnedValue::I64(10), PdbOwnedValue::I64(20)],
        };

        let partitioning = boundaries.to_datafusion(&schema).unwrap();
        assert_eq!(partitioning.partition_count(), 3);
        let Partitioning::Range(range) = &partitioning else {
            panic!("expected range partitioning, got {partitioning:?}");
        };
        assert_eq!(range.split_points().len(), 2);
        assert_eq!(
            range.split_points()[0].values(),
            &[ScalarValue::Int64(Some(10))]
        );
        assert_eq!(
            range.split_points()[1].values(),
            &[ScalarValue::Int64(Some(20))]
        );
        let sort_expr = range.ordering().iter().next().unwrap();
        assert_eq!(sort_expr.expr.to_string(), "id@1");
        assert!(!sort_expr.options.descending);
        assert!(sort_expr.options.nulls_first);

        // NULL split points have bespoke execution semantics that DataFusion's
        // model does not express; decline to declare.
        let with_null = RangePartitioning {
            partition_by: FieldName::from("id"),
            split_points: vec![PdbOwnedValue::Null],
        };
        assert!(with_null.to_datafusion(&schema).is_none());

        // The sampler's FFType-driven integer classification can yield U64 for an
        // Int64 column; the lossless cross-representation is accepted.
        let cross_int = RangePartitioning {
            partition_by: FieldName::from("id"),
            split_points: vec![PdbOwnedValue::U64(10)],
        };
        let cross_partitioning = cross_int.to_datafusion(&schema).unwrap();
        let Partitioning::Range(cross_range) = &cross_partitioning else {
            panic!("expected range partitioning, got {cross_partitioning:?}");
        };
        assert_eq!(
            cross_range.split_points()[0].values(),
            &[ScalarValue::Int64(Some(10))]
        );

        // Value/column type mismatches decline rather than declare imprecisely.
        let mismatched = RangePartitioning {
            partition_by: FieldName::from("id"),
            split_points: vec![PdbOwnedValue::F64(1.5)],
        };
        assert!(mismatched.to_datafusion(&schema).is_none());

        // Columns missing from the schema decline.
        let missing = RangePartitioning {
            partition_by: FieldName::from("missing"),
            split_points: vec![PdbOwnedValue::I64(10)],
        };
        assert!(missing.to_datafusion(&schema).is_none());
    }

    #[pg_test]
    #[allow(deprecated)] // Exercises PgSearchScanPlan's DataFusion partition-statistics contract.
    fn test_range_partitioned_assigned_execution() {
        use arrow_array::Int64Array;
        use datafusion::physical_plan::Partitioning;
        use datafusion_proto::physical_plan::DefaultPhysicalProtoConverter;

        let (heap_oid, index_oid) = get_relation_oids();
        let heap_rel = PgSearchRelation::open(heap_oid);
        let index_rel = PgSearchRelation::open(index_oid);

        let reader = SearchIndexReader::open(
            &index_rel,
            SearchQueryInput::All,
            false,
            MvccSatisfies::Snapshot,
        )
        .unwrap();

        let fields = vec![
            WhichFastField::Ctid,
            WhichFastField::Named("id".to_string(), SearchFieldType::I64(pg_sys::INT4OID)),
        ];
        let ffhelper = FFHelper::with_fields(&reader, &fields);

        unsafe {
            pg_sys::CommandCounterIncrement();
            let snap = pg_sys::GetTransactionSnapshot();
            pg_sys::PushActiveSnapshot(snap);
        }
        let snapshot = unsafe { pg_sys::GetActiveSnapshot() };
        let visibility = HeapVisibilityChecker::with_rel_and_snap(&heap_rel, snapshot);

        let scan_state = crate::scan::execution_plan::ScanState {
            source_idx: None,
            planner_estimated_rows: 100,
            scanner_config: crate::scan::execution_plan::ScannerConfig {
                which_fast_fields: fields.clone(),
                heap_relid: heap_oid.into(),
                batch_size_hint: None,
                score_needed: false,
                scan_mode: crate::scan::ScanMode::all(),
            },
            ffhelper: ffhelper.into(),
            visibility: Box::new(visibility),
            reader: reader.clone(),
        };

        // Table t has ids 1..=100; split points [25, 50, 75] give partitions
        // (-inf, 25), [25, 50), [50, 75), [75, inf).
        let sample = crate::scan::range_partitioning::RangePartitioningSample {
            partition_by: crate::api::FieldName::from("id"),
            persisted_points: vec![],
            sample_points: vec![
                crate::postgres::pdb_owned_value::PdbOwnedValue::I64(25),
                crate::postgres::pdb_owned_value::PdbOwnedValue::I64(50),
                crate::postgres::pdb_owned_value::PdbOwnedValue::I64(75),
            ],
        };

        let plan = PgSearchScanPlan::new(
            Some(scan_state),
            build_arrow_schema(&fields),
            SearchQueryInput::All,
            None,
            Vec::new(),
            None,
            index_oid.into(),
            None,
            4,
            None,
            Some(sample),
        );

        // The planner-facing original retains all four global ranges. Only the variant sent to
        // one distributed task advertises a single local partition.
        assert_eq!(plan.properties().output_partitioning().partition_count(), 4);
        assert!(matches!(
            plan.properties().output_partitioning(),
            Partitioning::Range(_)
        ));
        assert_eq!(
            plan.partition_statistics(None).unwrap().num_rows,
            Precision::Inexact(100)
        );
        assert_eq!(
            plan.partition_statistics(Some(1)).unwrap().num_rows,
            Precision::Inexact(25)
        );

        // As one of four task variants: this one owns partition 1 alone.
        let plan = plan.with_assigned_partition(1);

        assert!(plan.repartition(2).is_err());
        assert_eq!(plan.properties().output_partitioning().partition_count(), 1);
        assert!(matches!(
            plan.properties().output_partitioning(),
            Partitioning::UnknownPartitioning(1)
        ));
        assert_eq!(
            plan.partition_statistics(None).unwrap().num_rows,
            Precision::Inexact(25)
        );
        assert_eq!(
            plan.partition_statistics(Some(0)).unwrap().num_rows,
            Precision::Inexact(25)
        );
        assert!(plan.partition_statistics(Some(1)).is_err());

        // Dispatch must preserve the four global ranges even though this task-specialized
        // variant advertises one local partition to DataFusion.
        let proto_converter = DefaultPhysicalProtoConverter {};
        let encoded = plan.encode_for_dispatch(&proto_converter).unwrap();
        let task_context = TaskContext::default();
        let plan = PgSearchScanPlan::decode_for_dispatch(
            &encoded,
            None,
            None,
            &task_context,
            &proto_converter,
        )
        .unwrap();

        assert_eq!(plan.properties().output_partitioning().partition_count(), 1);
        assert!(matches!(
            plan.properties().output_partitioning(),
            Partitioning::UnknownPartitioning(1)
        ));

        // The specialized plan exposes only local partition 0. Rejecting another local
        // partition must not consume the assigned global partition's execution state.
        assert!(plan.execute(1, Arc::new(TaskContext::default())).is_err());

        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let collect_ids = |partition: usize| {
            let mut stream = plan
                .execute(partition, Arc::new(TaskContext::default()))
                .unwrap();
            let mut ids = Vec::new();
            runtime.block_on(async {
                while let Some(batch) = stream.next().await {
                    let batch = batch.unwrap();
                    let id_array = batch
                        .column(1)
                        .as_any()
                        .downcast_ref::<Int64Array>()
                        .expect("id should be an Int64Array");
                    ids.extend(id_array.values().iter().copied());
                }
            });
            ids.sort_unstable();
            ids
        };

        // Local partition 0 must map to global partition 1, not merely any 25-row range.
        assert_eq!(collect_ids(0), (25_i64..50).collect::<Vec<_>>());

        // The state is consumed exactly once.
        assert!(plan.execute(0, Arc::new(TaskContext::default())).is_err());
    }
}
