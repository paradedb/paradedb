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

pub mod metric;

use pgrx::{FromDatum, pg_sys};

/// Owned `Vec<f32>` extracted from a pgvector datum. pgvector's on-disk
/// layout (after the varlena header) is `[int16 dim][int16 unused]
/// [float4; dim]`. `from_datum` detoasts the datum, copies the floats
/// out, and frees the detoasted copy if it differs from the original
/// pointer.
#[derive(Debug, Clone, PartialEq)]
pub struct PgVector(pub Vec<f32>);

impl FromDatum for PgVector {
    unsafe fn from_polymorphic_datum(
        datum: pg_sys::Datum,
        is_null: bool,
        _typoid: pg_sys::Oid,
    ) -> Option<Self> {
        if is_null {
            return None;
        }
        let ptr = datum.cast_mut_ptr::<pg_sys::varlena>();
        let detoasted = pg_sys::pg_detoast_datum(ptr);
        let data = pgrx::varlena::vardata_any(detoasted);
        let dim = *(data as *const i16) as usize;
        let floats_ptr = data.add(4) as *const f32;
        let floats = std::slice::from_raw_parts(floats_ptr, dim).to_vec();
        if detoasted != ptr {
            pg_sys::pfree(detoasted as *mut std::ffi::c_void);
        }
        Some(Self(floats))
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use pgrx::prelude::*;
    use tantivy::schema::VectorDType;

    #[pg_test]
    fn f16_storage_search_and_f32_compatibility() {
        Spi::run("CREATE EXTENSION IF NOT EXISTS vector;
            SET max_parallel_workers_per_gather = 0;
            SET paradedb.global_mutable_segment_rows = 0;
            SET paradedb.vector_clustering_threshold = 64;
            SET paradedb.vector_cluster_max_probe = 1;
            CREATE TABLE half_vectors(id integer PRIMARY KEY, embedding vector(64));
            INSERT INTO half_vectors SELECT g, ARRAY(SELECT sin((g * 67 + i)::float8 * 0.071)::real
                FROM generate_series(1,64) i)::vector FROM generate_series(1,512) g;
            CREATE TABLE float_vectors AS TABLE half_vectors;
            CREATE INDEX half_vectors_idx ON half_vectors USING paradedb(id, embedding vector_cosine_ops)
                WITH (target_segment_count=1);
            CREATE INDEX float_vectors_idx ON float_vectors USING paradedb(id, embedding vector_cosine_ops)
                WITH (vector_fields='{\"embedding\":{\"dtype\":\"f32\"}}', target_segment_count=1);").unwrap();
        for (name, dtype) in [
            ("half_vectors_idx", VectorDType::F16),
            ("float_vectors_idx", VectorDType::F32),
        ] {
            let oid = Spi::get_one::<pg_sys::Oid>(&format!("SELECT '{name}'::regclass::oid"))
                .unwrap()
                .unwrap();
            let index = crate::postgres::rel::PgSearchRelation::open(oid);
            let (_, _, options) =
                crate::index::reader::open_vector_field(&index, "embedding").unwrap();
            assert_eq!(options.dtype(), dtype);
            assert_eq!(
                options.bytes_per_vector(),
                if dtype == VectorDType::F16 { 132 } else { 256 }
            );
        }
        let query = Spi::get_one::<String>("SELECT embedding::text FROM half_vectors WHERE id=123")
            .unwrap()
            .unwrap();
        let mut results = Vec::new();
        for table in ["half_vectors", "float_vectors"] {
            let sql = format!(
                "SELECT ARRAY(SELECT id FROM {table} WHERE id @@@ pdb.all() ORDER BY embedding <=> '{query}'::vector LIMIT 10)"
            );
            let ids = Spi::get_one::<Vec<i32>>(&sql).unwrap().unwrap();
            assert_eq!(ids[0], 123);
            results.push(ids);
        }
        assert!(
            results[0]
                .iter()
                .filter(|id| results[1].contains(id))
                .count()
                >= 9
        );
        Spi::run("DELETE FROM half_vectors WHERE id=123; REINDEX INDEX half_vectors_idx;").unwrap();
        let ids = Spi::get_one::<Vec<i32>>(&format!("SELECT ARRAY(SELECT id FROM half_vectors WHERE id @@@ pdb.all() ORDER BY embedding <=> '{query}'::vector LIMIT 10)")).unwrap().unwrap();
        assert_eq!(ids.len(), 10);
        assert!(!ids.contains(&123));
    }
}
