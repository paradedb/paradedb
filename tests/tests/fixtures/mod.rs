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
pub mod props;
pub mod querygen;
pub mod tables;
pub mod utils;

use async_std::task::block_on;
use proptest::prelude::*;
use proptest::strategy::LazyJust;
use rstest::*;
use sqlx::{self, PgConnection};

use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};

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
    initialize_conn(&database)
}

fn initialize_conn(database: &Db) -> PgConnection {
    block_on(async {
        let mut conn = database.connection().await;

        // You can hijack a test run to debug, like so:
        // let mut conn = <PgConnection as sqlx::Connection>::connect(
        //     "postgres://neilhansen@localhost:5432/postgres",
        // )
        // .await
        // .unwrap();

        sqlx::query("CREATE EXTENSION pg_search;")
            .execute(&mut conn)
            .await
            .expect("could not create extension pg_search");

        sqlx::query("SET log_error_verbosity TO VERBOSE;")
            .execute(&mut conn)
            .await
            .expect("could not adjust log_error_verbosity");

        conn
    })
}

// Holds a Db and a PgConnection to that Db, so that the former is not dropped before the latter is
// closed.
pub struct DbConnection {
    db: Db,
    conn: PgConnection,
}

impl Debug for DbConnection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DbConnection")
            .field("db", &self.db)
            .finish_non_exhaustive()
    }
}

impl Deref for DbConnection {
    type Target = PgConnection;

    fn deref(&self) -> &Self::Target {
        &self.conn
    }
}

impl DerefMut for DbConnection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.conn
    }
}

pub fn arb_conn() -> impl Strategy<Value = DbConnection> {
    LazyJust::new(|| {
        let db = block_on(async { Db::new().await });
        let conn = initialize_conn(&db);
        DbConnection { db, conn }
    })
}
