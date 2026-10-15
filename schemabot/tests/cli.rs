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

use std::process::Command;

#[test]
fn unsupported_removals_preserve_stdout_and_fail_with_diagnostics() {
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let output = Command::new(env!("CARGO_BIN_EXE_schemabot"))
        .arg("diff")
        .arg(fixtures.join("manual-base.sql"))
        .arg(fixtures.join("manual-head.sql"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stdout.contains("DROP FUNCTION IF EXISTS f()"));
    assert!(stdout.contains("CREATE TABLE added"));
    assert!(!stdout.contains("CREATE TYPE"));
    assert!(stderr.contains("pdb.source_code"));
    assert!(stderr.contains("unsupported"));
    assert!(!stderr.contains("SchemaBot: SchemaBot"));
}
