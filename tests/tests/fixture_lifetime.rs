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

use async_std::task::block_on;
use rstest::rstest;
use sqlx::{Connection, PgConnection};
use std::panic::{panic_any, resume_unwind};
use std::thread;
use tests::fixtures::*;

struct DatabaseName(String);

fn database_exists(name: &str) -> bool {
    let url = dotenvy::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let mut conn = block_on(PgConnection::connect(&url)).expect("connect to DATABASE_URL");
    block_on(
        sqlx::query_scalar("SELECT EXISTS (SELECT FROM pg_database WHERE datname = $1)")
            .bind(name)
            .fetch_one(&mut conn),
    )
    .expect("query pg_database")
}

/// Run the body on its own thread: its database must outlive the body and be dropped
/// when the thread exits, whether the body returns or panics.
#[rstest]
#[case::returns(false)]
#[case::panics(true)]
fn conn_database_is_dropped_after_the_test(#[case] panics: bool) {
    let test = thread::spawn(move || {
        let mut conn = conn::default();
        let (name,) = "SELECT current_database()::text".fetch_one::<(String,)>(&mut conn);
        // Outlast the 5 second wait of an early DROP DATABASE, so that it fails.
        "SELECT pg_sleep(8)".execute(&mut conn);
        assert!(database_exists(&name));
        if panics {
            panic_any(DatabaseName(name));
        }
        name
    });
    let name = match test.join() {
        Ok(name) => name,
        Err(payload) => match payload.downcast::<DatabaseName>() {
            Ok(name) => name.0,
            Err(other) => resume_unwind(other),
        },
    };
    assert!(
        !database_exists(&name),
        "{name} still exists after its test thread exited"
    );
}
