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

use rstest::*;
use sqlx::PgConnection;
use tests::fixtures::*;

/// tantivy rejects NaN/±Inf elements at `add_document` for cosine vector
/// fields (`InvalidArgument`, "non-finite element in vector field ..."):
/// such vectors cannot be unit-normalized at write time. This test documents
/// that the backstop is unreachable through SQL — pgvector enforces element
/// finiteness on every SQL-visible constructor of `vector` datums
/// (`vector_in`, `vector_recv`, array casts, and overflow checks on its
/// arithmetic operators), so a non-finite vector fails at the type level
/// before pg_search's ingestion (`PgVector::from_datum` →
/// `document.add_vector`) ever sees it.
///
/// If a non-finite vector datum ever did reach us (a weakened pgvector
/// invariant, or a C extension hand-building the varlena), tantivy's error
/// propagates through the insert path's `Result` → panic → Postgres ERROR
/// naming the vector field — a clean transaction abort, never swallowed.
///
/// There is no array bypass: `SearchFieldType::Vector` is produced only for
/// the pgvector `vector` type (`is_pgvector_oid`), so a `real[]`/`float4[]`
/// column can never declare a vector field — it maps to a numeric array
/// field, and pgvector's opclasses reject array columns at CREATE INDEX.
#[rstest]
async fn nonfinite_vector_rejected_at_type_level(mut conn: PgConnection) {
    r#"
        CREATE TABLE nonfinite_vec (
            id  int PRIMARY KEY,
            vec vector(3)
        );
        CREATE INDEX nonfinite_vec_idx ON nonfinite_vec
            USING bm25 (id, vec vector_cosine_ops)
            WITH (key_field = id);
    "#
    .execute(&mut conn);

    // vector_in: the text parser rejects NaN before a datum exists.
    match "INSERT INTO nonfinite_vec VALUES (1, '[NaN, 0, 0]')".execute_result(&mut conn) {
        Ok(_) => panic!("NaN vector literal should be rejected by pgvector"),
        Err(err) => assert!(
            err.to_string().contains("NaN not allowed in vector"),
            "unexpected error: {err}"
        ),
    }

    // array_to_vector: real[] casts are checked element-wise too, so an
    // expression-index shape over float arrays cannot smuggle one in either.
    match "INSERT INTO nonfinite_vec VALUES (2, ARRAY['Infinity'::float4, 0, 0]::vector)"
        .execute_result(&mut conn)
    {
        Ok(_) => panic!("infinite vector via real[] cast should be rejected by pgvector"),
        Err(err) => assert!(
            err.to_string()
                .contains("infinite value not allowed in vector"),
            "unexpected error: {err}"
        ),
    }

    // Control: finite vectors ingest into the cosine index and are
    // searchable, proving the rejections above happened before pg_search
    // rather than inside a broken index.
    "INSERT INTO nonfinite_vec VALUES (3, '[1, 0, 0]'), (4, '[0.5, 0.5, 0]')".execute(&mut conn);
    let rows =
        "SELECT id FROM nonfinite_vec WHERE id @@@ pdb.all() ORDER BY vec <=> '[1, 0, 0]' LIMIT 2"
            .fetch::<(i32,)>(&mut conn);
    assert_eq!(rows, vec![(3,), (4,)]);
}
