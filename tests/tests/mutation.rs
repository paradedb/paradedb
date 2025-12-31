// Copyright (C) 2023-2026 ParadeDB, Inc.
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

use fixtures::*;
use futures::executor::block_on;
use lockfree_object_pool::MutexObjectPool;
use proptest::prelude::*;
use proptest_derive::Arbitrary;
use rstest::*;
use sqlx::PgConnection;
use std::fmt::Debug;

#[derive(Debug, Clone, Copy, Arbitrary)]
enum Message {
    Match,
    NoMatch,
}

impl Message {
    fn as_str(&self) -> &'static str {
        match self {
            Message::Match => "cheese",
            Message::NoMatch => "bread",
        }
    }
}

#[derive(Debug, Clone, Arbitrary)]
enum Action {
    Insert(Message),
    Update {
        #[proptest(strategy = "any::<prop::sample::Index>()")]
        index: prop::sample::Index,
        new_message: Message,
    },
    Delete(#[proptest(strategy = "any::<prop::sample::Index>()")] prop::sample::Index),
    Vacuum,
}

fn setup(conn: &mut PgConnection, mutable_segment_rows: usize) {
    format!(r#"
    CREATE EXTENSION IF NOT EXISTS pg_search;
    SET log_error_verbosity TO VERBOSE;
    SET paradedb.global_mutable_segment_rows TO 0;
    DROP TABLE IF EXISTS test_table;
    CREATE TABLE test_table (id SERIAL8 PRIMARY KEY, message TEXT);
    CREATE INDEX idx_test_table ON test_table USING bm25 (id, message)
    WITH (key_field = 'id', text_fields='{{"message": {{ "tokenizer": {{"type": "default"}} }} }}', mutable_segment_rows={mutable_segment_rows});
    ANALYZE test_table;
    "#)
    .execute(conn);
}

#[rstest]
#[tokio::test]
async fn mutable_segment_correctness(database: Db) {
    let pool = MutexObjectPool::<PgConnection>::new(
        move || block_on(async { database.connection().await }),
        |_| {},
    );

    proptest!(|(
        actions in proptest::collection::vec(any::<Action>(), 1..32),
        mutable_segment_rows in prop_oneof![Just(0), Just(1), Just(10)],
    )| {
        let mut conn = pool.pull();
        setup(&mut conn, mutable_segment_rows);

        let mut model: Vec<(i64, Message)> = Vec::new();

        for (i, action) in actions.into_iter().enumerate() {
            match action {
                Action::Insert(message) => {
                    let (id,): (i64,) = format!(
                        "INSERT INTO test_table (message) VALUES ('{}') RETURNING id",
                        message.as_str()
                    ).fetch_one(&mut conn);
                    model.push((id, message));
                }
                Action::Update { index, new_message } => {
                    if model.is_empty() {
                        continue;
                    }
                    let idx = index.index(model.len());
                    let (id_to_update, _) = model[idx];

                    format!(
                        "UPDATE test_table SET message = '{}' WHERE id = {};",
                        new_message.as_str(),
                        id_to_update,
                    ).execute(&mut conn);

                    model[idx].1 = new_message;
                }
                Action::Delete(index) => {
                    if model.is_empty() {
                        continue;
                    }
                    let idx = index.index(model.len());
                    let (id_to_delete, _) = model[idx];

                    format!(
                        "DELETE FROM test_table WHERE id = {};",
                        id_to_delete,
                    ).execute(&mut conn);

                    model.remove(idx);
                }
                Action::Vacuum => {
                    "VACUUM test_table;".execute(&mut conn);
                }
            }

            let count_query = r#"SELECT COUNT(*) FROM test_table WHERE message @@@ 'cheese';"#;
            let (result_count,): (i64,) = count_query.fetch_one(&mut conn);

            let expected_count = model.iter().filter(|(_, m)| matches!(m, Message::Match)).count() as i64;

            prop_assert_eq!(
                result_count,
                expected_count,
                "Mismatch after action #{}: {:?}\nModel: {:?}",
                i,
                action,
                model
            );
        }
    });
}
