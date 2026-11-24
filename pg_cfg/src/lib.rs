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

/// Sets `pgNN_plus` for every major up to the one the crate's `pgNN` feature selects, so a
/// gate can say `#[cfg(pg18_plus)]` instead of listing the older versions.
///
/// A major this list doesn't know yet gets every `pgNN_plus`: a new version takes the newest
/// paths and fails to build only where its API changed.
pub fn emit() {
    const MAJORS: [u32; 5] = [15, 16, 17, 18, 19];
    let selected = MAJORS
        .iter()
        .copied()
        .filter(|major| std::env::var_os(format!("CARGO_FEATURE_PG{major}")).is_some())
        .max();

    println!("cargo::rerun-if-changed=build.rs");
    for major in &MAJORS[1..] {
        println!("cargo::rustc-check-cfg=cfg(pg{major}_plus)");
        if selected.is_none_or(|selected| selected >= *major) {
            println!("cargo::rustc-cfg=pg{major}_plus");
        }
    }
}
