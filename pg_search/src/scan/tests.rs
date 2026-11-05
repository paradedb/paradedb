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
    use crate::api::CTID_FIELD_NAME;
    use crate::index::fast_fields_helper::{FFHelper, WhichFastField, build_arrow_schema};
    use crate::index::mvcc::MvccSatisfies;
    use crate::index::reader::index::SearchIndexReader;
    use crate::postgres::heap::VisibilityChecker as HeapVisibilityChecker;
    use crate::postgres::rel::PgSearchRelation;
    use crate::query::SearchQueryInput;
    use crate::scan::execution_plan::{PgSearchScanPlan, ReceivedClassification};
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
        Spi::run("CREATE INDEX t_idx ON t USING paradedb (id, (data::pdb.simple))").unwrap();

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

    fn scan_statistics_fixture() -> (SearchIndexReader, PgSearchRelation, Vec<WhichFastField>) {
        Spi::run(
            "CREATE TABLE scan_stats (id bigint PRIMARY KEY, label text, price double precision,
                                      missing bigint, flag boolean, amount numeric, missing_text text);
             CREATE INDEX scan_stats_idx ON scan_stats USING paradedb
                 (id, (label::pdb.literal), price, missing, flag, amount, (missing_text::pdb.literal))
                 WITH (target_segment_count = 8, background_layer_sizes = '0');
             SET paradedb.global_mutable_segment_rows = 0;
             INSERT INTO scan_stats VALUES (1, 'shared', -2.5, NULL, true, 1.25, NULL),
                                          (2, 'first', 1.5, NULL, false, 2.5, NULL);
             INSERT INTO scan_stats VALUES (3, 'shared', 4.5, NULL, true, 1.25, NULL),
                                          (4, 'last', NULL, NULL, false, 3.75, NULL);
             RESET paradedb.global_mutable_segment_rows;",
        )
        .unwrap();
        push_active_snapshot();
        let index_oid = Spi::get_one::<pg_sys::Oid>("SELECT 'scan_stats_idx'::regclass::oid")
            .unwrap()
            .unwrap();
        let heap_oid = Spi::get_one::<pg_sys::Oid>("SELECT 'scan_stats'::regclass::oid")
            .unwrap()
            .unwrap();
        let index = PgSearchRelation::open(index_oid);
        let reader = SearchIndexReader::open(
            &index,
            SearchQueryInput::All,
            false,
            MvccSatisfies::Snapshot,
        )
        .unwrap();
        let fields = vec![
            WhichFastField::Ctid,
            WhichFastField::eager("label", SearchFieldType::Text(pg_sys::TEXTOID)),
            WhichFastField::eager("id", SearchFieldType::I64(pg_sys::INT8OID)),
            WhichFastField::eager("price", SearchFieldType::F64(pg_sys::FLOAT8OID)),
            WhichFastField::eager("missing", SearchFieldType::I64(pg_sys::INT8OID)),
            WhichFastField::eager("flag", SearchFieldType::Bool(pg_sys::BOOLOID)),
            WhichFastField::eager(
                "amount",
                SearchFieldType::NumericBytes(pg_sys::NUMERICOID, None),
            ),
            WhichFastField::eager("missing_text", SearchFieldType::Text(pg_sys::TEXTOID)),
        ];
        (reader, PgSearchRelation::open(heap_oid), fields)
    }

    #[pg_test]
    fn scan_statistics_reuse_readers_and_persisted_bounds() {
        use crate::scan::statistics::column_statistics;
        use arrow_schema::DataType;
        use datafusion::common::ScalarValue;

        let (reader, _heap, fields) = scan_statistics_fixture();
        let helper = FFHelper::with_fields(&reader, &fields);
        let id = column_statistics(&helper, 2, &DataType::Int64, Some(4));
        assert_eq!(
            id.min_value,
            Precision::Inexact(ScalarValue::Int64(Some(1)))
        );
        assert_eq!(
            id.max_value,
            Precision::Inexact(ScalarValue::Int64(Some(4)))
        );
        for ord in 0..helper.num_segments() as u32 {
            assert!(
                helper.cached_column(ord, 2).is_none(),
                "persisted bounds must not open a value column"
            );
        }
        let empirical_reads = crate::index::segment_pruning::EMPIRICAL_READS
            .load(std::sync::atomic::Ordering::Relaxed);
        assert_eq!(column_statistics(&helper, 2, &DataType::Int64, Some(4)), id);
        assert_eq!(
            crate::index::segment_pruning::EMPIRICAL_READS
                .load(std::sync::atomic::Ordering::Relaxed),
            empirical_reads
        );

        for (field, data_type) in [
            (1, DataType::Utf8View),
            (3, DataType::Float64),
            (4, DataType::Int64),
            (6, DataType::BinaryView),
        ] {
            column_statistics(&helper, field, &data_type, Some(4));
        }
        let mut nullable_segments = 0;
        for ord in 0..helper.num_segments() as u32 {
            for field in [1, 6] {
                let cached = helper.cached_column(ord, field).unwrap().unwrap();
                assert!(
                    std::ptr::eq(cached, helper.column(ord, field)),
                    "execution must reuse statistics' dictionary reader"
                );
            }
            let price = helper.persisted_statistics(ord, 3).unwrap();
            if price.nullable {
                nullable_segments += 1;
                let cached = helper.cached_column(ord, 3).unwrap().unwrap();
                assert!(std::ptr::eq(cached, helper.column(ord, 3)));
            } else {
                assert!(helper.cached_column(ord, 3).is_none());
            }
            assert!(
                helper.cached_column(ord, 4).is_some(),
                "all-null columns require the reader fallback"
            );
        }
        assert_eq!(nullable_segments, 1);

        // Reading values first avoids a redundant persisted metadata lookup.
        let helper = FFHelper::with_fields(&reader, &fields);
        for ord in 0..helper.num_segments() as u32 {
            helper.column(ord, 2);
        }
        let empirical_reads = crate::index::segment_pruning::EMPIRICAL_READS
            .load(std::sync::atomic::Ordering::Relaxed);
        assert_eq!(column_statistics(&helper, 2, &DataType::Int64, Some(4)), id);
        assert_eq!(
            crate::index::segment_pruning::EMPIRICAL_READS
                .load(std::sync::atomic::Ordering::Relaxed),
            empirical_reads
        );
    }

    #[pg_test]
    fn scan_statistics_missing_persisted_bounds_share_fallback() {
        use crate::index::segment_pruning::{InjectedStatsFailure, inject_stats_failure};
        use crate::scan::statistics::column_statistics;
        use arrow_schema::DataType;
        use datafusion::common::ScalarValue;
        let (reader, _heap, fields) = scan_statistics_fixture();
        let guards: Vec<_> = reader
            .segment_ids()
            .into_iter()
            .map(|id| inject_stats_failure(id, InjectedStatsFailure::Missing))
            .collect();
        let helper = FFHelper::with_fields(&reader, &fields);
        let stats = column_statistics(&helper, 2, &DataType::Int64, Some(4));
        assert_eq!(
            stats.min_value,
            Precision::Inexact(ScalarValue::Int64(Some(1)))
        );
        assert_eq!(
            stats.max_value,
            Precision::Inexact(ScalarValue::Int64(Some(4)))
        );
        assert!(guards.iter().all(|guard| guard.hits() == 1));
        for ord in 0..helper.num_segments() as u32 {
            let cached = helper.cached_column(ord, 2).unwrap().unwrap();
            assert!(std::ptr::eq(cached, helper.column(ord, 2)));
        }
        assert_eq!(
            column_statistics(&helper, 2, &DataType::Int64, Some(4)),
            stats
        );
        assert!(guards.iter().all(|guard| guard.hits() == 1));
    }

    #[pg_test]
    fn scan_statistics_unreadable_persisted_bounds_raise() {
        use crate::index::segment_pruning::{InjectedStatsFailure, inject_stats_failure};
        use crate::scan::statistics::column_statistics;
        use arrow_schema::DataType;

        let (_reader, _heap, fields) = scan_statistics_fixture();
        for operation in [InjectedStatsFailure::Open, InjectedStatsFailure::Empirical] {
            // Each error probe needs an uncached snapshot.
            let index_oid = Spi::get_one::<pg_sys::Oid>("SELECT 'scan_stats_idx'::regclass::oid")
                .unwrap()
                .unwrap();
            let index = PgSearchRelation::open(index_oid);
            let reader = SearchIndexReader::empty(&index, MvccSatisfies::Snapshot).unwrap();
            let guard = inject_stats_failure(reader.segment_ids()[0], operation);
            let helper = FFHelper::with_fields(&reader, &fields);
            let error = pgrx::PgTryBuilder::new(std::panic::AssertUnwindSafe(|| {
                column_statistics(&helper, 2, &DataType::Int64, Some(4));
                None
            }))
            .catch_others(|caught| match caught {
                pgrx::pg_sys::panic::CaughtError::ErrorReport(report) => {
                    Some(report.message().to_string())
                }
                other => other.rethrow(),
            })
            .execute()
            .expect("unreadable metadata must raise");
            assert!(
                error.contains(&format!("injected {operation:?} statistics failure")),
                "{error}"
            );
            assert_eq!(guard.hits(), 1);
            assert!(
                helper.statistics(2).get().is_none(),
                "errors must not cache absence"
            );
            drop(guard);
            assert!(
                column_statistics(&helper, 2, &DataType::Int64, Some(4))
                    .min_value
                    .get_value()
                    .is_some()
            );
        }
    }

    fn scan_statistics_plan(
        reader: SearchIndexReader,
        heap: &PgSearchRelation,
        fields: Vec<WhichFastField>,
        estimated_rows: u64,
    ) -> PgSearchScanPlan {
        let helper = Arc::new(FFHelper::with_fields(&reader, &fields));
        let visibility =
            HeapVisibilityChecker::with_rel_and_snap(heap, unsafe { pg_sys::GetActiveSnapshot() })
                .with_ffhelper(Arc::clone(&helper));
        let state = crate::scan::execution_plan::ScanState {
            source_idx: None,
            planner_estimated_rows: estimated_rows,
            scanner_config: crate::scan::execution_plan::ScannerConfig {
                which_fast_fields: fields.clone(),
                heap_relid: heap.oid().into(),
                batch_size_hint: None,
                score_needed: false,
                scan_mode: crate::scan::ScanMode::all(),
            },
            ffhelper: helper,
            visibility: Box::new(visibility),
            reader,
        };
        PgSearchScanPlan::new(
            Some(state),
            build_arrow_schema(&fields),
            SearchQueryInput::All,
            None,
            vec![],
            None,
            0,
            None,
            1,
            None,
            None,
        )
    }

    #[pg_test]
    #[allow(deprecated)]
    fn scan_statistics_scalar_metadata_and_execution() {
        use datafusion::common::{ScalarValue, stats::ColumnStatistics};
        use datafusion::physical_plan::statistics::{StatisticsArgs, StatisticsContext};
        let (reader, heap, fields) = scan_statistics_fixture();
        assert_eq!(
            reader.segment_readers().len(),
            2,
            "dictionary overlap needs separate segments"
        );
        let plan = scan_statistics_plan(reader, &heap, fields, 4);
        let before = StatisticsContext::new()
            .compute(&plan, &StatisticsArgs::new())
            .unwrap();
        assert_eq!(before.num_rows, Precision::Inexact(4));
        assert_eq!(before.column_statistics.len(), plan.schema().fields().len());
        assert_eq!(
            before.column_statistics[1].distinct_count,
            Precision::Inexact(4),
            "sum dictionaries, retaining the shared term's overlap as an estimate"
        );
        assert_eq!(
            before.column_statistics[2].min_value,
            Precision::Inexact(ScalarValue::Int64(Some(1)))
        );
        assert_eq!(
            before.column_statistics[2].max_value,
            Precision::Inexact(ScalarValue::Int64(Some(4)))
        );
        assert_eq!(
            before.column_statistics[3].min_value,
            Precision::Inexact(ScalarValue::Float64(Some(-2.5)))
        );
        assert_eq!(
            before.column_statistics[3].max_value,
            Precision::Inexact(ScalarValue::Float64(Some(4.5)))
        );
        for index in [0, 4, 5] {
            assert_eq!(before.column_statistics[index], ColumnStatistics::default());
        }
        assert_eq!(
            before.column_statistics[6].distinct_count,
            Precision::Inexact(4)
        );
        assert_eq!(
            before.column_statistics[7].distinct_count,
            Precision::Inexact(0)
        );
        assert!(
            before
                .column_statistics
                .iter()
                .all(|s| s.null_count == Precision::Absent && s.sum_value == Precision::Absent)
        );
        let cloned = plan.clone();
        assert_eq!(cloned.partition_statistics(None).unwrap(), before);
        let mut stream = plan.execute(0, Arc::new(TaskContext::default())).unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let rows = runtime.block_on(async {
            let mut rows = 0;
            while let Some(batch) = stream.next().await {
                rows += batch.unwrap().num_rows();
            }
            rows
        });
        assert_eq!(rows, 4);
        assert_eq!(
            plan.partition_statistics(None).unwrap(),
            before,
            "execute must retain statistics"
        );
        assert_eq!(plan.partition_statistics(Some(0)).unwrap(), before);
        assert!(plan.partition_statistics(Some(1)).is_err());
    }

    #[pg_test]
    #[allow(deprecated)]
    fn scan_statistics_capping_deferred_and_partition_variants() {
        use crate::index::fast_fields_helper::{FieldCardinality, FieldDelivery};
        use crate::scan::statistics::column_statistics;
        use arrow_schema::DataType;
        use datafusion::common::stats::ColumnStatistics;
        use datafusion::physical_plan::statistics::{StatisticsArgs, StatisticsContext};
        let (reader, heap, fields) = scan_statistics_fixture();
        let helper = FFHelper::with_fields(&reader, &fields);
        assert!(helper.statistics(1).get().is_none());
        assert_eq!(
            column_statistics(&helper, 1, &DataType::Utf8View, Some(1)).distinct_count,
            Precision::Inexact(1),
            "filtered row estimate caps dictionary NDV"
        );
        assert_eq!(
            column_statistics(&helper, 1, &DataType::Utf8View, Some(0)).distinct_count,
            Precision::Inexact(0)
        );
        assert_eq!(
            helper.statistics(1).get().unwrap().distinct_count,
            Precision::Inexact(4),
            "cache the raw metadata once, applying row caps independently per request"
        );
        assert_eq!(
            column_statistics(&helper, 1, &DataType::UInt64, Some(4)),
            ColumnStatistics::default()
        );
        let mut deferred = fields.clone();
        deferred[1] = WhichFastField::named(
            "label",
            SearchFieldType::Text(pg_sys::TEXTOID),
            FieldCardinality::Scalar,
            FieldDelivery::Deferred,
        );
        let plan = scan_statistics_plan(reader.clone(), &heap, deferred, 4);
        assert_eq!(
            plan.partition_statistics(None).unwrap().column_statistics[1],
            ColumnStatistics::default()
        );
        let projected = datafusion::physical_plan::projection::ProjectionExec::try_new(
            [("label", 1), ("id", 2)]
                .into_iter()
                .map(|(name, index)| {
                    (
                        Arc::new(datafusion::physical_plan::expressions::Column::new(
                            name, index,
                        ))
                            as Arc<dyn datafusion::physical_expr::PhysicalExpr>,
                        name.to_string(),
                    )
                })
                .collect::<Vec<_>>(),
            Arc::new(plan),
        )
        .unwrap();
        let decode = crate::scan::tantivy_decode_exec::TantivyDecodeExec::new(
            Arc::new(projected),
            vec![crate::scan::deferred_lookup::PhysicalDeferredField {
                col_idx: 0,
                display_name: "label".into(),
                is_bytes: false,
                cardinality: FieldCardinality::Scalar,
                canonical: crate::index::fast_fields_helper::CanonicalColumn {
                    indexrelid: 0,
                    ff_index: 1,
                },
                heap_rti: 1,
                rebuild: None,
            }],
            [(0, Arc::new(FFHelper::with_fields(&reader, &fields)))]
                .into_iter()
                .collect(),
        )
        .unwrap();
        assert_eq!(
            StatisticsContext::new()
                .compute(&decode, &StatisticsArgs::new())
                .unwrap()
                .column_statistics[0]
                .distinct_count,
            Precision::Inexact(4),
            "decoding restores logical dictionary statistics"
        );
        let mut lists = fields.clone();
        lists[1] = WhichFastField::named(
            "label",
            SearchFieldType::Text(pg_sys::TEXTOID),
            FieldCardinality::List,
            FieldDelivery::Eager,
        );
        let list_helper = FFHelper::with_fields(&reader, &lists);
        assert_eq!(
            column_statistics(&list_helper, 1, &DataType::Utf8View, Some(4)),
            ColumnStatistics::default()
        );

        let plan = scan_statistics_plan(reader, &heap, fields, 4)
            .repartition(2)
            .unwrap();
        let plan = plan.downcast_ref::<PgSearchScanPlan>().unwrap();
        for partition in 0..2 {
            let statistics = plan.partition_statistics(Some(partition)).unwrap();
            assert_eq!(statistics.num_rows, Precision::Inexact(2));
            assert_eq!(
                statistics.column_statistics[1].distinct_count,
                Precision::Inexact(2)
            );
            let assigned = plan.with_assigned_partition(partition);
            assert_eq!(assigned.partition_statistics(None).unwrap(), statistics);
            assert_eq!(assigned.partition_statistics(Some(0)).unwrap(), statistics);
            assert!(assigned.partition_statistics(Some(1)).is_err());
        }
    }

    #[pg_test]
    fn scan_statistics_empty_and_mutable_readers() {
        use crate::index::reader::index::test_support::segmented_index_fixture;
        use crate::scan::statistics::column_statistics;
        use arrow_schema::DataType;
        use datafusion::common::stats::ColumnStatistics;
        for (name, batches, mutable) in [
            ("scan_statistics_empty", 0, false),
            ("scan_statistics_mutable", 1, true),
        ] {
            let (index, _) = segmented_index_fixture(name, batches, mutable);
            let reader = SearchIndexReader::empty(&index, MvccSatisfies::Snapshot).unwrap();
            let helper = FFHelper::with_fields(
                &reader,
                &[
                    WhichFastField::eager("id", SearchFieldType::I64(pg_sys::INT8OID)),
                    WhichFastField::eager("title", SearchFieldType::Text(pg_sys::TEXTOID)),
                ],
            );
            assert_eq!(
                column_statistics(&helper, 0, &DataType::Int64, None),
                ColumnStatistics::default()
            );
            let text = column_statistics(&helper, 1, &DataType::Utf8View, None);
            assert_eq!(
                text.distinct_count,
                if mutable {
                    Precision::Absent
                } else {
                    Precision::Inexact(0)
                }
            );
            assert_eq!(
                column_statistics(&helper, 1, &DataType::Utf8View, None),
                text
            );
        }
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
            WhichFastField::eager_scalar("id".to_string(), SearchFieldType::I64(pg_sys::INT4OID)),
        ];

        let ffhelper: Arc<FFHelper> = FFHelper::with_fields(&reader, &fields).into();

        // Ensure current transaction changes are visible
        unsafe {
            pg_sys::CommandCounterIncrement();
            let snap = pg_sys::GetTransactionSnapshot();
            pg_sys::PushActiveSnapshot(snap);
        }
        let snapshot = unsafe { pg_sys::GetActiveSnapshot() };
        let visibility = HeapVisibilityChecker::with_rel_and_snap(&heap_rel, snapshot)
            .with_ffhelper(Arc::clone(&ffhelper));

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
            ffhelper: Arc::clone(&ffhelper),
            visibility: Box::new(visibility),
            reader: reader.clone(),
        };

        let plan = PgSearchScanPlan::new(
            Some(partition),
            build_arrow_schema(&fields),
            SearchQueryInput::All,
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
            WhichFastField::eager_scalar(
                "id".to_string(),
                SearchFieldType::I64(pg_sys::InvalidOid),
            ),
            WhichFastField::eager_scalar(
                "price".to_string(),
                SearchFieldType::F64(pg_sys::InvalidOid),
            ),
            WhichFastField::eager_scalar(
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
             USING paradedb (id, price, quantity);",
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
    fn test_range_partitioning_points_build() {
        use crate::api::FieldName;
        use crate::postgres::pdb_owned_value::PdbOwnedValue;
        use crate::scan::range_partitioning::RangeSplitPoints;

        let split_points = RangeSplitPoints {
            partition_by: FieldName::from("id"),
            points: vec![
                PdbOwnedValue::I64(10),
                PdbOwnedValue::I64(20),
                PdbOwnedValue::I64(30),
            ],
        };

        // Down-sample: target partitions (2) < points (4)
        // 2 partitions requires 1 split point.
        // i=1: (1 * 3) / 2 = 1. points[1] is 20.
        let build_2 = split_points.build(2);
        assert_eq!(build_2.split_points.len(), 1);
        assert_eq!(build_2.split_points[0], PdbOwnedValue::I64(20));

        // Exact match: target partitions (4) == points (4)
        // 4 partitions requires 3 split points.
        let build_4 = split_points.build(4);
        assert_eq!(build_4.split_points.len(), 3);
        assert_eq!(build_4.split_points, split_points.points);

        // Capped: target partitions (6) > points (4)
        // Since we cap at points.len() + 1, it will generate 3 split points (4 partitions).
        // The remaining 2 partitions will yield empty streams at execution time.
        let build_6 = split_points.build(6);
        assert_eq!(build_6.split_points.len(), 3);
        assert_eq!(build_6.split_points[0], PdbOwnedValue::I64(10));
        assert_eq!(build_6.split_points[1], PdbOwnedValue::I64(20));
        assert_eq!(build_6.split_points[2], PdbOwnedValue::I64(30));

        // Single partition (no splits)
        let build_1 = split_points.build(1);
        assert_eq!(build_1.split_points.len(), 0);
    }

    /// Execute the generated bounds: a pure-negative NULL clause has the right shape but
    /// returns no rows in Tantivy unless it also has a positive All clause.
    #[pg_test]
    fn test_range_partition_queries_cover_nulls_once() {
        use crate::api::FieldName;
        use crate::index::stats::segments_for_partition;
        use crate::postgres::pdb_owned_value::PdbOwnedValue::{I64, Null};
        use crate::scan::range_partitioning::RangePartitioning;

        Spi::run(
            "CREATE TABLE null_partition_rows (id bigint PRIMARY KEY, value bigint);
             CREATE INDEX null_partition_rows_idx ON null_partition_rows
             USING paradedb (id, value)
             WITH (target_segment_count = 8, background_layer_sizes = '0');
             SET paradedb.global_mutable_segment_rows = 0;
             INSERT INTO null_partition_rows VALUES (1, NULL), (2, -10), (3, 10), (4, 20), (5, NULL);
             RESET paradedb.global_mutable_segment_rows;",
        ).unwrap();
        unsafe { pg_sys::CommandCounterIncrement() };
        let oid = Spi::get_one::<pg_sys::Oid>("SELECT 'null_partition_rows_idx'::regclass::oid")
            .unwrap()
            .unwrap();
        let index_rel = PgSearchRelation::open(oid);
        for scoring in [false, true] {
            let reader = SearchIndexReader::open(
                &index_rel,
                SearchQueryInput::All,
                scoring,
                MvccSatisfies::Snapshot,
            )
            .unwrap();
            for (split_points, expected) in [
                (vec![], vec![vec![1, 2, 3, 4, 5]]),
                (vec![I64(10)], vec![vec![1, 2, 5], vec![3, 4]]),
                (vec![Null], vec![vec![1, 5], vec![2, 3, 4]]),
                (vec![Null, Null], vec![vec![1, 5], vec![], vec![2, 3, 4]]),
                (
                    vec![Null, Null, I64(10)],
                    vec![vec![1, 5], vec![], vec![2], vec![3, 4]],
                ),
                (
                    vec![I64(10), I64(10)],
                    vec![vec![1, 2, 5], vec![], vec![3, 4]],
                ),
            ] {
                let partitioning = RangePartitioning {
                    partition_by: FieldName::from("value"),
                    split_points,
                };
                for (partition, expected_ids) in expected.into_iter().enumerate() {
                    let query = partitioning.partition_bounds(partition);
                    let exact = reader.and_query_input(&query);
                    let ids = segments_for_partition(&reader, &partitioning, partition);
                    let collect_ids =
                        |results: crate::index::reader::index::MultiSegmentSearchResults| {
                            let mut ids = results
                                .map(|(_, doc)| {
                                    reader
                                        .searcher()
                                        .segment_reader(doc.segment_ord)
                                        .fast_fields()
                                        .i64("id")
                                        .unwrap()
                                        .first(doc.doc_id)
                                        .unwrap()
                                })
                                .collect::<Vec<_>>();
                            ids.sort_unstable();
                            ids
                        };
                    assert_eq!(
                        collect_ids(exact.search()),
                        expected_ids,
                        "compiled bounds {query:?}, scoring={scoring}"
                    );
                    assert_eq!(
                        collect_ids(exact.search_segments(ids.into_iter())),
                        expected_ids,
                        "routed bounds {query:?}, scoring={scoring}"
                    );
                }
            }
        }
    }

    #[pg_test]
    fn test_range_partitioning_points_nulls() {
        use crate::api::FieldName;
        use crate::postgres::pdb_owned_value::PdbOwnedValue;
        use crate::query::SearchQueryInput;
        use crate::scan::range_partitioning::RangeSplitPoints;

        let split_points = RangeSplitPoints {
            partition_by: FieldName::from("id"),
            points: vec![
                PdbOwnedValue::Null,
                PdbOwnedValue::Null,
                PdbOwnedValue::I64(10),
            ],
        };

        // Down-sample to 4 partitions: 3 split points
        let build = split_points.build(4);
        assert_eq!(build.split_points.len(), 3);
        assert_eq!(build.split_points[0], PdbOwnedValue::Null);
        assert_eq!(build.split_points[1], PdbOwnedValue::Null);
        assert_eq!(build.split_points[2], PdbOwnedValue::I64(10));

        // partition 0: upper is NULL -> only NULL rows (All AND NOT Exists).
        let p0 = build.partition_bounds(0);
        assert!(matches!(
            p0,
            SearchQueryInput::Boolean { ref must, .. }
                if matches!(must.as_slice(), [SearchQueryInput::All])
        ));

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
    fn test_range_partitioning_points_all_nulls() {
        use crate::api::FieldName;
        use crate::postgres::pdb_owned_value::PdbOwnedValue;
        use crate::query::SearchQueryInput;
        use crate::scan::range_partitioning::RangeSplitPoints;

        let split_points = RangeSplitPoints {
            partition_by: FieldName::from("id"),
            points: vec![PdbOwnedValue::Null, PdbOwnedValue::Null],
        };

        let build = split_points.build(3);
        assert_eq!(build.split_points.len(), 2);
        assert_eq!(build.split_points[0], PdbOwnedValue::Null);
        assert_eq!(build.split_points[1], PdbOwnedValue::Null);

        // partition 0: upper is NULL -> only NULL rows (All AND NOT Exists).
        let p0 = build.partition_bounds(0);
        assert!(matches!(
            p0,
            SearchQueryInput::Boolean { ref must, .. }
                if matches!(must.as_slice(), [SearchQueryInput::All])
        ));

        // partition 1: lower is Null (Unbounded), upper is Null (Empty) -> Empty
        let p1 = build.partition_bounds(1);
        assert!(matches!(p1, SearchQueryInput::Empty));

        // partition 2: both bounds are Unbounded -> Exists (all non-NULL values)
        let p2 = build.partition_bounds(2);
        assert!(matches!(p2, SearchQueryInput::FieldedQuery { .. }));
    }

    #[pg_test]
    fn test_range_partitioning_points_identical_values() {
        use crate::api::FieldName;
        use crate::postgres::pdb_owned_value::PdbOwnedValue;
        use crate::query::SearchQueryInput;
        use crate::query::pdb_query::pdb::Query;
        use crate::scan::range_partitioning::RangeSplitPoints;
        use std::ops::Bound;

        let split_points = RangeSplitPoints {
            partition_by: FieldName::from("id"),
            points: vec![
                PdbOwnedValue::I64(10),
                PdbOwnedValue::I64(10),
                PdbOwnedValue::I64(10),
            ],
        };

        let build = split_points.build(4);
        assert_eq!(build.split_points.len(), 3);
        assert_eq!(build.split_points[0], PdbOwnedValue::I64(10));
        assert_eq!(build.split_points[1], PdbOwnedValue::I64(10));
        assert_eq!(build.split_points[2], PdbOwnedValue::I64(10));

        // partition 0: upper is 10 -> All AND NOT Range(at or above 10), which keeps the NULLs
        // without a union.
        let p0 = build.partition_bounds(0);
        let SearchQueryInput::Boolean {
            must,
            should,
            must_not,
            ..
        } = p0
        else {
            panic!("expected a Boolean, got {p0:?}");
        };
        assert!(matches!(must.as_slice(), [SearchQueryInput::All]));
        assert!(should.is_empty());
        assert!(matches!(
            must_not.as_slice(),
            [SearchQueryInput::FieldedQuery {
                field,
                query: Query::StoredRange {
                    lower_bound: Bound::Included(PdbOwnedValue::I64(10)),
                    upper_bound: Bound::Unbounded,
                },
            }] if field.as_ref() == "id"
        ));

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
    fn test_range_partitioning_bounds_stay_in_stored_form() {
        use crate::api::FieldName;
        use crate::postgres::pdb_owned_value::PdbOwnedValue;
        use crate::postgres::rel::PgSearchRelation;
        use crate::query::numeric::numeric_value_to_decimal_bytes;
        use crate::scan::range_partitioning::RangeSplitPoints;

        let matching_ids = |reader: &SearchIndexReader| {
            let fields = [WhichFastField::eager_scalar(
                "id".to_string(),
                SearchFieldType::I64(pg_sys::INT4OID),
            )];
            let fast_fields = FFHelper::with_fields(reader, &fields);
            let mut ids = reader
                .search()
                .map(|(_, doc_address)| {
                    i64::try_from(
                        fast_fields
                            .value(0, doc_address)
                            .expect("id should be a fast field"),
                    )
                    .expect("id should be stored as an I64")
                })
                .collect::<Vec<_>>();
            ids.sort_unstable();
            ids
        };

        Spi::run(
            "CREATE TABLE numeric_partition_bounds_test (
                id serial PRIMARY KEY,
                amount numeric(10,2) NOT NULL
            );
            INSERT INTO numeric_partition_bounds_test (amount)
            VALUES (1.22), (1.23), (4.55), (4.56);
            CREATE INDEX numeric_partition_bounds_test_idx
            ON numeric_partition_bounds_test USING paradedb (id, amount);",
        )
        .expect("create Numeric64 partition-bound fixture");

        let index_oid = Spi::get_one::<pg_sys::Oid>(
            "SELECT oid
             FROM pg_class
             WHERE relname = 'numeric_partition_bounds_test_idx'
               AND relkind = 'i';",
        )
        .expect("look up test index")
        .expect("test index should exist");
        let index_rel = PgSearchRelation::open(index_oid);

        let split_points = RangeSplitPoints {
            partition_by: FieldName::from("amount"),
            points: vec![PdbOwnedValue::I64(123), PdbOwnedValue::I64(456)],
        };
        let build = split_points.build(3);

        let p1 = build.partition_bounds(1);

        let reader = SearchIndexReader::open(&index_rel, p1, false, MvccSatisfies::Snapshot)
            .expect("open reader with stored partition bounds");
        assert_eq!(matching_ids(&reader), vec![2, 3]);

        Spi::run(
            "CREATE TABLE numeric_bytes_partition_bounds_test (
                id serial PRIMARY KEY,
                amount numeric NOT NULL
            );
            INSERT INTO numeric_bytes_partition_bounds_test (amount)
            VALUES (1.22), (1.23), (4.55), (4.56);
            CREATE INDEX numeric_bytes_partition_bounds_test_idx
            ON numeric_bytes_partition_bounds_test USING paradedb (id, amount);",
        )
        .expect("create NumericBytes partition-bound fixture");

        let index_oid = Spi::get_one::<pg_sys::Oid>(
            "SELECT oid
             FROM pg_class
             WHERE relname = 'numeric_bytes_partition_bounds_test_idx'
               AND relkind = 'i';",
        )
        .expect("look up NumericBytes test index")
        .expect("NumericBytes test index should exist");
        let index_rel = PgSearchRelation::open(index_oid);
        let index_created_by_version = index_rel.created_by_version();

        let stored_numeric = |value: &str| {
            numeric_value_to_decimal_bytes(
                PdbOwnedValue::Str(value.to_owned()),
                index_created_by_version,
            )
            .expect("encode NumericBytes split point")
        };
        let lower = stored_numeric("1.23");
        let upper = stored_numeric("4.56");

        let split_points = RangeSplitPoints {
            partition_by: FieldName::from("amount"),
            points: vec![lower.clone(), upper.clone()],
        };
        let p1 = split_points.build(3).partition_bounds(1);

        let reader = SearchIndexReader::open(&index_rel, p1, false, MvccSatisfies::Snapshot)
            .expect("open reader with stored NumericBytes partition bounds");
        assert_eq!(matching_ids(&reader), vec![2, 3]);
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
            WhichFastField::eager_scalar("id".to_string(), SearchFieldType::I64(pg_sys::INT4OID)),
        ];
        let ffhelper: Arc<FFHelper> = FFHelper::with_fields(&reader, &fields).into();

        unsafe {
            pg_sys::CommandCounterIncrement();
            let snap = pg_sys::GetTransactionSnapshot();
            pg_sys::PushActiveSnapshot(snap);
        }
        let snapshot = unsafe { pg_sys::GetActiveSnapshot() };
        let visibility = HeapVisibilityChecker::with_rel_and_snap(&heap_rel, snapshot)
            .with_ffhelper(Arc::clone(&ffhelper));

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
            ffhelper: Arc::clone(&ffhelper),
            visibility: Box::new(visibility),
            reader: reader.clone(),
        };

        let split_points = crate::scan::range_partitioning::RangeSplitPoints {
            partition_by: crate::api::FieldName::from("id"),
            points: vec![
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
            Vec::new(),
            None,
            index_oid.into(),
            None,
            5,
            None,
            Some(split_points),
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

        // 10 partitions exceed what 4 split points seat: the plan caps itself at 5 and still
        // declares `Partitioning::Range`, so no task is empty.
        let plan_10 = plan.repartition(10).unwrap();
        assert_eq!(
            plan_10.properties().output_partitioning().partition_count(),
            5
        );
        assert!(matches!(
            plan_10.properties().output_partitioning(),
            Partitioning::Range(_)
        ));
        assert_eq!(
            plan_10.partition_statistics(Some(0)).unwrap().num_rows,
            Precision::Inexact(20)
        );
        assert_eq!(
            plan_10.partition_statistics(Some(4)).unwrap().num_rows,
            Precision::Inexact(20)
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
            Field::new(CTID_FIELD_NAME, DataType::UInt64, true),
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

        // A split point can arrive as U64 for an Int64 column; the lossless
        // cross-representation is accepted.
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

        // Stored NumericBytes split points are declared as BinaryView without
        // altering their byte representation.
        let binary_schema = Arc::new(Schema::new(vec![
            Field::new(CTID_FIELD_NAME, DataType::UInt64, true),
            Field::new("amount", DataType::BinaryView, true),
        ]));
        let first_bytes = vec![0x01, 0x00, 0xff];
        let second_bytes = vec![0x02, 0x00, 0x00];
        let binary_boundaries = RangePartitioning {
            partition_by: FieldName::from("amount"),
            split_points: vec![
                PdbOwnedValue::Bytes(first_bytes.clone()),
                PdbOwnedValue::Bytes(second_bytes.clone()),
            ],
        };
        let binary_partitioning = binary_boundaries.to_datafusion(&binary_schema).unwrap();
        assert_eq!(binary_partitioning.partition_count(), 3);
        let Partitioning::Range(binary_range) = &binary_partitioning else {
            panic!("expected range partitioning, got {binary_partitioning:?}");
        };
        assert_eq!(
            binary_range.split_points()[0].values(),
            &[ScalarValue::BinaryView(Some(first_bytes))]
        );
        assert_eq!(
            binary_range.split_points()[1].values(),
            &[ScalarValue::BinaryView(Some(second_bytes))]
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

    #[test]
    fn test_range_split_points_to_datafusion_and_scaling() {
        use crate::api::FieldName;
        use crate::postgres::pdb_owned_value::PdbOwnedValue;
        use crate::scan::range_partitioning::RangeSplitPoints;
        use arrow_schema::{DataType, Field, Schema};
        use datafusion::common::ScalarValue;
        use datafusion::physical_plan::Partitioning;

        let schema = Arc::new(Schema::new(vec![
            Field::new(CTID_FIELD_NAME, DataType::UInt64, true),
            Field::new("id", DataType::Int64, true),
        ]));

        // 5 persisted points: 10, 20, 30, 40, 50
        let points = RangeSplitPoints {
            partition_by: FieldName::from("id"),
            points: vec![
                PdbOwnedValue::I64(10),
                PdbOwnedValue::I64(20),
                PdbOwnedValue::I64(30),
                PdbOwnedValue::I64(40),
                PdbOwnedValue::I64(50),
            ],
        };

        // Target partition count 3
        let partitioning = points.to_datafusion(&schema, 3).unwrap();
        assert_eq!(partitioning.partition_count(), 3);
        let Partitioning::Range(df_range) = &partitioning else {
            panic!("expected range partitioning, got {partitioning:?}");
        };
        // Samples retain all 5 persisted points
        assert_eq!(df_range.samples().len(), 5);
        assert_eq!(df_range.split_points().len(), 2);

        // Scaling DataFusion partitioning to T=2 must produce split points identical
        // to RangeSplitPoints::build(2)
        let scaled = df_range.scale(2).unwrap();
        let built = points.build(2);
        assert_eq!(scaled.split_points().len(), 1);
        assert_eq!(built.split_points.len(), 1);
        assert_eq!(
            scaled.split_points()[0].values(),
            &[ScalarValue::Int64(Some(30))]
        );
        assert_eq!(built.split_points[0], PdbOwnedValue::I64(30));

        // Invalid partition count (<= 1 or > points.len() + 1) returns None
        assert!(points.to_datafusion(&schema, 1).is_none());
        assert!(points.to_datafusion(&schema, 0).is_none());
        assert!(points.to_datafusion(&schema, 7).is_none());
    }

    /// Like [`get_relation_oids`], but built as four range partitions of 25 rows, so the segments
    /// are 1..=25, 26..=50, 51..=75 and 76..=100.
    fn get_partitioned_relation_oids() -> (pg_sys::Oid, pg_sys::Oid) {
        Spi::run("SET client_min_messages = 'debug1';").unwrap();
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run("INSERT INTO t (data) SELECT 'test ' || i FROM generate_series(1, 100) i;")
            .unwrap();
        Spi::run(
            "CREATE INDEX t_idx ON t USING paradedb (id, (data::pdb.simple))
             WITH (partition_by = 'id', target_segment_count = 4)",
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
    #[allow(deprecated)] // Exercises PgSearchScanPlan's DataFusion partition-statistics contract.
    fn test_range_partitioned_assigned_execution() {
        use crate::api::FieldName;
        use crate::index::reader::index::test_support::range_query;
        use crate::index::segment_pruning::{EMPIRICAL_READS, STATS_OPENS};
        use crate::postgres::pdb_owned_value::PdbOwnedValue;
        use arrow_array::Int64Array;
        use datafusion::physical_plan::Partitioning;
        use datafusion_proto::physical_plan::DefaultPhysicalProtoConverter;
        use std::sync::atomic::Ordering::Relaxed;
        use tantivy::index::SegmentId;

        let (heap_oid, index_oid) = get_partitioned_relation_oids();
        let heap_rel = PgSearchRelation::open(heap_oid);
        let index_rel = PgSearchRelation::open(index_oid);

        let fields = vec![
            WhichFastField::Ctid,
            WhichFastField::eager_scalar("id".to_string(), SearchFieldType::I64(pg_sys::INT4OID)),
        ];
        unsafe {
            pg_sys::CommandCounterIncrement();
            let snap = pg_sys::GetTransactionSnapshot();
            pg_sys::PushActiveSnapshot(snap);
        }
        let snapshot = unsafe { pg_sys::GetActiveSnapshot() };

        // Table t has ids 1..=100; split points [25, 51, 75] give partitions
        // (-inf, 25), [25, 51), [51, 75), [75, inf).
        let split_points = crate::scan::range_partitioning::RangeSplitPoints {
            partition_by: FieldName::from("id"),
            points: vec![
                PdbOwnedValue::I64(25),
                PdbOwnedValue::I64(51),
                PdbOwnedValue::I64(75),
            ],
        };

        let make_plan = |query: SearchQueryInput| {
            let reader =
                SearchIndexReader::open(&index_rel, query.clone(), false, MvccSatisfies::Snapshot)
                    .unwrap();
            let ffhelper: Arc<FFHelper> = FFHelper::with_fields(&reader, &fields).into();
            let visibility = Box::new(
                HeapVisibilityChecker::with_rel_and_snap(&heap_rel, snapshot)
                    .with_ffhelper(Arc::clone(&ffhelper)),
            );
            let scan_state = crate::scan::execution_plan::ScanState {
                source_idx: None,
                planner_estimated_rows: 100,
                scanner_config: crate::scan::execution_plan::ScannerConfig {
                    which_fast_fields: fields.clone(),
                    heap_relid: heap_oid.into(),
                    batch_size_hint: None,
                    score_needed: false,
                    scan_mode: crate::scan::ScanMode::standard(query.clone()),
                },
                ffhelper,
                visibility,
                reader: reader.clone(),
            };
            let plan = PgSearchScanPlan::new(
                Some(scan_state),
                build_arrow_schema(&fields),
                query,
                Vec::new(),
                None,
                index_oid.into(),
                None,
                4,
                None,
                Some(split_points.clone()),
            );
            (plan, reader)
        };
        let (plan, reader) = make_plan(SearchQueryInput::All);

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

        // As one of four task variants: this one owns partition 1 alone. Assignment is where
        // the leader classifies segments, so it is the only step allowed to open statistics.
        STATS_OPENS.store(0, Relaxed);
        let unassigned = plan.clone();
        let plan = plan.with_assigned_partition(1);
        let leader_opens = STATS_OPENS.load(Relaxed);
        assert!(leader_opens > 0, "assignment classifies from statistics");
        let classified = plan.partition_segments.clone().unwrap();
        // [25, 51) fully includes 26..=50, partially includes 1..=25 through id 25, and prunes
        // the two segments above it.
        assert_eq!(classified.included.len(), 1);
        assert_eq!(classified.partially_included.len(), 1);
        assert_eq!(classified.pruned.len(), 2);

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
        let decode = |bytes: &[u8], parallel_state| {
            PgSearchScanPlan::decode_for_dispatch(
                bytes,
                parallel_state,
                None,
                &task_context,
                &proto_converter,
            )
        };

        for _ in 0..3 {
            let copy = decode(&encoded, None).unwrap();
            let copy = copy.downcast_ref::<PgSearchScanPlan>().unwrap();
            assert_eq!(copy.partition_segments.as_ref(), Some(&classified));
        }
        assert_eq!(STATS_OPENS.load(Relaxed), leader_opens);

        // Each bad shape keeps the segment count, so only identity can reject it. Under the
        // shared view that is an error; outside it the copy classifies again.
        let mut stale = classified.clone();
        stale.pruned[0] = SegmentId::generate_random();
        let mut duplicated = classified.clone();
        duplicated.pruned[0] = classified.included[0];
        let mut short = classified.clone();
        short.pruned.pop();
        for bad in [&stale, &duplicated, &short] {
            assert!(
                unassigned
                    .assign_partition(1, Some(ReceivedClassification::SharedView(bad.clone())))
                    .is_err(),
                "{bad:?}"
            );
        }
        // This copy shares the leader's opened statistics, so reclassification shows up as
        // field decodes rather than component opens.
        let reads_before = EMPIRICAL_READS.load(Relaxed);
        let recomputed = unassigned
            .assign_partition(1, Some(ReceivedClassification::Snapshot(stale.clone())))
            .unwrap();
        assert_eq!(recomputed.partition_segments.as_ref(), Some(&classified));
        assert!(EMPIRICAL_READS.load(Relaxed) > reads_before);
        drop(recomputed);
        drop(unassigned);

        // Under the leader's parallel state the worker replays the leader's view.
        let parallel_state =
            crate::postgres::test_support::parallel_state_for_view(reader.segment_view());
        let mut stale_plan = (*plan).clone();
        stale_plan.partition_segments = Some(stale);
        let stale_encoded = stale_plan.encode_for_dispatch(&proto_converter).unwrap();
        assert!(decode(&stale_encoded, Some(parallel_state)).is_err());
        let plan = decode(&encoded, Some(parallel_state)).unwrap();
        assert_eq!(
            plan.downcast_ref::<PgSearchScanPlan>()
                .unwrap()
                .partition_segments
                .as_ref(),
            Some(&classified)
        );
        assert_eq!(STATS_OPENS.load(Relaxed), leader_opens);

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
        let collect_ids = |plan: &Arc<dyn ExecutionPlan>, partition: usize| {
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

        // Local partition 0 must map to global partition 1, not merely any 26-row range.
        assert_eq!(collect_ids(&plan, 0), (25_i64..=50).collect::<Vec<_>>());
        assert_eq!(STATS_OPENS.load(Relaxed), leader_opens);

        // The state is consumed exactly once.
        assert!(plan.execute(0, Arc::new(TaskContext::default())).is_err());

        // A predicate on the partition column still prunes after dispatch. Decode reuses the
        // leader's partition classification without opening statistics; execution then opens
        // statistics and decides each searched segment against the predicate.
        let (filtered, _) = make_plan(range_query("id", 30, 40));
        let filtered = filtered.with_assigned_partition(1);
        let filtered_encoded = filtered.encode_for_dispatch(&proto_converter).unwrap();
        let opens_before_decode = STATS_OPENS.load(Relaxed);
        let worker = decode(&filtered_encoded, Some(parallel_state)).unwrap();
        assert_eq!(
            worker
                .downcast_ref::<PgSearchScanPlan>()
                .unwrap()
                .partition_segments,
            filtered.partition_segments
        );
        assert_eq!(
            STATS_OPENS.load(Relaxed),
            opens_before_decode,
            "decode reuses the leader's classification without opening statistics"
        );
        let reads_before_execution = EMPIRICAL_READS.load(Relaxed);
        assert_eq!(collect_ids(&worker, 0), (30_i64..=40).collect::<Vec<_>>());
        assert!(
            STATS_OPENS.load(Relaxed) > opens_before_decode,
            "the worker must open statistics for its own predicate during execution"
        );
        assert!(
            EMPIRICAL_READS.load(Relaxed) > reads_before_execution,
            "execution must decide each searched segment against the predicate"
        );
    }
}
