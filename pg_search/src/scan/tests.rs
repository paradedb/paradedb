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
    use crate::scan::datafusion_plan::SegmentPlan;
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

        let scanner = Scanner::new(
            search_results,
            Some(10), // batch size hint
            fields,
            heap_oid.into(),
        );

        let plan = SegmentPlan::new(
            scanner,
            ffhelper,
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
}
