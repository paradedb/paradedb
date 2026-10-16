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
    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(
        stdout,
        "DROP FUNCTION IF EXISTS f();\nCREATE TABLE added(x int);\n"
    );
    assert_eq!(stderr.matches("automatically").count(), 2);
    assert!(stderr.contains("pdb.source_code"));
    assert!(stderr.contains("STORAGE = plain"));
    assert!(stderr.contains("STORAGE = extended"));
    assert!(stderr.contains("unsupported"));
    assert!(!stderr.contains("SchemaBot: SchemaBot"));
}

#[test]
fn manual_type_changes_require_a_fragment_targeting_the_same_object() {
    let dir = std::env::temp_dir().join(format!("schemabot-manual-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_schemabot"))
            .current_dir(&dir)
            .args(args)
            .output()
            .unwrap()
    };
    for (before, after, alter) in [
        (
            "CREATE TYPE pdb.source_code (INPUT = pdb.decode, OUTPUT = pdb.encode, STORAGE = plain)",
            "CREATE TYPE pdb.source_code (INPUT = pdb.decode, OUTPUT = pdb.encode, STORAGE = extended)",
            "ALTER TYPE pdb.source_code SET (STORAGE = extended)",
        ),
        (
            "CREATE TYPE pdb.status AS ENUM ('open')",
            "CREATE TYPE pdb.status AS ENUM ('open', 'closed')",
            "ALTER TYPE pdb.status ADD VALUE 'closed'",
        ),
    ] {
        std::fs::write(dir.join("base.sql"), before).unwrap();
        std::fs::write(dir.join("head.sql"), after).unwrap();
        let diff = run(&["diff", "--manual", "manual.sql", "base.sql", "head.sql"]);
        assert_eq!(diff.status.code(), Some(2));
        assert!(diff.stdout.is_empty());
        std::fs::write(dir.join("diff.sql"), diff.stdout).unwrap();
        let check_args = [
            "check",
            "--manual",
            "manual.sql",
            "diff.sql",
            "fragment.sql",
        ];
        for fragment in ["SELECT 1", "ALTER TYPE other.status ADD VALUE 'closed'"] {
            std::fs::write(dir.join("fragment.sql"), fragment).unwrap();
            assert_eq!(run(&check_args).status.code(), Some(1));
        }
        std::fs::write(dir.join("fragment.sql"), alter).unwrap();
        assert!(run(&check_args).status.success());
        let drop = if before.contains("source_code") {
            "DROP TYPE pdb.source_code"
        } else {
            "DROP TYPE pdb.status"
        };
        std::fs::write(dir.join("fragment.sql"), drop).unwrap();
        assert!(run(&check_args).status.success());
    }
    // Manual coverage must not bypass the automatically generated requirements.
    std::fs::write(dir.join("diff.sql"), "CREATE TABLE required(x int)").unwrap();
    assert_eq!(
        run(&[
            "check",
            "--manual",
            "manual.sql",
            "diff.sql",
            "fragment.sql"
        ])
        .status
        .code(),
        Some(1)
    );
    // Hard failures are distinct from manual work and emit no suggestion.
    std::fs::write(dir.join("head.sql"), "CREATE TABLE (").unwrap();
    let failure = run(&["diff", "base.sql", "head.sql"]);
    assert_eq!(failure.status.code(), Some(1));
    assert!(failure.stdout.is_empty());
    assert!(
        String::from_utf8(failure.stderr)
            .unwrap()
            .contains("SQL parse error")
    );
    assert_eq!(
        run(&["diff", "missing.sql", "head.sql"]).status.code(),
        Some(1)
    );
    std::fs::remove_dir_all(dir).unwrap();
}
