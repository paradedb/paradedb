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

mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn fresh_install_exposes_pdb_admin_functions(mut conn: PgConnection) {
    let functions: Vec<(String,)> = r#"
        SELECT p.proname
        FROM pg_proc p
        JOIN pg_namespace n ON n.oid = p.pronamespace
        WHERE n.nspname = 'pdb'
          AND p.proname IN ('indexes', 'index_segments', 'verify_all_indexes', 'verify_index')
        ORDER BY p.proname
    "#
    .fetch_collect(&mut conn);

    let function_names = functions
        .into_iter()
        .map(|(name,)| name)
        .collect::<Vec<_>>();

    assert_eq!(
        function_names,
        vec![
            "index_segments".to_string(),
            "indexes".to_string(),
            "verify_all_indexes".to_string(),
            "verify_index".to_string(),
        ]
    );
}
