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
