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

use sqlx::{AssertSqlSafe, PgConnection};

/// The pid of the client backend whose `application_name` is `app_name` and whose
/// `pg_stat_activity` row also matches the SQL predicate `condition`, if one exists right now.
pub async fn client_backend_pid(
    conn: &mut PgConnection,
    app_name: &str,
    condition: &str,
) -> sqlx::Result<Option<i32>> {
    sqlx::query_scalar(AssertSqlSafe(format!(
        "SELECT pid FROM pg_stat_activity \
         WHERE application_name = $1 AND backend_type = 'client backend' AND ({condition})"
    )))
    .bind(app_name)
    .fetch_optional(&mut *conn)
    .await
}

/// One `pg_stat_activity` row: pid, backend type, database, application name, state,
/// wait event, start time, leader pid, query.
type ActivityRow = (
    i32,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
);

/// Every backend on the cluster with its wait state, for a failure message about a worker-slot
/// shortage that an earlier test left behind.
pub async fn activity_dump(conn: &mut PgConnection) -> sqlx::Result<String> {
    let limits: (String, String) = sqlx::query_as(
        "SELECT current_setting('max_worker_processes'), current_setting('max_parallel_workers')",
    )
    .fetch_one(&mut *conn)
    .await?;
    let rows: Vec<ActivityRow> = sqlx::query_as(
        "SELECT pid, backend_type, coalesce(datname, ''), coalesce(application_name, ''), \
         coalesce(state, ''), coalesce(wait_event_type || '/' || wait_event, ''), \
         to_char(backend_start, 'HH24:MI:SS'), coalesce(leader_pid::text, ''), \
         left(coalesce(query, ''), 120) \
         FROM pg_stat_activity ORDER BY backend_start",
    )
    .fetch_all(&mut *conn)
    .await?;
    let mut out = format!(
        "max_worker_processes={} max_parallel_workers={}\n",
        limits.0, limits.1
    );
    for (pid, kind, db, app, state, wait, started, leader, query) in rows {
        out.push_str(&format!(
            "  pid={pid} {kind} db={db} app={app} state={state} wait={wait} since={started} \
             leader={leader} query={query}\n"
        ));
    }
    Ok(out)
}
