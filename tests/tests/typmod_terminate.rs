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

//! A die (`pg_terminate_backend`) acted on while `load_typmod` or `save_typmod` runs SPI exits
//! without unwinding. The typmod caches used to be locked across that SPI call, so the guard was
//! never dropped, and the Abort callback of an earlier cache miss then waited on it forever: the
//! backend stayed active instead of exiting. This test parks a backend inside the second typmod
//! lookup of a transaction, terminates it, and checks that it goes away.

use anyhow::Result;
use rstest::*;
use sqlx::{AssertSqlSafe, Executor, PgConnection};
use std::time::{Duration, Instant};
use tests::fixtures::*;
use tokio::time::sleep;

const SETUP_SQL: &str = r#"
CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;
CREATE TABLE typmod_die_a (id bigserial primary key, body text);
CREATE TABLE typmod_die_b (id bigserial primary key, body text);
CREATE INDEX typmod_die_a_idx ON typmod_die_a USING paradedb (id, (body::pdb.simple('alias=a')));
CREATE INDEX typmod_die_b_idx ON typmod_die_b USING paradedb (id, (body::pdb.simple('alias=b')));
INSERT INTO typmod_die_a (body) VALUES ('hello world');
INSERT INTO typmod_die_b (body) VALUES ('hello world');
-- Typmod lookups run SPI as the session's role, so row security applies to them. This policy
-- parks a lookup in `pg_sleep`, which acts on a die, while the session setting is on.
CREATE FUNCTION typmod_die_gate() RETURNS bool LANGUAGE plpgsql AS
$$ BEGIN
    IF current_setting('typmod_die.gate', true) = 'on' THEN PERFORM pg_sleep(60); END IF;
    RETURN true;
END $$;
ALTER TABLE paradedb._typmod_cache ENABLE ROW LEVEL SECURITY;
CREATE POLICY typmod_die_gate ON paradedb._typmod_cache FOR SELECT USING (typmod_die_gate());
"#;

struct Case {
    name: &'static str,
    /// The transaction's first typmod lookup: a cache miss that registers an Abort callback.
    first: &'static str,
    /// The second lookup, which the policy parks inside its SPI call.
    second: &'static str,
}

static CASES: [Case; 2] = [
    // Opening each index loads its field's typmod through `load_typmod`.
    Case {
        name: "load",
        first: "SELECT count(*) FROM typmod_die_a WHERE id @@@ paradedb.all()",
        second: "SELECT count(*) FROM typmod_die_b WHERE id @@@ paradedb.all()",
    },
    // Each cast to a typmod that isn't saved yet goes through `save_typmod`.
    Case {
        name: "save",
        first: "SELECT 'x'::pdb.simple('alias=unsaved_1')",
        second: "SELECT 'x'::pdb.simple('alias=unsaved_2')",
    },
];

const PARK_TIMEOUT: Duration = Duration::from_secs(30);
const EXIT_TIMEOUT: Duration = Duration::from_secs(10);

/// Runs the case's two lookups in one transaction on `target`. The second one only returns once
/// the backend is terminated, so its error is expected and ignored.
async fn run_target(mut target: PgConnection, app_name: String, case: &'static Case) -> Result<()> {
    target
        .execute(AssertSqlSafe(format!(
            "SET application_name = '{app_name}'"
        )))
        .await?;
    // Neither a superuser nor BYPASSRLS, so the policy applies to this session's lookups.
    target.execute("SET ROLE pg_read_all_data").await?;
    target.execute("BEGIN").await?;
    target.execute(case.first).await?;
    target.execute("SET LOCAL typmod_die.gate = 'on'").await?;
    let _ = target.execute(case.second).await;
    Ok(())
}

async fn parked_pid(conn: &mut PgConnection, app_name: &str) -> Result<Option<i32>> {
    Ok(sqlx::query_scalar(
        "SELECT pid FROM pg_stat_activity WHERE application_name = $1 AND wait_event = 'PgSleep'",
    )
    .bind(app_name)
    .fetch_optional(&mut *conn)
    .await?)
}

async fn is_running(conn: &mut PgConnection, pid: i32) -> Result<bool> {
    Ok(
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_stat_activity WHERE pid = $1)")
            .bind(pid)
            .fetch_one(&mut *conn)
            .await?,
    )
}

#[rstest]
#[tokio::test]
async fn terminate_during_typmod_lookup_exits(database: Db) -> Result<()> {
    let mut setup = database.connection().await;
    setup.execute(SETUP_SQL).await?;

    for case in &CASES {
        let app_name = format!("typmod_die_{}", case.name);
        let target = database.connection().await;
        let handle = tokio::spawn(run_target(target, app_name.clone(), case));

        let deadline = Instant::now() + PARK_TIMEOUT;
        let pid = loop {
            if let Some(pid) = parked_pid(&mut setup, &app_name).await? {
                break pid;
            }
            anyhow::ensure!(
                !handle.is_finished() && Instant::now() < deadline,
                "{}: the second typmod lookup never parked in its SPI call",
                case.name
            );
            sleep(Duration::from_millis(50)).await;
        };

        sqlx::query("SELECT pg_terminate_backend($1)")
            .bind(pid)
            .execute(&mut setup)
            .await?;

        let deadline = Instant::now() + EXIT_TIMEOUT;
        while is_running(&mut setup, pid).await? {
            anyhow::ensure!(
                Instant::now() < deadline,
                "{}: backend {pid} is still running {EXIT_TIMEOUT:?} after pg_terminate_backend",
                case.name
            );
            sleep(Duration::from_millis(100)).await;
        }
        handle.await??;
    }

    Ok(())
}
