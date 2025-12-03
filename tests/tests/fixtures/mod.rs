// Copyright (c) 2023-2025 ParadeDB, Inc.
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
    block_on(async { Db::new().await })
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

        sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_search;")
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
