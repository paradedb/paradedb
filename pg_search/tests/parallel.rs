// Copyright (c) 2023-2024 Retake, Inc.
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

mod fixtures;

use anyhow::Result;
use fixtures::*;
use futures::future::join_all;
use pretty_assertions::assert_eq;
use rand::Rng;
use rstest::*;

/// This test targets the locking functionality between Tantivy writers.
/// With no locking implemented, a high number of concurrent writers will
/// cause in an error when they all try to commit to the index at once.
#[rstest]
#[tokio::test]
async fn test_simultaneous_commits_with_bm25(database: Db) -> Result<()> {
    let mut conn1 = database.connection().await;

    // Create table once using any of the connections.
    "CREATE EXTENSION pg_search;

    CREATE TABLE concurrent_items (
      id SERIAL PRIMARY KEY,
      description TEXT,
      category VARCHAR(255),
      created_at TIMESTAMP DEFAULT now()
    );

    CALL paradedb.create_bm25(
        table_name => 'concurrent_items',
        index_name => 'concurrent_items_bm25',
        schema_name => 'public',
        key_field => 'id',
        text_fields => paradedb.field('description')
    );"
    .execute(&mut conn1);

    // Dynamically generate at least 100 rows for each connection
    let mut rng = rand::thread_rng();
    let categories = [
        "Category 1",
        "Category 2",
        "Category 3",
        "Category 4",
        "Category 5",
    ];

    for i in 0..5 {
        let random_category = categories[rng.gen_range(0..categories.len())];

        // Create new connections for this iteration and store them in a vector
        let mut connections = vec![];
        for _ in 0..50 {
            connections.push(database.connection().await);
        }

        let mut futures = vec![];
        for (n, mut conn) in connections.into_iter().enumerate() {
            let query = format!(
                "INSERT INTO concurrent_items (description, category)
                 VALUES ('Item {i} from conn{n}', '{random_category}')"
            );
            // Move the connection into the future, avoiding multiple borrows
            futures.push(async move { query.execute_async(&mut conn).await });
        }

        // Await all the futures for this iteration
        join_all(futures).await;
    }

    // Verify the number of rows in each database
    let rows1: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM concurrent_items")
        .fetch_one(&mut conn1)
        .await?;

    assert_eq!(rows1, 250);

    Ok(())
}