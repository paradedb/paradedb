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

pub mod score;
pub mod snippet;
pub mod window_agg;

/// `pdb.score` / `pdb.snippet*` are placeholders rewritten by a ParadeDB custom
/// scan. Reaching here means the scan did not take over the query — most often
/// because no ParadeDB operator is in `WHERE`.
pub(crate) fn placeholder_not_rewritten() -> ! {
    pgrx::error!(
        "pdb.score() / pdb.snippet() require a ParadeDB search operator (@@@, |||, &&&, ===, or ###) in the WHERE clause. \
         If this query already has one, please report it at https://github.com/paradedb/paradedb/issues/new/choose"
    )
}
