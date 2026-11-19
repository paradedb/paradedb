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

pub use common::*;

pub mod api {
    pub use common::api::*;

    #[cfg(any(test, feature = "pg_test"))]
    pub mod vector_test_support {
        use crate::postgres::rel::PgSearchRelation;
        use crate::postgres::storage::block::{LinkedListData, SegmentMetaEntryContent};
        use crate::postgres::storage::buffer::BufferManager;
        use crate::postgres::storage::metadata::MetaPage;
        use pgrx::prelude::*;

        pub fn vector_metadata_fixture() -> PgSearchRelation {
            Spi::run("CREATE EXTENSION IF NOT EXISTS vector;
                SET paradedb.vector_clustering_threshold = 64;
                CREATE TABLE metadata_vectors(id int PRIMARY KEY, vec vector(1024));
                INSERT INTO metadata_vectors SELECT g, ARRAY(SELECT ((g+i)%17+1)::real
                    FROM generate_series(1,1024) i)::vector FROM generate_series(1,2048) g;
                CREATE INDEX metadata_vectors_idx ON metadata_vectors USING paradedb(id, vec vector_l2_ops)
                    WITH (quantization='[1,4]', target_segment_count=1);").unwrap();
            PgSearchRelation::open(
                Spi::get_one::<pg_sys::Oid>("SELECT 'metadata_vectors_idx'::regclass::oid")
                    .unwrap()
                    .unwrap(),
            )
        }

        pub fn set_single_layer_target(indexrel: &PgSearchRelation) {
            let mut settings: serde_json::Value = serde_json::from_slice(&unsafe {
                MetaPage::open(indexrel).settings_bytes().read_all()
            })
            .unwrap();
            settings["vector_quantization"][0]["layers"]
                .as_array_mut()
                .unwrap()
                .truncate(1);
            settings["vector_quantization"][0]["grids"]
                .as_array_mut()
                .unwrap()
                .retain(|grid| grid["bits"] == 1);
            MetaPage::replace_settings_for_test(indexrel, &serde_json::to_vec(&settings).unwrap());
        }

        pub fn vector_query() -> String {
            let query = vec!["1"; 1024].join(",");
            format!(
                "SELECT id FROM metadata_vectors WHERE id @@@ pdb.all() ORDER BY vec <-> '[{query}]'::vector LIMIT 5"
            )
        }

        pub fn expect_reindex(sql: &str) {
            Spi::run(&format!(
                r#"DO $body$
                DECLARE message text; hint text;
                BEGIN
                    BEGIN
                        EXECUTE {query};
                        RAISE EXCEPTION 'expected vector format failure';
                    EXCEPTION WHEN feature_not_supported THEN
                        GET STACKED DIAGNOSTICS message = MESSAGE_TEXT, hint = PG_EXCEPTION_HINT;
                        ASSERT position('metadata_vectors_idx' in message) > 0, message;
                        ASSERT hint = 'Rebuild index "metadata_vectors_idx" with REINDEX.', hint;
                    END;
                END $body$;"#,
                query = quote_literal(sql)
            ))
            .unwrap();
        }

        pub fn quote_literal(value: &str) -> String {
            format!("'{}'", value.replace("'", "''"))
        }

        pub fn replace_vector_version(indexrel: &PgSearchRelation, version: u32) {
            let entries = unsafe { MetaPage::open(indexrel).segment_metas().list(None) };
            let file = entries
                .iter()
                .filter(|entry| !entry.is_deleted())
                .find_map(|entry| match entry.content {
                    SegmentMetaEntryContent::Immutable(content) => content.vec,
                    _ => None,
                })
                .expect("fixture has a vector file");
            let mut bman = BufferManager::new(indexrel);
            let block = bman
                .get_buffer(file.starting_block)
                .page()
                .contents::<LinkedListData>()
                .start_blockno;
            let mut buffer = bman.get_buffer_mut(block);
            *buffer.page_mut().contents_mut::<u32>() = version.to_le();
        }
    }
}
pub mod index;
pub mod postgres;
pub mod schema;

pub use postgres::customscan;
