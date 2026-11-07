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
#![allow(clippy::missing_safety_doc, unsafe_op_in_unsafe_fn)]

use crate::index::mvcc::SegmentView;
use crate::postgres::ParallelScanState;
use tantivy::index::SegmentId;

pub unsafe fn checkout_segment_for_source(
    pscan_state: *mut ParallelScanState,
    source_idx: usize,
) -> Option<SegmentId> {
    (*pscan_state).checkout_segment_for_source(source_idx)
}

pub unsafe fn segment_view(pscan_state: *mut ParallelScanState) -> SegmentView {
    (*pscan_state).segment_view()
}
