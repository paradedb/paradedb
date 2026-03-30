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

//! Re-exports of JoinScan infrastructure for use by AggregateScan's DataFusion backend.
//!
//! This module centralizes all imports from `joinscan/` that AggregateScan needs,
//! providing a single place to manage the dependency surface.
//!
//! These re-exports are intentionally unused until the DataFusion aggregate backend
//! is implemented (see #4483–#4488).

#![allow(unused_imports)]

// -- Relational IR tree --
pub use crate::postgres::customscan::joinscan::build::FilterNode;
pub use crate::postgres::customscan::joinscan::build::JoinCSClause;
pub use crate::postgres::customscan::joinscan::build::JoinKeyPair;
pub use crate::postgres::customscan::joinscan::build::JoinLevelExpr;
pub use crate::postgres::customscan::joinscan::build::JoinLevelSearchPredicate;
pub use crate::postgres::customscan::joinscan::build::JoinNode;
pub use crate::postgres::customscan::joinscan::build::JoinSource;
pub use crate::postgres::customscan::joinscan::build::JoinSourceCandidate;
pub use crate::postgres::customscan::joinscan::build::JoinType;
pub use crate::postgres::customscan::joinscan::build::MultiTablePredicateInfo;
pub use crate::postgres::customscan::joinscan::build::RelNode;
pub use crate::postgres::customscan::joinscan::build::RelationAlias;

// -- DataFusion plan building --
pub use crate::postgres::customscan::joinscan::scan_state::create_session_context;

// -- Memory pool --
pub use crate::postgres::customscan::joinscan::memory::create_memory_pool;

// -- Expression translation --
pub use crate::postgres::customscan::joinscan::translator::make_col;
pub use crate::postgres::customscan::joinscan::translator::PredicateTranslator;

// -- Scan infrastructure (already public via crate::scan) --
pub use crate::scan::info::ScanInfo;
pub use crate::scan::table_provider::PgSearchTableProvider;
