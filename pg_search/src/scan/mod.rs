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

pub mod batch_scanner;
pub mod codec;
pub mod execution_plan;
pub mod filter_pushdown;
pub mod info;
pub mod partitioning;
pub mod pre_filter;
pub mod search_predicate_udf;
pub mod table_provider;
#[cfg(any(test, feature = "pg_test"))]
mod tests;

pub use batch_scanner::Scanner;
pub use codec::PgSearchExtensionCodec;
pub use info::ScanInfo;
pub use search_predicate_udf::SearchPredicateUDF;
pub use table_provider::PgSearchTableProvider;

/// A trait for checking visibility of rows.
pub trait VisibilityChecker {
    /// Checks if a row is visible.
    ///
    /// Returns `Some(ctid)` if the row is visible, potentially updating the ctid
    /// (e.g. if following a HOT chain). Returns `None` if the row is not visible.
    fn check(&mut self, ctid: u64) -> Option<u64>;
}
