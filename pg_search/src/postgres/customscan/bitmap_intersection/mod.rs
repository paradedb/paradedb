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

//! Intersects a ParadeDB custom scan with a `BitmapIndexScan` over a non-ParadeDB
//! index (btree, GIN, PostGIS GiST) whose bitmapqual covers quals the scan would
//! otherwise evaluate as heap filters.
//!
//! Plan time ([`planning`]): `BitmapPlanner` gathers the coverable HeapExpr quals
//! and `harvest` builds a covering `BitmapHeapPath`, which the owning scan attaches
//! as a `custom_paths` child before rewriting the covered HeapFilters. Execution
//! time ([`execution`]): `BitmapExec` runs the planned child and streams its
//! TIDBitmap through per-`(consumer, segment)` cursors
//! (`crate::query::tid_bitmap_stream`) that the HeapFilter scorers probe.
//!
//! Scope: a single `BitmapIndexScan` or `BitmapAnd` tree over top-level AND clauses.
//! The owning scans keep deciding how heap expressions are extracted, where the
//! harvested child is attached, how the build cost is surfaced, and when
//! `BitmapExec` is initialized.

mod execution;
mod planning;

pub(crate) use execution::BitmapExec;
pub(crate) use planning::{BitmapPlanner, keep_bitmap_child_plan};
