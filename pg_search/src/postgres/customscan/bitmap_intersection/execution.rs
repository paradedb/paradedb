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

//! Execution half of [`super`] (see the module docs there): runs the planned
//! child bitmap scan, owns the resulting TIDBitmap (and its DSA area when
//! shared), and publishes/attaches the claim table across rescans and
//! parallel participants.

use crate::query::tid_bitmap_stream::{
    BitmapCursorSource, SharedBitmapHandle, create_area, free_shared_table, publish_shared_table,
};
use pgrx::{PgList, pg_sys};
use std::ffi::CStr;
use std::sync::Arc;
use tantivy::index::SegmentId;

/// Owns the initialized child `BitmapIndexScan`/`BitmapAnd` planned from the
/// harvested path; builds its TIDBitmap on first execution and hands out the
/// cursor source the HeapFilter scorers stream from.
pub struct BitmapExec {
    child: *mut pg_sys::PlanState,
    consumed: bool,
    /// The completed TIDBitmap, retained across the scan so cursors can iterate
    /// it; freed on rescan/shutdown.
    tbm: *mut pg_sys::TIDBitmap,
    /// The DSA area the bitmap was built in (null = private).
    built_in: *mut pg_sys::dsa_area,
    /// This scan's own DSA area, when a shared build was published.
    area: *mut pg_sys::dsa_area,
    owns_area: bool,
    /// The published claim table, when shared.
    table: pg_sys::dsa_pointer,
    source: Option<Arc<BitmapCursorSource>>,
    /// Publish arguments cached for rescan republish.
    publish_args: Option<(u32, Vec<SegmentId>)>,
}

impl BitmapExec {
    /// Initialize the CustomScan's planned child bitmap scan, if it carries one.
    pub unsafe fn init(
        cscan: *mut pg_sys::CustomScan,
        estate: *mut pg_sys::EState,
        eflags: i32,
    ) -> Option<Self> {
        unsafe {
            let custom_plans = PgList::<pg_sys::Plan>::from_pg((*cscan).custom_plans);
            assert!(
                custom_plans.len() <= 1,
                "at most one harvested bitmap child is supported"
            );
            let child_plan = custom_plans.get_ptr(0)?;
            Some(Self {
                child: pg_sys::ExecInitNode(child_plan, estate, eflags),
                consumed: false,
                tbm: std::ptr::null_mut(),
                built_in: std::ptr::null_mut(),
                area: std::ptr::null_mut(),
                owns_area: false,
                table: 0,
                source: None,
                publish_args: None,
            })
        }
    }

    pub fn planstate(&self) -> *mut pg_sys::PlanState {
        self.child
    }

    /// Names of the indexes feeding the bitmap, in plan order (a `BitmapAnd`
    /// child contributes every leaf scan's index).
    pub fn index_names(&self) -> Vec<String> {
        unsafe fn collect(plan: *mut pg_sys::Plan, out: &mut Vec<String>) {
            unsafe {
                match (*plan).type_ {
                    pg_sys::NodeTag::T_BitmapIndexScan => {
                        let name =
                            pg_sys::get_rel_name((*plan.cast::<pg_sys::BitmapIndexScan>()).indexid);
                        if !name.is_null() {
                            out.push(CStr::from_ptr(name).to_string_lossy().into_owned());
                        }
                    }
                    pg_sys::NodeTag::T_BitmapAnd => {
                        let subplans = (*plan.cast::<pg_sys::BitmapAnd>()).bitmapplans;
                        for sub in PgList::<pg_sys::Plan>::from_pg(subplans).iter_ptr() {
                            collect(sub, out);
                        }
                    }
                    _ => {}
                }
            }
        }
        unsafe {
            let mut names = Vec::new();
            collect((*self.child).plan, &mut names);
            names
        }
    }

    /// Cursor counter totals (exact/lossy/recheck pages, rejected docs), once a
    /// source exists.
    pub fn cursor_stats(&self) -> Option<(u64, u64, u64, u64)> {
        self.source.as_ref().map(|source| source.counters())
    }

    pub unsafe fn shutdown(mut self) {
        unsafe {
            self.release_shared();
            self.free_tbm();
            if !self.area.is_null() {
                pg_sys::dsa_detach(self.area);
                self.area = std::ptr::null_mut();
            }
            pg_sys::ExecEndNode(self.child)
        }
    }

    /// Prepare for a rescan: params may have changed, so the bitmap must be rebuilt on
    /// the next execution. Runs after the previous execution's scorers are gone (and,
    /// for parallel scans, after its workers exited).
    pub unsafe fn rescan(&mut self) {
        unsafe {
            pg_sys::ExecReScan(self.child);
            self.release_shared();
            self.free_tbm();
        }
        self.consumed = false;
        self.source = None;
    }

    unsafe fn free_tbm(&mut self) {
        if !self.tbm.is_null() {
            unsafe { pg_sys::tbm_free(self.tbm) };
            self.tbm = std::ptr::null_mut();
            self.built_in = std::ptr::null_mut();
        }
    }

    /// Owner only: free the prepared iterator states and the claim table.
    unsafe fn release_shared(&mut self) {
        if self.owns_area && self.table != 0 {
            unsafe { free_shared_table(self.area, self.table) };
        }
        self.table = 0;
    }

    /// Build (or rebuild) the bitmap in `area` (null = private).
    unsafe fn ensure_built(&mut self, area: *mut pg_sys::dsa_area) {
        unsafe {
            if self.consumed {
                if self.built_in == area {
                    return;
                }
                // Built in the wrong place (e.g. a private estimate-time build that
                // must now be shared): run the child again into the right area.
                pg_sys::ExecReScan(self.child);
                self.free_tbm();
            }
            self.consumed = true;
            self.run_child(area);
        }
    }

    /// Serial path: build privately and hand out private-iterator cursors.
    pub(crate) unsafe fn private_source(&mut self) -> Option<Arc<BitmapCursorSource>> {
        unsafe {
            if self.source.is_none() {
                self.ensure_built(std::ptr::null_mut());
                if self.tbm.is_null() {
                    return None;
                }
                self.source = Some(Arc::new(BitmapCursorSource::private(self.tbm)));
            }
            self.source.clone()
        }
    }

    /// Owner only: build in this scan's own DSA area, prepare one iterator state
    /// per `(consumer, segment)` stream, and return the handle to publish for
    /// the other participants.
    pub unsafe fn shared_source(
        &mut self,
        consumers: u32,
        segments: &[SegmentId],
    ) -> Option<SharedBitmapHandle> {
        unsafe {
            if self.area.is_null() {
                self.area = create_area();
                self.owns_area = true;
            }
            self.release_shared();
            self.ensure_built(self.area);
            if self.tbm.is_null() {
                return None;
            }
            self.table = publish_shared_table(self.tbm, self.area, consumers, segments);
            self.publish_args = Some((consumers, segments.to_vec()));
            self.source = Some(Arc::new(BitmapCursorSource::shared(self.area, self.table)));
            Some(SharedBitmapHandle {
                area: pg_sys::dsa_get_handle(self.area),
                table: self.table,
            })
        }
    }

    /// Republish after a rescan reset, with the same consumers/segments.
    pub unsafe fn republish(&mut self) -> Option<SharedBitmapHandle> {
        unsafe {
            let (consumers, segments) = self.publish_args.clone()?;
            self.shared_source(consumers, &segments)
        }
    }

    /// The current source, once built.
    pub(crate) fn source(&self) -> Option<Arc<BitmapCursorSource>> {
        self.source.clone()
    }

    /// Worker: attach the owner's published area and claim table.
    pub(crate) unsafe fn worker_attach_source(
        &mut self,
        handle: SharedBitmapHandle,
    ) -> Arc<BitmapCursorSource> {
        unsafe {
            if self.area.is_null() {
                self.area = pg_sys::dsa_attach(handle.area);
                self.owns_area = false;
            }
            let source = Arc::new(BitmapCursorSource::shared(self.area, handle.table));
            self.source = Some(source.clone());
            source
        }
    }

    /// Run the planned child once, pre-seeding its first leaf with a TIDBitmap this
    /// node creates and owns — the parent-node contract documented in
    /// `MultiExecBitmapIndexScan`. A `BitmapAnd` child keeps its first leaf's bitmap
    /// as the intersection accumulator, so the seed lands there.
    unsafe fn run_child(&mut self, area: *mut pg_sys::dsa_area) {
        unsafe {
            let work_mem_bytes = (*std::ptr::addr_of!(pg_sys::work_mem)) as i64 * 1024;
            let tbm = pg_sys::tbm_create(work_mem_bytes as _, area);
            seed_first_leaf(self.child, tbm);
            let result = pg_sys::MultiExecProcNode(self.child);
            if result.is_null() {
                pg_sys::tbm_free(tbm);
                return;
            }
            assert_eq!(
                (*result).type_,
                pg_sys::NodeTag::T_TIDBitmap,
                "MultiExecProcNode must return a TIDBitmap"
            );
            debug_assert_eq!(
                result.cast::<pg_sys::TIDBitmap>(),
                tbm,
                "child must fill the pre-seeded bitmap"
            );
            self.tbm = result.cast();
            self.built_in = area;
        }
    }
}

/// Store `tbm` into the child's first `BitmapIndexScanState` so the child fills a
/// bitmap this node owns. A `BitmapAnd` retains its first subplan's bitmap as the
/// intersection accumulator, so recurse to that leaf.
unsafe fn seed_first_leaf(planstate: *mut pg_sys::PlanState, tbm: *mut pg_sys::TIDBitmap) {
    unsafe {
        match (*planstate).type_ {
            pg_sys::NodeTag::T_BitmapIndexScanState => {
                (*planstate.cast::<pg_sys::BitmapIndexScanState>()).biss_result = tbm;
            }
            pg_sys::NodeTag::T_BitmapAndState => {
                let and = planstate.cast::<pg_sys::BitmapAndState>();
                assert!((*and).nplans > 0, "BitmapAnd must have subplans");
                seed_first_leaf(*(*and).bitmapplans, tbm);
            }
            other => panic!("unexpected bitmap child node: {other:?}"),
        }
    }
}
