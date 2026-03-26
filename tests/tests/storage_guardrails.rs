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

use std::fs;
use std::path::{Path, PathBuf};

fn collect_rs_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("source directory should be readable") {
        let entry = entry.expect("directory entry should be readable");
        let path = entry.path();

        if path.is_dir() {
            collect_rs_files(&path, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

#[test]
fn raw_storage_primitives_stay_inside_wrappers() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root should exist")
        .to_path_buf();
    let pg_search_src = workspace_root.join("pg_search").join("src");

    let allowed_files = [
        workspace_root.join("pg_search/src/postgres/storage/buffer.rs"),
        workspace_root.join("pg_search/src/postgres/storage/utils.rs"),
    ];
    let forbidden_patterns = [
        (
            "RelationBufferAccess::open(",
            "raw buffer acquisition must stay in the buffer wrapper layer",
        ),
        (
            "BufferGetPage(",
            "raw page access must stay in the buffer wrapper layer",
        ),
        (
            "PageGetContents(",
            "raw page content access must stay in the page wrapper layer",
        ),
        (
            "PageGetSpecialPointer(",
            "raw special-area access must stay in the page wrapper layer",
        ),
        (
            "MarkBufferDirty(",
            "dirty-marking must stay centralized with WAL handling",
        ),
        (
            "GenericXLogStart(",
            "generic WAL entry points must stay centralized",
        ),
        (
            "GenericXLogRegisterBuffer(",
            "generic WAL entry points must stay centralized",
        ),
        (
            "GenericXLogFinish(",
            "generic WAL entry points must stay centralized",
        ),
        (
            "GenericXLogAbort(",
            "generic WAL entry points must stay centralized",
        ),
        (
            "PageInit(",
            "page initialization must stay centralized with WAL handling",
        ),
        (
            "PageAddItemExtended(",
            "page tuple mutation must stay in PageMut",
        ),
        (
            "PageIndexTupleOverwrite(",
            "page tuple mutation must stay in PageMut",
        ),
        (
            "PageIndexMultiDelete(",
            "page tuple mutation must stay in PageMut",
        ),
        (
            "PageIndexTupleDelete(",
            "page tuple mutation must stay in PageMut",
        ),
    ];

    let mut source_files = Vec::new();
    collect_rs_files(&pg_search_src, &mut source_files);
    source_files.sort();

    let mut violations = Vec::new();
    for source_file in source_files {
        if allowed_files.contains(&source_file) {
            continue;
        }

        let relative_path = source_file
            .strip_prefix(&workspace_root)
            .expect("source file should be inside workspace");
        let contents = fs::read_to_string(&source_file)
            .unwrap_or_else(|err| panic!("failed reading {}: {err}", source_file.display()));

        for (line_number, line) in contents.lines().enumerate() {
            for (needle, reason) in forbidden_patterns {
                if line.contains(needle) {
                    violations.push(format!(
                        "{}:{} contains `{needle}`: {reason}",
                        relative_path.display(),
                        line_number + 1,
                    ));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "raw storage primitives escaped the wrapper layer:\n{}",
        violations.join("\n")
    );
}
