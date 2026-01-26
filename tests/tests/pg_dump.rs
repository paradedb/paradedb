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

use anyhow::Result;
use cmd_lib::run_fun;
use fixtures::{conn, db::Query};
use rstest::*;
use sqlx::PgConnection;
use std::path::PathBuf;
use std::process::Command;
use tempfile::NamedTempFile;

/// Get the Postgres bin directory from PG_CONFIG
fn pg_bin_path() -> PathBuf {
    let pg_config_path =
        std::env::var("PG_CONFIG").expect("PG_CONFIG must be set to find pg_dump/pg_restore");
    match run_fun!($pg_config_path --bindir) {
        Ok(path) => PathBuf::from(path.trim()),
        Err(err) => panic!("could not run pg_config --bindir: {err}"),
    }
}

#[rstest]
fn test_pg_dump_restore(mut conn: PgConnection) -> Result<()> {
    // Query database for connection info (similar to how replication.rs gets username)
    let dbname = "SELECT current_database()"
        .fetch_one::<(String,)>(&mut conn)
        .0;
    let port = "SELECT setting FROM pg_settings WHERE name = 'port'"
        .fetch_one::<(String,)>(&mut conn)
        .0;
    let user = "SELECT current_user".fetch_one::<(String,)>(&mut conn).0;
    let host = "localhost".to_string();

    r#"
    CREATE TABLE lt
    (
        id              uuid    not null,
        organization_id uuid    not null,
        is_live       boolean not null,
        description     text,
        metadata   jsonb
    );
    "#
    .execute(&mut conn);

    r#"
    INSERT INTO lt (id, organization_id, is_live, description, metadata)
    VALUES
        ('550e8400-e29b-41d4-a716-446655440000'::uuid, '660e8400-e29b-41d4-a716-446655440001'::uuid, true, 'Payment for services', '{"amount": 1000, "currency": "USD"}'),
        ('550e8400-e29b-41d4-a716-446655440001'::uuid, '660e8400-e29b-41d4-a716-446655440001'::uuid, false, 'Refund processed', '{"amount": -500, "currency": "USD"}'),
        ('550e8400-e29b-41d4-a716-446655440002'::uuid, '660e8400-e29b-41d4-a716-446655440002'::uuid, true, 'Subscription renewal', '{"amount": 99, "currency": "USD", "plan": "premium"}');
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX lt_search_index_v3
        ON lt USING bm25 (
          id,
          organization_id,
          is_live,
          (description::pdb.ngram(3,5)),
          (metadata::pdb.literal),
          (metadata::pdb.unicode_words('alias=metadata_words'))
        )
        WITH (key_field=id);
    "#
    .execute(&mut conn);

    // Verify initial data and search functionality
    let initial_count: Vec<(i64,)> = "SELECT COUNT(*) FROM lt".fetch(&mut conn);
    assert_eq!(initial_count[0].0, 3);

    let search_results: Vec<(String,)> = r#"
        SELECT id::text FROM lt
        WHERE lt @@@ 'description:payment'
        ORDER BY id
    "#
    .fetch(&mut conn);
    assert_eq!(search_results.len(), 1);
    assert_eq!(search_results[0].0, "550e8400-e29b-41d4-a716-446655440000");

    // Create a temporary file for the dump
    let dump_file = NamedTempFile::new()?;
    let dump_path = dump_file.path().to_str().unwrap();

    // Build pg_dump command using the correct version from PG_CONFIG
    let pg_bin = pg_bin_path();
    let mut pg_dump_cmd = Command::new(pg_bin.join("pg_dump"));
    pg_dump_cmd
        .arg("-Fc") // Custom format
        .arg("--no-acl") // Skip ACLs
        .arg("--no-owner") // Skip ownership
        .arg("-h")
        .arg(&host)
        .arg("-p")
        .arg(&port)
        .arg("-U")
        .arg(&user)
        .arg("-t")
        .arg("lt") // Only dump this table
        .arg(&dbname);

    // Run pg_dump
    let pg_dump_output = pg_dump_cmd.output().expect("Failed to execute pg_dump");

    if !pg_dump_output.status.success() {
        let stderr = String::from_utf8_lossy(&pg_dump_output.stderr);
        panic!("pg_dump failed: {}", stderr);
    }

    // Write dump to file
    std::fs::write(dump_path, &pg_dump_output.stdout)?;

    // Drop the table and index
    "DROP TABLE IF EXISTS lt CASCADE".execute(&mut conn);

    // Verify table is dropped
    let table_exists: Vec<(bool,)> = r#"
        SELECT EXISTS (
            SELECT FROM information_schema.tables
            WHERE table_name = 'lt'
        )
    "#
    .fetch(&mut conn);
    assert!(!table_exists[0].0);

    // Build pg_restore command using the correct version from PG_CONFIG
    let mut pg_restore_cmd = Command::new(pg_bin.join("pg_restore"));
    pg_restore_cmd
        .arg("--verbose")
        .arg("--clean")
        .arg("--no-acl")
        .arg("--no-owner")
        .arg("-h")
        .arg(&host)
        .arg("-p")
        .arg(&port)
        .arg("-U")
        .arg(&user)
        .arg("-d")
        .arg(&dbname)
        .arg(dump_path);

    // Run pg_restore
    let pg_restore_output = pg_restore_cmd
        .output()
        .expect("Failed to execute pg_restore");

    if !pg_restore_output.status.success() {
        let stderr = String::from_utf8_lossy(&pg_restore_output.stderr);
        panic!("pg_restore failed: {}", stderr);
    }

    // Verify table is restored
    let table_exists: Vec<(bool,)> = r#"
        SELECT EXISTS (
            SELECT FROM information_schema.tables
            WHERE table_name = 'lt'
        )
    "#
    .fetch(&mut conn);
    assert!(table_exists[0].0);

    // Verify data is restored
    let restored_count: Vec<(i64,)> = "SELECT COUNT(*) FROM lt".fetch(&mut conn);
    assert_eq!(restored_count[0].0, 3);

    // Verify search functionality still works
    let search_results: Vec<(String,)> = r#"
        SELECT id::text FROM lt
        WHERE id @@@ pdb.all()
        ORDER BY id
    "#
    .fetch(&mut conn);
    assert_eq!(search_results.len(), 3);

    Ok(())
}
