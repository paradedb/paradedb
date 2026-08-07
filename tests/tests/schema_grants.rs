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

//! Tests for the privileges the extension grants PUBLIC on its own schemas.
//!
//! The upgrade job's schema-parity check diffs `pg_depend` object lists, which
//! says nothing about ACLs, so assert the grants themselves here.

use rstest::*;
use sqlx::PgConnection;
use tests::fixtures::*;

#[rstest]
fn public_gets_usage_but_not_create_on_extension_schemas(mut conn: PgConnection) {
    // Roles are cluster-wide and the tests share a cluster, so guard the create
    // rather than assuming a previous run cleaned up after itself.
    r#"
    DO $$
    BEGIN
        IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'schema_grants_lowpriv') THEN
            CREATE ROLE schema_grants_lowpriv;
        END IF;
    END
    $$;
    "#
    .execute(&mut conn);

    for schema in ["paradedb", "pdb"] {
        // The role holds no grants of its own, so this reflects what PUBLIC has.
        let (usage, create) = format!(
            "SELECT has_schema_privilege('schema_grants_lowpriv', '{schema}', 'USAGE'),
                    has_schema_privilege('schema_grants_lowpriv', '{schema}', 'CREATE')"
        )
        .fetch_one::<(bool, bool)>(&mut conn);

        assert!(
            usage,
            "PUBLIC needs USAGE on {schema} to reach the extension's objects"
        );
        assert!(!create, "PUBLIC must not hold CREATE on {schema}");
    }

    // ...and prove it end to end, not just in the catalog.
    "SET ROLE schema_grants_lowpriv".execute(&mut conn);
    match "CREATE TABLE pdb.squat(id int)".execute_result(&mut conn) {
        Ok(_) => panic!("an ordinary role should not be able to create objects in pdb"),
        Err(err) => assert_eq!(
            db_error_message(&err),
            "error returned from database: permission denied for schema pdb"
        ),
    }
    "RESET ROLE".execute(&mut conn);

    "DROP ROLE schema_grants_lowpriv".execute(&mut conn);
}
