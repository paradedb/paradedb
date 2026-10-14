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

//! The SQL that `load_typmod` and `save_typmod` run through SPI can call back into the same
//! function, for example from a row security policy on `paradedb._typmod_cache`. Nothing may be
//! locked across that SPI call, or the inner call waits on the outer one forever, and the backend
//! ignores `pg_terminate_backend` while it waits.

use anyhow::Result;
use rstest::*;
use sqlx::{AssertSqlSafe, Executor, PgConnection};
use std::time::Duration;
use tests::fixtures::*;
use tokio::time::timeout;

const SETUP_SQL: &str = r#"
CREATE EXTENSION IF NOT EXISTS pg_search CASCADE;
SELECT 'x'::pdb.simple('alias=reentrant_seed');
-- While the session setting is on, the first row check calls back into the typmod functions:
-- `generic_typmod_out` goes through `load_typmod`, and the cast through `save_typmod`.
CREATE FUNCTION typmod_reentrant_gate() RETURNS bool LANGUAGE plpgsql AS
$$ BEGIN
    IF current_setting('typmod_reentrant.gate', true) = 'on' THEN
        PERFORM set_config('typmod_reentrant.gate', 'off', true);
        PERFORM paradedb.generic_typmod_out(id) FROM paradedb._typmod_cache
            WHERE typmod = ARRAY['alias=reentrant_seed'];
        PERFORM 'x'::pdb.simple('alias=reentrant_inner');
    END IF;
    RETURN true;
END $$;
ALTER TABLE paradedb._typmod_cache ENABLE ROW LEVEL SECURITY;
CREATE POLICY typmod_reentrant_gate ON paradedb._typmod_cache FOR SELECT
    USING (typmod_reentrant_gate());
"#;

const LOOKUP_TIMEOUT: Duration = Duration::from_secs(10);

/// Sets up the gate in `database`, then runs `sql` on a new connection with the gate on, as a
/// role that row security applies to. `sql` must not read `paradedb._typmod_cache` itself, so
/// that the gate first fires inside the typmod function's own SPI call.
async fn run_gated(database: &Db, sql: String) -> Result<()> {
    let mut conn = database.connection().await;
    // Neither a superuser nor BYPASSRLS, so the policy applies to this session's lookups.
    conn.execute("SET ROLE pg_read_all_data").await?;
    conn.execute("BEGIN").await?;
    conn.execute("SET LOCAL typmod_reentrant.gate = 'on'")
        .await?;
    timeout(LOOKUP_TIMEOUT, conn.execute(AssertSqlSafe(sql)))
        .await
        .map_err(|_| anyhow::anyhow!("the lookup did not finish within {LOOKUP_TIMEOUT:?}"))??;
    conn.execute("COMMIT").await?;
    Ok(())
}

async fn setup(database: &Db) -> Result<PgConnection> {
    let mut conn = database.connection().await;
    conn.execute(SETUP_SQL).await?;
    Ok(conn)
}

#[rstest]
#[tokio::test]
async fn load_typmod_reentered_through_spi(database: Db) -> Result<()> {
    let mut conn = setup(&database).await?;
    let seed_id: i32 = sqlx::query_scalar(
        "SELECT id FROM paradedb._typmod_cache WHERE typmod = ARRAY['alias=reentrant_seed']",
    )
    .fetch_one(&mut conn)
    .await?;
    run_gated(
        &database,
        format!("SELECT paradedb.generic_typmod_out({seed_id})"),
    )
    .await
}

#[rstest]
#[tokio::test]
async fn save_typmod_reentered_through_spi(database: Db) -> Result<()> {
    setup(&database).await?;
    run_gated(
        &database,
        "SELECT 'x'::pdb.simple('alias=reentrant_outer')".into(),
    )
    .await
}
