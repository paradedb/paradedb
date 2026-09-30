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

//! Regression test for #6489. If a typmod cache stays locked across SPI, a
//! `pg_terminate_backend` handled inside that SPI call exits without unwinding, and the Abort
//! callback of an earlier cache miss blocks on the lock forever, so the backend never exits.
//! This test parks a backend inside the second typmod lookup of a transaction, terminates it,
//! and checks that it goes away.

use anyhow::Result;
use rstest::*;
use sqlx::{Executor, PgConnection};
use std::time::{Duration, Instant};
use tests::fixtures::*;
use tokio::time::sleep;

const SETUP_SQL: &str = r#"
CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;
CREATE TABLE typmod_die_a (id bigserial primary key, body text);
CREATE TABLE typmod_die_b (id bigserial primary key, body text);
CREATE INDEX typmod_die_a_idx ON typmod_die_a USING paradedb (id, (body::pdb.simple('alias=a'))) WITH (key_field = 'id');
CREATE INDEX typmod_die_b_idx ON typmod_die_b USING paradedb (id, (body::pdb.simple('alias=b'))) WITH (key_field = 'id');
INSERT INTO typmod_die_a (body) VALUES ('hello world');
INSERT INTO typmod_die_b (body) VALUES ('hello world');
-- Typmod lookups run SPI as the session's role, so row security applies to them. While the
-- session setting is on, this policy parks a lookup in `pg_sleep`, which checks for interrupts,
-- so the terminate is handled inside that lookup's SPI call.
CREATE FUNCTION typmod_die_gate() RETURNS bool LANGUAGE plpgsql AS
$$ BEGIN
    IF current_setting('typmod_die.gate', true) = 'on' THEN PERFORM pg_sleep(60); END IF;
    RETURN true;
END $$;
ALTER TABLE paradedb._typmod_cache ENABLE ROW LEVEL SECURITY;
CREATE POLICY typmod_die_gate ON paradedb._typmod_cache FOR SELECT USING (typmod_die_gate());
"#;

const PARK_TIMEOUT: Duration = Duration::from_secs(30);
const EXIT_TIMEOUT: Duration = Duration::from_secs(10);

/// Runs `first` and `second` in one transaction on `target`. The second one only returns once
/// the backend is terminated, so its error is expected and ignored.
async fn run_target(
    mut target: PgConnection,
    app_name: &'static str,
    first: &'static str,
    second: &'static str,
) -> Result<()> {
    target
        .execute(format!("SET application_name = '{app_name}'").as_str())
        .await?;
    // Neither a superuser nor BYPASSRLS, so the policy applies to this session's lookups.
    target.execute("SET ROLE pg_read_all_data").await?;
    target.execute("BEGIN").await?;
    target.execute(first).await?;
    target.execute("SET LOCAL typmod_die.gate = 'on'").await?;
    let _ = target.execute(second).await;
    Ok(())
}

async fn is_running(conn: &mut PgConnection, pid: i32) -> Result<bool> {
    Ok(
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_stat_activity WHERE pid = $1)")
            .bind(pid)
            .fetch_one(&mut *conn)
            .await?,
    )
}

/// `first` is a cache miss that registers an Abort callback; the policy parks `second` inside
/// its SPI call, where the backend is terminated.
#[rstest]
// Opening each index loads its field's typmod through `load_typmod`.
#[case::load(
    "typmod_die_load",
    "SELECT count(*) FROM typmod_die_a WHERE id @@@ pdb.all()",
    "SELECT count(*) FROM typmod_die_b WHERE id @@@ pdb.all()"
)]
// Each cast to a typmod that isn't saved yet goes through `save_typmod`.
#[case::save(
    "typmod_die_save",
    "SELECT 'x'::pdb.simple('alias=unsaved_1')",
    "SELECT 'x'::pdb.simple('alias=unsaved_2')"
)]
#[tokio::test]
async fn terminate_during_typmod_lookup_exits(
    database: Db,
    #[case] app_name: &'static str,
    #[case] first: &'static str,
    #[case] second: &'static str,
) -> Result<()> {
    let mut setup = database.connection().await;
    setup.execute(SETUP_SQL).await?;

    let target = database.connection().await;
    let handle = tokio::spawn(run_target(target, app_name, first, second));

    let deadline = Instant::now() + PARK_TIMEOUT;
    let pid = loop {
        if let Some(pid) =
            client_backend_pid(&mut setup, app_name, "wait_event = 'PgSleep'").await?
        {
            break pid;
        }
        if handle.is_finished() {
            handle.await??;
            anyhow::bail!("the second typmod lookup returned without parking in its SPI call");
        }
        anyhow::ensure!(
            Instant::now() < deadline,
            "the second typmod lookup never parked in its SPI call"
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
            "backend {pid} is still running {EXIT_TIMEOUT:?} after pg_terminate_backend"
        );
        sleep(Duration::from_millis(100)).await;
    }
    handle.await??;
    Ok(())
}
