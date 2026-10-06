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

#[pgrx::pg_schema]
mod tests {
    use crate::index::mvcc::MvccSatisfies;
    use crate::index::reader::index::{SearchIndexReader, test_support::segmented_index_fixture};
    use crate::query::{SearchQueryInput, estimate::estimate_docs};
    use pgrx::prelude::*;

    #[pg_test]
    fn metadata_estimation_never_fetches_mlt_document() {
        let (index, _) = segmented_index_fixture("estimate_mlt", 1, false);
        let reader = SearchIndexReader::open(
            &index,
            SearchQueryInput::All,
            false,
            MvccSatisfies::Estimation,
        )
        .unwrap();
        let query =
            crate::query::more_like_this::MoreLikeThisQueryBuilder::new(Default::default(), None)
                .with_field_value(
                    "missing".into(),
                    crate::postgres::pdb_owned_value::PdbOwnedValue::U64(1),
                    None,
                    pg_sys::Oid::INVALID,
                );
        assert_eq!(
            estimate_docs(&query, &reader.segment_readers()[0], None).unwrap(),
            Some((1, 10))
        );
    }

    #[pg_test]
    fn metadata_estimation_uses_one_immutable_segment() {
        for (name, immutable) in [("estimate_mixed", 3), ("estimate_mutable", 0)] {
            let (index, _) = segmented_index_fixture(name, immutable, true);
            let reader = SearchIndexReader::open(
                &index,
                SearchQueryInput::All,
                false,
                MvccSatisfies::Estimation,
            )
            .unwrap();
            assert_eq!(reader.segment_readers().len(), usize::from(immutable != 0));
            if immutable != 0 {
                assert_eq!(reader.segment_readers()[0].max_doc(), 10);
                assert_eq!(
                    reader.estimate_metadata(&SearchQueryInput::All, None),
                    Some((1.0, 1.0))
                );
            }
            assert_eq!(
                crate::api::operator::estimate_selectivity_and_cost(
                    &index,
                    SearchQueryInput::All,
                    None
                )
                .0,
                Some(1.0)
            );
        }
    }

    #[pg_test]
    fn metadata_estimation_keeps_heap_planner_context() {
        Spi::run("CREATE TABLE estimate_heap_context (id bigint, title text, heap_value int);
            INSERT INTO estimate_heap_context SELECT g, CASE WHEN g % 2 = 0 THEN 'red' ELSE 'blue' END, g FROM generate_series(1,1000) g;
            CREATE INDEX estimate_heap_context_idx ON estimate_heap_context USING paradedb (id, title) WITH (target_segment_count=1);
            ANALYZE estimate_heap_context;
            SET LOCAL enable_seqscan=off;
            SET LOCAL max_parallel_workers_per_gather=0;").unwrap();
        let plan = Spi::get_one::<pgrx::Json>("EXPLAIN (FORMAT JSON) SELECT id FROM estimate_heap_context WHERE title @@@ 'red' AND heap_value >= 800").unwrap().unwrap().0;
        let rows = plan[0]["Plan"]["Plan Rows"].as_u64().unwrap();
        assert!((90..=111).contains(&rows), "{plan}");
        let count = Spi::get_one::<i64>("SELECT count(*) FROM estimate_heap_context WHERE title @@@ 'red' AND heap_value >= 800").unwrap().unwrap();
        assert_eq!(count, 101);
    }
}
