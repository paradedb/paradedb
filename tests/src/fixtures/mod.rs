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

#![allow(dead_code)]
#![allow(unused_imports)]

pub mod db;
pub mod querygen;
pub mod tables;
pub mod utils;

use async_std::task::block_on;
use rstest::*;
use sqlx::{self, PgConnection};

pub use crate::fixtures::db::*;
pub use crate::fixtures::tables::*;

#[fixture]
pub fn database() -> Db {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
        .try_init();
    block_on(async { Db::new().await })
}

/// Render a database error as just the message the server sent.
///
/// `sqlx::Error`'s `Display` is not a stable thing to assert on: from 0.9 it appends the
/// Postgres source line that raised the error, so `err.to_string()` reads
/// `...does not exist at line 248`. That line number is a position in *our* source and
/// moves whenever the extension's code moves, which would make every assertion on an
/// error message a tripwire. Assert on this instead.
///
/// On sqlx 0.8 this produces exactly what `Display` already produces; it exists here so
/// tests written against `main` port back unchanged.
pub fn db_error_message(err: &sqlx::Error) -> String {
    match err.as_database_error() {
        Some(db_err) => format!("error returned from database: {}", db_err.message()),
        None => err.to_string(),
    }
}

pub fn pg_major_version(conn: &mut PgConnection) -> usize {
    r#"select (regexp_match(version(), 'PostgreSQL (\d+)'))[1]::int;"#
        .fetch_one::<(i32,)>(conn)
        .0 as usize
}

#[fixture]
pub fn conn(database: Db) -> PgConnection {
    block_on(async {
        let mut conn = database.connection().await;

        // pg_search declares `requires = 'vector'`, so CASCADE pulls pgvector
        // in automatically (its extension script references the `vector` type).
        sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;")
            .execute(&mut conn)
            .await
            .expect("could not create extension pg_search");

        sqlx::query("SET log_error_verbosity TO VERBOSE;")
            .execute(&mut conn)
            .await
            .expect("could not adjust log_error_verbosity");

        // Setting to 1 provides test coverage for both mutable and immutable segments, because
        // bulk insert statements will create both a mutable and immutable segment.
        sqlx::query("SET paradedb.global_mutable_segment_rows TO 1;")
            .execute(&mut conn)
            .await
            .expect("could not adjust mutable_segment_rows");

        sqlx::query("SET log_min_duration_statement TO 1000;")
            .execute(&mut conn)
            .await
            .expect("could not set long-running-statement logging");

        conn
    })
}
