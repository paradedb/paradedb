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

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context};
use lindera::dictionary::Dictionary;
use once_cell::sync::OnceCell;

pub const DICTIONARY_ROOT_ENV: &str = "PARADEDB_LINDERA_DICT_ROOT";

pub const DICTIONARY_NAMES: &[&str] = &["cc-cedict", "ipadic", "ko-dic"];

// Must stay in sync with lindera-dict-builder's component list and with
// lindera-dictionary's mmap component layout for the pinned Lindera version.
pub const DICTIONARY_COMPONENT_FILES: &[&str] = &[
    "metadata.json",
    "char_def.bin",
    "unk.bin",
    "matrix.mtx",
    "dict.da",
    "dict.vals",
    "dict.wordsidx",
    "dict.words",
];

static DICTIONARY_ROOT: OnceCell<PathBuf> = OnceCell::new();

pub fn set_dictionary_root(path: impl Into<PathBuf>) {
    let path = path.into();
    if DICTIONARY_ROOT.get().is_some() {
        return;
    }

    let _ = DICTIONARY_ROOT.set(path);
}

pub fn dictionary_root() -> anyhow::Result<PathBuf> {
    if let Some(root) = DICTIONARY_ROOT.get() {
        return Ok(root.clone());
    }

    if let Some(root) = std::env::var_os(DICTIONARY_ROOT_ENV) {
        return Ok(root.into());
    }

    Err(anyhow!(
        "pg_search did not configure a Lindera dictionary root and {DICTIONARY_ROOT_ENV} is not set"
    ))
}

pub fn dictionary_dir_ready(path: &Path) -> bool {
    path.is_dir()
        && DICTIONARY_COMPONENT_FILES
            .iter()
            .all(|component| path.join(component).is_file())
}

pub fn installed_dictionaries_ready() -> bool {
    let Ok(root) = dictionary_root() else {
        return false;
    };

    DICTIONARY_NAMES
        .iter()
        .all(|name| dictionary_dir_ready(&root.join(name)))
}

pub fn load_dictionary(name: &str) -> anyhow::Result<Dictionary> {
    let root = dictionary_root()?;
    let dictionary_dir = root.join(name);

    if !dictionary_dir_ready(&dictionary_dir) {
        let missing = DICTIONARY_COMPONENT_FILES
            .iter()
            .filter(|component| !dictionary_dir.join(component).is_file())
            .copied()
            .collect::<Vec<_>>();

        if missing.is_empty() {
            bail!(
                "Lindera dictionary directory does not exist or is not a directory: {}",
                dictionary_dir.display()
            );
        }

        bail!(
            "Lindera dictionary `{name}` is missing component files under {}: {}",
            dictionary_dir.display(),
            missing.join(", ")
        );
    }

    Dictionary::load_from_path_with_options(&dictionary_dir, true).with_context(|| {
        format!(
            "failed to mmap-load Lindera dictionary `{name}` from {}",
            dictionary_dir.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_TEST_DIR: AtomicUsize = AtomicUsize::new(0);

    fn temp_dir() -> PathBuf {
        let id = NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "paradedb-lindera-mmap-test-{}-{id}",
            std::process::id()
        ))
    }

    #[test]
    fn dictionary_dir_ready_requires_all_components() {
        let dir = temp_dir();
        fs::create_dir_all(&dir).unwrap();

        for component in DICTIONARY_COMPONENT_FILES {
            assert!(!dictionary_dir_ready(&dir));
            fs::write(dir.join(component), []).unwrap();
        }

        assert!(dictionary_dir_ready(&dir));

        fs::remove_file(dir.join("unk.bin")).unwrap();
        assert!(!dictionary_dir_ready(&dir));

        fs::remove_dir_all(&dir).unwrap();
    }
}
