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
mod fixtures;

use fixtures::db::Query;
use fixtures::*;
use rstest::*;
use sqlx::{PgConnection, Row};

#[rstest]
async fn fieldnorms_disabled(mut conn: PgConnection) {
    r#"
    CREATE TABLE fieldnorms_test (
        id INT,
        content pdb.simple('fieldnorms=false')
    );

    -- Explicitly cast strings to ::pdb.simple
    INSERT INTO fieldnorms_test VALUES
    (1, 'this is a test'::pdb.simple),
    (2, ('this is a test ' || repeat('word ', 500))::pdb.simple);

    CREATE INDEX fieldnorms_test_idx
    ON fieldnorms_test
    USING bm25(content)
    WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    // Fetch the scores for both documents.
    // We expect 2 rows. Since fieldnorms are disabled and both docs contain "test" exactly once,
    // the BM25 score must be identical regardless of document length.
    let scores: Vec<f32> = sqlx::query(
        "SELECT paradedb.score(id) FROM fieldnorms_test WHERE content @@@ 'test' ORDER BY id",
    )
    .fetch_all(&mut conn)
    .await
    .expect("Failed to fetch scores")
    .into_iter()
    .map(|row| row.get::<f32, _>(0))
    .collect();

    assert_eq!(scores.len(), 2, "Should find both documents");

    let score_short = scores[0];
    let score_long = scores[1];

    assert_eq!(
        score_short, score_long,
        "Scores should be identical when fieldnorms=false. Short: {}, Long: {}",
        score_short, score_long
    );
}
