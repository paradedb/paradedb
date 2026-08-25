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

//! Finds a `BitmapHeapPath` over a non-ParadeDB index (btree, GIN, PostGIS GiST) whose
//! bitmapqual covers quals we would otherwise evaluate as a `heap_filter`, so the
//! index's TIDBitmap can be intersected with the Tantivy result set.
//!
//! Plan time: `BitmapPlanner::from_query` gathers the coverable HeapExpr quals and
//! `harvest` finds or builds a covering `BitmapHeapPath`, which the caller attaches as
//! a `custom_paths` child and rewrites the covered HeapFilters. Execution time:
//! `BitmapExec` runs the planned child `BitmapIndexScan` and converts its TIDBitmap
//! into the `TidBitmapSet` the HeapFilter scorer probes.
//!
//! Scope: a single `BitmapIndexScan` or `BitmapAnd` tree over top-level AND clauses;

use crate::postgres::customscan::CustomScan;
use crate::postgres::customscan::builders::custom_scan::CustomScanBuilder;
use crate::postgres::customscan::qual_inspect::Qual;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::{BitmapBuildClaim, ParallelScanState};
use crate::query::SearchQueryInput;
use crate::query::heap_field_filter::TidBitmapSet;
use pgrx::{PgList, pg_sys};
use std::ffi::CStr;
use std::sync::Arc;

pub struct TidBitmapStats {
    /// Row locations the bitmap knows precisely; misses are rejected with no heap
    /// access or filter evaluation at all.
    pub exact_ctids: usize,
    /// Heap blocks the TIDBitmap degraded under `work_mem` pressure. Membership is
    /// untestable there: nothing can be rejected, and every row runs all filters.
    pub lossy_blocks: usize,
    /// Heap blocks whose exact candidates the index AM flagged as approximate matches.
    /// Absent ctids still reject; present ones also run the recheck filters.
    pub recheck_blocks: usize,
}

pub struct BitmapPlanner {
    root: *mut pg_sys::PlannerInfo,
    rel: *mut pg_sys::RelOptInfo,
    bm25_oid: pg_sys::Oid,
    heap_exprs: Vec<*mut pg_sys::Node>,
    /// Estimated rows the driving ParadeDB index scan will emit, when the caller has one.
    /// Feeds the cost ledger; without it, harvest falls back to first-match.
    bm25_row_estimate: Option<f64>,
}

struct HarvestedClause {
    index_name: String,
    clause: *mut pg_sys::Node,
    lossy: bool,
    matched_heap_expr: bool,
}

/// A successful harvest: the path to attach as a `custom_paths` child, plus the
/// HeapExpr clauses its bitmap covers (with their planner-level lossiness).
pub struct HarvestedBitmap {
    pub path: *mut pg_sys::Path,
    /// Estimated cost of building the bitmap and converting it into the
    /// probe-able set; the owning scan should surface it as startup cost.
    pub build_cost: f64,
    covered: Vec<(*mut pg_sys::Node, bool)>,
}

impl HarvestedBitmap {
    /// Flag every HeapFilter covered by the bitmap and move filters for exactly-matched
    /// (non-lossy) clauses into `recheck_filters`: their predicate is proven by exact
    /// bitmap membership and only needs re-evaluation on lossy/recheck pages. Lossy
    /// clauses stay in `always_filters`, which run on every bitmap survivor.
    pub unsafe fn rewrite_query(&self, query: &mut SearchQueryInput) {
        query.visit(&mut |sqi| {
            if let SearchQueryInput::HeapFilter {
                always_filters,
                recheck_filters,
                uses_tid_bitmap,
                ..
            } = sqi
            {
                let mut i = 0;
                while i < always_filters.len() {
                    let node = unsafe { always_filters[i].get_expression_node() };
                    let covered = self.covered.iter().find(|(clause, _)| unsafe {
                        pg_sys::equal(node.cast(), (*clause).cast())
                    });
                    match covered {
                        Some((_, lossy)) => {
                            *uses_tid_bitmap = true;
                            if !*lossy {
                                recheck_filters.push(always_filters.remove(i));
                                continue;
                            }
                            i += 1;
                        }
                        None => i += 1,
                    }
                }
            }
        });
    }
}

impl BitmapPlanner {
    /// Collect the expr node of every `Qual::HeapExpr` reachable through top-level AND
    /// structure. Returns `None` when there is nothing to cover, or when a HeapExpr
    /// appears under OR/NOT, where its bitmap could not be used to reject rows.
    /// (Covering OR clauses at whole-rinfo granularity via BitmapOr is future work.)
    pub fn from_query(
        root: *mut pg_sys::PlannerInfo,
        rel: *mut pg_sys::RelOptInfo,
        bm25_oid: pg_sys::Oid,
        quals: &Qual,
        bm25_row_estimate: Option<f64>,
    ) -> Option<Self> {
        fn collect(qual: &Qual, out: &mut Vec<*mut pg_sys::Node>) -> bool {
            match qual {
                Qual::HeapExpr { expr_node, .. } => {
                    out.push(*expr_node);
                    true
                }
                Qual::And(qs) => qs.iter().all(|q| collect(q, out)),
                Qual::Or(_) | Qual::Not(_) => !qual.contains_heap_expr(),
                _ => true,
            }
        }

        let mut heap_exprs = Vec::new();
        if !collect(quals, &mut heap_exprs) || heap_exprs.is_empty() {
            return None;
        }
        Some(Self {
            root,
            rel,
            bm25_oid,
            heap_exprs,
            bm25_row_estimate,
        })
    }

    /// Like `from_query`, but for callers that only have the built `SearchQueryInput`
    /// (the Aggregate Scan). HeapFilter expressions are collected through AND-position
    /// structure (`must` chains and score-neutral wrappers); a HeapFilter under
    /// `should`/`must_not` disqualifies for the same reason OR/NOT do in `from_query`.
    pub unsafe fn from_search_query(
        root: *mut pg_sys::PlannerInfo,
        rel: *mut pg_sys::RelOptInfo,
        bm25_oid: pg_sys::Oid,
        query: &SearchQueryInput,
        bm25_row_estimate: Option<f64>,
    ) -> Option<Self> {
        unsafe fn collect(sqi: &SearchQueryInput, out: &mut Vec<*mut pg_sys::Node>) -> bool {
            unsafe {
                match sqi {
                    SearchQueryInput::HeapFilter {
                        indexed_query,
                        always_filters,
                        ..
                    } => {
                        for filter in always_filters {
                            let node = filter.get_expression_node();
                            if !node.is_null() {
                                out.push(node);
                            }
                        }
                        collect(indexed_query, out)
                    }
                    SearchQueryInput::Boolean {
                        must,
                        should,
                        must_not,
                        ..
                    } => {
                        must.iter().all(|q| collect(q, out))
                            && !should
                                .iter()
                                .chain(must_not.iter())
                                .any(|q| q.has_heap_filters())
                    }
                    SearchQueryInput::Boost { query, .. }
                    | SearchQueryInput::ConstScore { query, .. }
                    | SearchQueryInput::WithIndex { query, .. } => collect(query, out),
                    SearchQueryInput::ScoreFilter { query, .. } => {
                        query.as_ref().is_none_or(|q| collect(q, out))
                    }
                    other => !other.has_heap_filters(),
                }
            }
        }

        let mut heap_exprs = Vec::new();
        if rel.is_null() || unsafe { !collect(query, &mut heap_exprs) } || heap_exprs.is_empty() {
            return None;
        }
        Some(Self {
            root,
            rel,
            bm25_oid,
            heap_exprs,
            bm25_row_estimate,
        })
    }

    /// Build a `BitmapHeapPath` usable as an intersection source. Always self-built:
    /// scavenging the rel's pathlists is unsound because `add_path` pfrees dominated
    /// non-IndexPath paths, so a harvested pointer could be freed before plan creation.
    pub unsafe fn harvest(&self) -> Option<HarvestedBitmap> {
        unsafe {
            self.build_bitmap_path()
                .and_then(|(bhp, build_cost)| self.accept(bhp, build_cost))
        }
    }

    /// Accept or reject one candidate: walk its bitmapqual tree enforcing the
    /// module-level scope rules, and require that it covers at least one of our
    /// HeapExpr clauses.
    unsafe fn accept(
        &self,
        bhp: *mut pg_sys::BitmapHeapPath,
        build_cost: f64,
    ) -> Option<HarvestedBitmap> {
        unsafe {
            // A parameterized path's index quals reference outer rels or nestloop
            // params the custom scan's plan context never supplies.
            if !(*bhp).path.param_info.is_null() {
                return None;
            }
            let mut clauses = Vec::new();
            let mut pending = vec![(*bhp).bitmapqual];
            while let Some(path) = pending.pop() {
                match (*path).type_ {
                    pg_sys::NodeTag::T_BitmapAndPath => {
                        let and_path: *mut pg_sys::BitmapAndPath = path.cast();
                        pending.extend(
                            PgList::<pg_sys::Path>::from_pg((*and_path).bitmapquals).iter_ptr(),
                        );
                    }
                    pg_sys::NodeTag::T_IndexPath => {
                        let ip: *mut pg_sys::IndexPath = path.cast();
                        let indexoid = (*(*ip).indexinfo).indexoid;
                        if indexoid == self.bm25_oid {
                            pgrx::debug1!(
                                "[bitmap_intersection] skipping BitmapHeapPath: bitmapqual is over the ParadeDB index itself"
                            );
                            return None;
                        }
                        let index_name = PgSearchRelation::open(indexoid).name().to_string();
                        for iclause in
                            PgList::<pg_sys::IndexClause>::from_pg((*ip).indexclauses).iter_ptr()
                        {
                            let clause = (*(*iclause).rinfo).clause.cast::<pg_sys::Node>();
                            clauses.push(HarvestedClause {
                                index_name: index_name.clone(),
                                clause,
                                lossy: (*iclause).lossy,
                                matched_heap_expr: self.matches_heap_expr(clause),
                            });
                        }
                    }
                    unsupported => {
                        // TODO: `BitmapOr` is not supported yet
                        pgrx::debug1!(
                            "[bitmap_intersection] skipping BitmapHeapPath: bitmapqual contains {:?}",
                            unsupported
                        );
                        return None;
                    }
                }
            }

            if !clauses.iter().any(|c| c.matched_heap_expr) {
                pgrx::debug1!(
                    "[bitmap_intersection] skipping BitmapHeapPath: covers no heap_filter clause"
                );
                return None;
            }

            for clause in &clauses {
                pgrx::debug1!(
                    "[bitmap_intersection] index={} lossy={} matched_heap_expr={}",
                    clause.index_name,
                    clause.lossy,
                    clause.matched_heap_expr,
                );
            }
            pgrx::debug1!(
                "[bitmap_intersection] harvested BitmapHeapPath: {} index clause(s), {} matched heap_filter clause(s), est rows={:.0}",
                clauses.len(),
                clauses.iter().filter(|c| c.matched_heap_expr).count(),
                (*bhp).path.rows,
            );
            Some(HarvestedBitmap {
                path: bhp.cast(),
                build_cost,
                covered: clauses
                    .into_iter()
                    .filter(|c| c.matched_heap_expr)
                    .map(|c| (c.clause, c.lossy))
                    .collect(),
            })
        }
    }

    /// Build a `BitmapHeapPath` over the non-ParadeDB index whose bitmap is worth
    /// intersecting: every index with a key column matching a HeapExpr clause is
    /// scored by [`Self::ledger`] and the best positive-net one wins. Without a
    /// ParadeDB-side row estimate the first workable index is taken unscored.
    unsafe fn build_bitmap_path(&self) -> Option<(*mut pg_sys::BitmapHeapPath, f64)> {
        unsafe {
            // TODO: This currently returns the best index, but we can
            // return all indexes that have a positive net benefit
            let mut best: Option<(*mut pg_sys::IndexPath, f64)> = None;
            for ioi in PgList::<pg_sys::IndexOptInfo>::from_pg((*self.rel).indexlist).iter_ptr() {
                if (*ioi).indexoid == self.bm25_oid || !(*ioi).amhasgetbitmap || (*ioi).hypothetical
                {
                    continue;
                }
                // Partial indexes need predicate-implication checks.
                if !(*ioi).indpred.is_null() {
                    continue;
                }

                let mut matched = Vec::new();
                for ri in PgList::<pg_sys::RestrictInfo>::from_pg((*ioi).indrestrictinfo).iter_ptr()
                {
                    if self.matches_heap_expr((*ri).clause.cast())
                        && let Some(iclause) = IndexClause::from_clause(self.root, ri, ioi)
                    {
                        matched.push(iclause);
                    }
                }
                if matched.is_empty() {
                    continue;
                }
                // `IndexPath.indexclauses` must be ordered by index column; the
                // clauses above accumulate in `indrestrictinfo` order.
                matched.sort_by_key(IndexClause::indexcol);
                let mut iclauses = PgList::<pg_sys::IndexClause>::new();
                for iclause in matched {
                    iclauses.push(iclause.into_pg());
                }
                let ipath = pg_sys::create_index_path(
                    self.root,
                    ioi,
                    iclauses.into_pg(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    pg_sys::ScanDirection::ForwardScanDirection,
                    false,
                    std::ptr::null_mut(),
                    1.0,
                    false,
                );
                let index_name = PgSearchRelation::open((*ioi).indexoid).name().to_string();
                if self.overflows_work_mem(ipath) {
                    pgrx::debug1!(
                        "[bitmap_intersection] index {index_name}: bitmap would overflow work_mem, skipping"
                    );
                    continue;
                }
                let Some(bm25_row_estimate) = self.bm25_row_estimate else {
                    return Some((self.heap_path(ipath), self.bitmap_build_cost(ipath)));
                };
                let net = self.ledger(ipath, bm25_row_estimate);
                pgrx::debug1!(
                    "[bitmap_intersection] index {index_name}: net={net:.2} (bm25_row_estimate={bm25_row_estimate:.0} selectivity={:.6} indextotalcost={:.2})",
                    (*ipath).indexselectivity,
                    (*ipath).indextotalcost,
                );
                if net > 0.0 && best.is_none_or(|(_, b)| net > b) {
                    best = Some((ipath, net));
                }
            }
            best.map(|(ipath, _)| (self.heap_path(ipath), self.bitmap_build_cost(ipath)))
        }
    }

    /// Cost of producing the probe-able set: the index scan that builds the
    /// TIDBitmap plus converting its entries.
    unsafe fn bitmap_build_cost(&self, ipath: *mut pg_sys::IndexPath) -> f64 {
        unsafe {
            let bitmap_rows =
                (*ipath).indexselectivity.clamp(0.0, 1.0) * (*self.rel).tuples.max(1.0);
            let cpu_operator_cost = *std::ptr::addr_of!(pg_sys::cpu_operator_cost);
            (*ipath).indextotalcost + bitmap_rows * 0.1 * cpu_operator_cost
        }
    }

    /// Net benefit of probing this index's bitmap ahead of the heap filters.
    ///
    /// Benefit: each ParadeDB-index row the bitmap rejects (the non-selected fraction)
    /// skips its heap fetch and all heap filter evaluation.
    ///
    //  Cost: building the bitmap (`indextotalcost`) plus converting its entries into
    // the probe-able set.
    unsafe fn ledger(&self, ipath: *mut pg_sys::IndexPath, bm25_row_estimate: f64) -> f64 {
        unsafe {
            let tuples = (*self.rel).tuples.max(1.0);
            let pages = (*self.rel).pages as f64;
            let sel = (*ipath).indexselectivity.clamp(0.0, 1.0);

            let cpu_tuple_cost = *std::ptr::addr_of!(pg_sys::cpu_tuple_cost);
            let random_page_cost = *std::ptr::addr_of!(pg_sys::random_page_cost);

            let mut qual_cost = pg_sys::QualCost::default();
            let mut exprs = PgList::<pg_sys::Node>::new();
            for expr in &self.heap_exprs {
                exprs.push(*expr);
            }
            pg_sys::cost_qual_eval(&mut qual_cost, exprs.into_pg(), self.root);

            let per_row_saved =
                cpu_tuple_cost + qual_cost.per_tuple + (pages / tuples) * random_page_cost;
            let benefit = bm25_row_estimate * (1.0 - sel) * per_row_saved;
            benefit - self.bitmap_build_cost(ipath)
        }
    }

    /// A bitmap whose estimated TID count can't fit in `work_mem` degrades to lossy
    /// pages, which cannot reject anything.
    unsafe fn overflows_work_mem(&self, ipath: *mut pg_sys::IndexPath) -> bool {
        unsafe {
            let bitmap_rows =
                (*ipath).indexselectivity.clamp(0.0, 1.0) * (*self.rel).tuples.max(1.0);
            let work_mem_kb = *std::ptr::addr_of!(pg_sys::work_mem) as f64;
            bitmap_rows * 8.0 > work_mem_kb * 1024.0
        }
    }

    unsafe fn heap_path(&self, ipath: *mut pg_sys::IndexPath) -> *mut pg_sys::BitmapHeapPath {
        unsafe {
            pg_sys::create_bitmap_heap_path(
                self.root,
                self.rel,
                ipath.cast(),
                (*self.rel).lateral_relids,
                1.0,
                0,
            )
        }
    }

    unsafe fn matches_heap_expr(&self, clause: *mut pg_sys::Node) -> bool {
        unsafe {
            self.heap_exprs
                .iter()
                .any(|expr| pg_sys::equal(clause.cast(), (*expr).cast()))
        }
    }
}

/// `create_plan_recurse` can't emit a bare `BitmapIndexScan`, so a harvested bitmap
/// path arrives planned as a full BitmapHeapScan; keep only its bitmap-producing
/// subtree (`BitmapIndexScan` / `BitmapAnd`). Shared by every scan type that carries a
/// harvested child.
pub unsafe fn keep_bitmap_child_plan<CS: CustomScan>(builder: &mut CustomScanBuilder<CS>) {
    unsafe {
        let custom_plans = PgList::<pg_sys::Plan>::from_pg(builder.custom_plans());
        if let Some(child) = custom_plans.get_ptr(0)
            && (*child).type_ == pg_sys::NodeTag::T_BitmapHeapScan
        {
            let mut replacement = PgList::<pg_sys::Plan>::new();
            replacement.push((*child).lefttree);
            builder.set_custom_plans(replacement.into_pg());
        }
    }
}

/// Execution half: owns the initialized child `BitmapIndexScan`/`BitmapAnd` planned
/// from the harvested path, and converts its TIDBitmap into a probe-able
/// `TidBitmapSet` on first execution.
pub struct BitmapExec {
    child: *mut pg_sys::PlanState,
    consumed: bool,
    set: Option<Arc<TidBitmapSet>>,
}

impl BitmapExec {
    /// Initialize the CustomScan's planned child bitmap scan, if it carries one.
    pub unsafe fn init(
        cscan: *mut pg_sys::CustomScan,
        estate: *mut pg_sys::EState,
        eflags: i32,
    ) -> Option<Self> {
        unsafe {
            let child_plan = PgList::<pg_sys::Plan>::from_pg((*cscan).custom_plans).get_ptr(0)?;
            Some(Self {
                child: pg_sys::ExecInitNode(child_plan, estate, eflags),
                consumed: false,
                set: None,
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

    /// Sizes of the built `TidBitmapSet`, if the child scan has run.
    pub fn tid_bitmap_stats(&self) -> Option<TidBitmapStats> {
        self.set.as_ref().map(|set| TidBitmapStats {
            exact_ctids: set.exact_ctids.len(),
            lossy_blocks: set.lossy_blocks.len(),
            recheck_blocks: set.recheck_blocks.len(),
        })
    }

    pub unsafe fn shutdown(self) {
        unsafe { pg_sys::ExecEndNode(self.child) }
    }

    /// Prepare for a rescan: params may have changed, so the bitmap must be rebuilt on
    /// the next execution.
    pub unsafe fn rescan(&mut self) {
        unsafe { pg_sys::ExecReScan(self.child) };
        self.consumed = false;
        self.set = None;
    }

    /// Parallel variant of `tid_bitmap_set`: the first participant to arrive builds the
    /// set from its own child scan and publishes the flat bytes into the query's DSA;
    /// every other participant waits on the shared state and reads them back.
    /// (Parallel Bitmap Heap Scan's build-once coordination, with our own probe-able
    /// representation instead of a shared TIDBitmap.)
    pub unsafe fn shared_tid_bitmap_set(
        &mut self,
        pstate: *mut ParallelScanState,
        dsa: *mut pg_sys::dsa_area,
    ) -> Option<Arc<TidBitmapSet>> {
        unsafe {
            if pstate.is_null() || dsa.is_null() {
                return self.tid_bitmap_set();
            }
            match (*pstate).bitmap_claim_build() {
                BitmapBuildClaim::Build => {
                    let set = self.tid_bitmap_set();
                    let pointer = set
                        .as_deref()
                        .map_or(0, |set| DsaTidBitmapSet::from_set(dsa, set).raw());
                    (*pstate).bitmap_publish(pointer);
                    set
                }
                BitmapBuildClaim::Wait => DsaTidBitmapSet((*pstate).bitmap_wait_done()).read(dsa),
                BitmapBuildClaim::Done(pointer) => DsaTidBitmapSet(pointer).read(dsa),
            }
        }
    }

    /// Run the child via `MultiExecProcNode` (first execution only) and convert its
    /// TIDBitmap into a `TidBitmapSet`.
    pub unsafe fn tid_bitmap_set(&mut self) -> Option<Arc<TidBitmapSet>> {
        if !self.consumed {
            self.consumed = true;
            self.set = unsafe { self.build_tid_bitmap_set() };
        }
        self.set.clone()
    }

    unsafe fn build_tid_bitmap_set(&mut self) -> Option<Arc<TidBitmapSet>> {
        unsafe {
            let result = pg_sys::MultiExecProcNode(self.child);
            if result.is_null() {
                return None;
            }
            assert_eq!(
                (*result).type_,
                pg_sys::NodeTag::T_TIDBitmap,
                "MultiExecProcNode must return a TIDBitmap"
            );
            let tbm: *mut pg_sys::TIDBitmap = result.cast();

            // TIDBitmap iteration returns pages in block order with ascending offsets,
            // so the vecs below are built sorted, as `TidBitmapSet::probe` requires.
            let mut set = TidBitmapSet::default();
            let pack = |blockno: u32, offset: pg_sys::OffsetNumber| {
                ((blockno as u64) << 16) | (offset as u64)
            };

            #[cfg(feature = "pg18")]
            {
                // Safe over-approximation of MaxHeapTuplesPerPage (which divides by
                // >= 28 per tuple) for any BLCKSZ; the macro is not in the bindings.
                const OFFSETS_CAP: usize = (pg_sys::BLCKSZ as usize) / 16;
                let iter = pg_sys::tbm_begin_private_iterate(tbm);
                let mut res = pg_sys::TBMIterateResult::default();
                let mut offsets = [0 as pg_sys::OffsetNumber; OFFSETS_CAP];
                while pg_sys::tbm_private_iterate(iter, &mut res) {
                    if res.lossy {
                        set.lossy_blocks.push(res.blockno);
                    } else {
                        let ntuples = pg_sys::tbm_extract_page_tuple(
                            &mut res,
                            offsets.as_mut_ptr(),
                            OFFSETS_CAP as u32,
                        );
                        for &offset in &offsets[..ntuples as usize] {
                            set.exact_ctids.push(pack(res.blockno, offset));
                        }
                        if res.recheck {
                            set.recheck_blocks.push(res.blockno);
                        }
                    }
                }
                pg_sys::tbm_end_private_iterate(iter);
            }

            #[cfg(not(feature = "pg18"))]
            {
                let iter = pg_sys::tbm_begin_iterate(tbm);
                loop {
                    let res = pg_sys::tbm_iterate(iter);
                    if res.is_null() {
                        break;
                    }
                    if (*res).ntuples < 0 {
                        set.lossy_blocks.push((*res).blockno);
                    } else {
                        for &offset in (*res).offsets.as_slice((*res).ntuples as usize) {
                            set.exact_ctids.push(pack((*res).blockno, offset));
                        }
                        if (*res).recheck {
                            set.recheck_blocks.push((*res).blockno);
                        }
                    }
                }
                pg_sys::tbm_end_iterate(iter);
            }

            pg_sys::tbm_free(tbm);
            pgrx::debug1!(
                "[bitmap_intersection] TidBitmapSet: {} exact ctids, {} lossy blocks, {} recheck blocks",
                set.exact_ctids.len(),
                set.lossy_blocks.len(),
                set.recheck_blocks.len(),
            );
            Some(Arc::new(set))
        }
    }
}

struct IndexClause(*mut pg_sys::IndexClause);

impl IndexClause {
    /// Match one restriction clause against every key column of `ioi`.
    unsafe fn from_clause(
        root: *mut pg_sys::PlannerInfo,
        ri: *mut pg_sys::RestrictInfo,
        ioi: *mut pg_sys::IndexOptInfo,
    ) -> Option<Self> {
        unsafe {
            // Same gates as core's `match_clause_to_index`: pseudoconstant clauses
            // never become index quals, and a clause above the rel's minimum
            // security level (RLS / security-barrier quals) must not be evaluated
            // early by the index AM unless it is leakproof.
            if (*ri).pseudoconstant || !pg_sys::restriction_is_securely_promotable(ri, (*ioi).rel) {
                return None;
            }
            match (*(*ri).clause.cast::<pg_sys::Node>()).type_ {
                pg_sys::NodeTag::T_OpExpr => Self::from_opexpr(root, ri, ioi),
                pg_sys::NodeTag::T_FuncExpr => Self::from_funcexpr(root, ri, ioi),
                _ => None,
            }
        }
    }

    unsafe fn from_opexpr(
        root: *mut pg_sys::PlannerInfo,
        ri: *mut pg_sys::RestrictInfo,
        ioi: *mut pg_sys::IndexOptInfo,
    ) -> Option<Self> {
        unsafe {
            let opexpr = (*ri).clause.cast::<pg_sys::OpExpr>();
            let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);
            if args.len() != 2 {
                return None;
            }
            let (leftop, rightop) = (
                strip_relabel(args.get_ptr(0)?),
                strip_relabel(args.get_ptr(1)?),
            );

            for indexcol in 0..(*ioi).nkeycolumns {
                let opfamily = *(*ioi).opfamily.add(indexcol as usize);
                // The direct opfamily arms require the index collation to match the operator's
                // input collation, or the index answers under different comparison semantics
                // and its bitmap can miss valid rows.
                let idxcoll = *(*ioi).indexcollations.add(indexcol as usize);
                let collation_ok =
                    idxcoll == pg_sys::InvalidOid || idxcoll == (*opexpr).inputcollid;

                if pg_sys::match_index_to_operand(leftop, indexcol, ioi)
                    && is_pseudoconstant(rightop)
                {
                    if collation_ok && pg_sys::op_in_opfamily((*opexpr).opno, opfamily) {
                        let mut indexquals = PgList::<pg_sys::RestrictInfo>::new();
                        indexquals.push(ri);
                        return Some(Self::new(ri, indexquals, false, indexcol));
                    }
                    if let Some(iclause) =
                        Self::from_support_function(root, ri, (*opexpr).opfuncid, 0, indexcol, ioi)
                    {
                        return Some(iclause);
                    }
                }

                if pg_sys::match_index_to_operand(rightop, indexcol, ioi)
                    && is_pseudoconstant(leftop)
                {
                    let comm_op = pg_sys::get_commutator((*opexpr).opno);
                    if collation_ok
                        && comm_op != pg_sys::InvalidOid
                        && pg_sys::op_in_opfamily(comm_op, opfamily)
                    {
                        let mut indexquals = PgList::<pg_sys::RestrictInfo>::new();
                        indexquals.push(pg_sys::commute_restrictinfo(ri, comm_op));
                        return Some(Self::new(ri, indexquals, false, indexcol));
                    }
                    if let Some(iclause) =
                        Self::from_support_function(root, ri, (*opexpr).opfuncid, 1, indexcol, ioi)
                    {
                        return Some(iclause);
                    }
                }
            }
            None
        }
    }

    unsafe fn from_funcexpr(
        root: *mut pg_sys::PlannerInfo,
        ri: *mut pg_sys::RestrictInfo,
        ioi: *mut pg_sys::IndexOptInfo,
    ) -> Option<Self> {
        unsafe {
            let funcexpr = (*ri).clause.cast::<pg_sys::FuncExpr>();
            let args = PgList::<pg_sys::Node>::from_pg((*funcexpr).args);
            for indexcol in 0..(*ioi).nkeycolumns {
                for (indexarg, arg) in args.iter_ptr().enumerate() {
                    if pg_sys::match_index_to_operand(strip_relabel(arg), indexcol, ioi)
                        && let Some(iclause) = Self::from_support_function(
                            root,
                            ri,
                            (*funcexpr).funcid,
                            indexarg as i32,
                            indexcol,
                            ioi,
                        )
                    {
                        return Some(iclause);
                    }
                }
            }
            None
        }
    }

    /// Support functions allow quals to be derived from an index, potentially lossily
    unsafe fn from_support_function(
        root: *mut pg_sys::PlannerInfo,
        ri: *mut pg_sys::RestrictInfo,
        funcid: pg_sys::Oid,
        indexarg: i32,
        indexcol: i32,
        ioi: *mut pg_sys::IndexOptInfo,
    ) -> Option<Self> {
        unsafe {
            let prosupport = pg_sys::get_func_support(funcid);
            if prosupport == pg_sys::InvalidOid {
                return None;
            }

            let mut req = pg_sys::SupportRequestIndexCondition::default();
            req.type_ = pg_sys::NodeTag::T_SupportRequestIndexCondition;
            req.root = root;
            req.funcid = funcid;
            req.node = (*ri).clause.cast();
            req.indexarg = indexarg;
            req.index = ioi;
            req.indexcol = indexcol;
            req.opfamily = *(*ioi).opfamily.add(indexcol as usize);
            req.indexcollation = *(*ioi).indexcollations.add(indexcol as usize);
            req.lossy = true;

            let result = pg_sys::OidFunctionCall1Coll(
                prosupport,
                pg_sys::InvalidOid,
                pg_sys::Datum::from(std::ptr::addr_of_mut!(req) as usize),
            );
            let derived: *mut pg_sys::List = result.cast_mut_ptr();
            if derived.is_null() {
                return None;
            }

            let mut indexquals = PgList::<pg_sys::RestrictInfo>::new();
            for expr in PgList::<pg_sys::Node>::from_pg(derived).iter_ptr() {
                indexquals.push(make_simple_restrictinfo(root, expr.cast()));
            }
            if indexquals.is_empty() {
                return None;
            }
            Some(Self::new(ri, indexquals, req.lossy, indexcol))
        }
    }

    unsafe fn new(
        ri: *mut pg_sys::RestrictInfo,
        indexquals: PgList<pg_sys::RestrictInfo>,
        lossy: bool,
        indexcol: i32,
    ) -> Self {
        unsafe {
            let iclause: *mut pg_sys::IndexClause =
                pg_sys::palloc0(std::mem::size_of::<pg_sys::IndexClause>()).cast();
            (*iclause).type_ = pg_sys::NodeTag::T_IndexClause;
            (*iclause).rinfo = ri;
            (*iclause).indexquals = indexquals.into_pg();
            (*iclause).lossy = lossy;
            (*iclause).indexcol = indexcol as _;
            (*iclause).indexcols = std::ptr::null_mut();
            Self(iclause)
        }
    }

    fn indexcol(&self) -> pg_sys::AttrNumber {
        unsafe { (*self.0).indexcol }
    }

    fn into_pg(self) -> *mut pg_sys::IndexClause {
        self.0
    }
}

/// A `TidBitmapSet` serialized into one DSA allocation, so a single build can be
/// shared across every process of a parallel scan. Wraps the `dsa_pointer` that
/// `ParallelScanState` carries; `0` (invalid) means nothing was published.
///
/// Flat layout: `[exact_len u64][lossy_len u64][recheck_len u64]` followed by the
/// three arrays. DSA allocations are MAXALIGNed, so the typed copies are aligned.
#[derive(Clone, Copy)]
struct DsaTidBitmapSet(pg_sys::dsa_pointer);

impl DsaTidBitmapSet {
    unsafe fn from_set(dsa: *mut pg_sys::dsa_area, set: &TidBitmapSet) -> Self {
        unsafe {
            let (ne, nl, nr) = (
                set.exact_ctids.len(),
                set.lossy_blocks.len(),
                set.recheck_blocks.len(),
            );
            let size = 24 + ne * 8 + nl * 4 + nr * 4;
            let pointer = pg_sys::dsa_allocate_extended(dsa, size, 0);
            let base = pg_sys::dsa_get_address(dsa, pointer).cast::<u8>();

            let header = base.cast::<u64>();
            header.write(ne as u64);
            header.add(1).write(nl as u64);
            header.add(2).write(nr as u64);

            let mut dst = base.add(24);
            std::ptr::copy_nonoverlapping(set.exact_ctids.as_ptr().cast::<u8>(), dst, ne * 8);
            dst = dst.add(ne * 8);
            std::ptr::copy_nonoverlapping(set.lossy_blocks.as_ptr().cast::<u8>(), dst, nl * 4);
            dst = dst.add(nl * 4);
            std::ptr::copy_nonoverlapping(set.recheck_blocks.as_ptr().cast::<u8>(), dst, nr * 4);
            Self(pointer)
        }
    }

    /// Deserialize into a process-local copy; `None` when nothing was published.
    unsafe fn read(self, dsa: *mut pg_sys::dsa_area) -> Option<Arc<TidBitmapSet>> {
        unsafe {
            if self.0 == 0 {
                return None;
            }
            let base = pg_sys::dsa_get_address(dsa, self.0).cast::<u8>();
            let header = std::slice::from_raw_parts(base.cast::<u64>(), 3);
            let (ne, nl, nr) = (header[0] as usize, header[1] as usize, header[2] as usize);
            let exact = std::slice::from_raw_parts(base.add(24).cast::<u64>(), ne);
            let lossy = std::slice::from_raw_parts(base.add(24 + ne * 8).cast::<u32>(), nl);
            let recheck =
                std::slice::from_raw_parts(base.add(24 + ne * 8 + nl * 4).cast::<u32>(), nr);
            Some(Arc::new(TidBitmapSet {
                exact_ctids: exact.to_vec(),
                lossy_blocks: lossy.to_vec(),
                recheck_blocks: recheck.to_vec(),
            }))
        }
    }

    fn raw(self) -> pg_sys::dsa_pointer {
        self.0
    }
}

unsafe fn strip_relabel(mut node: *mut pg_sys::Node) -> *mut pg_sys::Node {
    unsafe {
        while (*node).type_ == pg_sys::NodeTag::T_RelabelType {
            node = (*node.cast::<pg_sys::RelabelType>()).arg.cast();
        }
        node
    }
}

unsafe fn is_pseudoconstant(node: *mut pg_sys::Node) -> bool {
    unsafe { !pg_sys::contain_var_clause(node) && !pg_sys::contain_volatile_functions(node) }
}

#[cfg(feature = "pg15")]
unsafe fn make_simple_restrictinfo(
    root: *mut pg_sys::PlannerInfo,
    clause: *mut pg_sys::Expr,
) -> *mut pg_sys::RestrictInfo {
    unsafe {
        pg_sys::make_restrictinfo(
            root,
            clause,
            true,
            false,
            false,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    }
}

#[cfg(not(feature = "pg15"))]
unsafe fn make_simple_restrictinfo(
    root: *mut pg_sys::PlannerInfo,
    clause: *mut pg_sys::Expr,
) -> *mut pg_sys::RestrictInfo {
    unsafe {
        pg_sys::make_restrictinfo(
            root,
            clause,
            true,
            false,
            false,
            false,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    }
}
