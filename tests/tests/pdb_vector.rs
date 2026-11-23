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

use futures::TryStreamExt;
use rstest::*;
use sqlx::{AssertSqlSafe, Error, query_scalar, raw_sql};
use tests::fixtures::{Db, SerialGuard, database, serial};

#[rstest]
#[case::native_first(false, false)]
#[case::pgvector_first(true, false)]
#[case::relocated_pgvector(true, true)]
#[async_std::test]
async fn vector_extension_lifecycle(
    #[from(serial)] _serial: SerialGuard,
    database: Db,
    #[case] pgvector_first: bool,
    #[case] relocated: bool,
) -> Result<(), Error> {
    let mut conn = database.connection().await;
    let install_pgvector = if relocated {
        "CREATE SCHEMA IF NOT EXISTS vector_ext; CREATE EXTENSION vector WITH SCHEMA vector_ext;"
    } else {
        "CREATE EXTENSION vector;"
    };
    if pgvector_first {
        raw_sql(install_pgvector).execute(&mut conn).await?;
    }
    raw_sql(
        "CREATE EXTENSION pg_search;
         CREATE TABLE native_vectors (id int PRIMARY KEY, embedding pdb.vector(3));
         INSERT INTO native_vectors VALUES (1, '[1,0,0]'), (2, '[0,1,0]');
         CREATE INDEX native_vectors_idx ON native_vectors
             USING paradedb (id, embedding pdb.vector_cosine_ops);",
    )
    .execute(&mut conn)
    .await?;
    let native_search = "SELECT id FROM native_vectors WHERE id @@@ pdb.all()
                         ORDER BY embedding <=> '[1,0,0]' LIMIT 2";
    assert_eq!(
        query_scalar::<_, i32>(native_search)
            .fetch_all(&mut conn)
            .await?,
        vec![1, 2]
    );
    if !pgvector_first {
        assert!(
            !query_scalar::<_, bool>(
                "SELECT EXISTS (SELECT FROM pg_extension WHERE extname = 'vector')"
            )
            .fetch_one(&mut conn)
            .await?
        );
        raw_sql(install_pgvector).execute(&mut conn).await?;
    }
    let vector_schema = if relocated { "vector_ext" } else { "public" };
    let compatibility_sql = format!(
        "SELECT pdb.setup_pgvector();
         CREATE TABLE compatible_vectors (id int PRIMARY KEY, embedding {vector_schema}.vector(3));
         INSERT INTO compatible_vectors SELECT id, embedding FROM native_vectors;
         CREATE INDEX compatible_vectors_idx ON compatible_vectors
             USING paradedb (id, embedding {vector_schema}.vector_cosine_ops);"
    );
    raw_sql(AssertSqlSafe(compatibility_sql.as_str()))
        .execute(&mut conn)
        .await?;
    let plan = "EXPLAIN (COSTS OFF) SELECT id FROM compatible_vectors WHERE id @@@ pdb.all()
                ORDER BY embedding::pdb.vector <=> '[1,0,0]'::pdb.vector LIMIT 2";
    let plan: Vec<String> = query_scalar(plan).fetch_all(&mut conn).await?;
    assert!(plan.join("\n").contains("TopK Order By"), "{plan:?}");
    let binary_compatibility = format!(
        "SELECT paradedb.vector_send('[1.2345678,1e-40,3.4028235e38]'::pdb.vector)
            = {vector_schema}.vector_send('[1.2345678,1e-40,3.4028235e38]'::{vector_schema}.vector)"
    );
    assert!(
        query_scalar::<_, bool>(AssertSqlSafe(binary_compatibility.as_str()))
            .fetch_one(&mut conn)
            .await?
    );
    raw_sql(
        "CREATE TABLE binary_vectors (id int, embedding pdb.vector(3));
         CREATE TABLE wrong_dimensions (id int, embedding pdb.vector(2));",
    )
    .execute(&mut conn)
    .await?;
    let chunks: Vec<_> = conn
        .copy_out_raw("COPY compatible_vectors TO STDOUT (FORMAT BINARY)")
        .await?
        .try_collect()
        .await?;
    let mut copy = conn
        .copy_in_raw("COPY binary_vectors FROM STDIN (FORMAT BINARY)")
        .await?;
    for chunk in &chunks {
        copy.send(chunk.as_ref()).await?;
    }
    assert_eq!(copy.finish().await?, 2);
    let vectors: Vec<String> =
        query_scalar("SELECT embedding::text FROM binary_vectors ORDER BY id")
            .fetch_all(&mut conn)
            .await?;
    assert_eq!(vectors, vec!["[1,0,0]", "[0,1,0]"]);
    let mut copy = conn
        .copy_in_raw("COPY wrong_dimensions FROM STDIN (FORMAT BINARY)")
        .await?;
    for chunk in &chunks {
        copy.send(chunk.as_ref()).await?;
    }
    let error = copy.finish().await.unwrap_err();
    assert!(
        error.to_string().contains("expected 2 dimensions, not 3"),
        "{error}"
    );
    raw_sql("DROP TABLE compatible_vectors; DROP EXTENSION vector CASCADE;")
        .execute(&mut conn)
        .await?;
    assert_eq!(
        query_scalar::<_, i32>(native_search)
            .fetch_all(&mut conn)
            .await?,
        vec![1, 2]
    );
    raw_sql(install_pgvector).execute(&mut conn).await?;
    raw_sql("SELECT pdb.setup_pgvector();")
        .execute(&mut conn)
        .await?;
    let cast = format!("SELECT '[1,2,3]'::{vector_schema}.vector::pdb.vector::text");
    assert_eq!(
        query_scalar::<_, String>(AssertSqlSafe(cast.as_str()))
            .fetch_one(&mut conn)
            .await?,
        "[1,2,3]"
    );
    raw_sql("DROP EXTENSION pg_search CASCADE;")
        .execute(&mut conn)
        .await?;
    assert!(
        query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT FROM pg_extension WHERE extname = 'vector')"
        )
        .fetch_one(&mut conn)
        .await?
    );
    Ok(())
}
