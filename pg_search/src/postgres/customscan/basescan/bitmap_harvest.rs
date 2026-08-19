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

//! SPIKE (#5702): harvest a planner-generated `BitmapHeapPath` whose bitmapqual covers
//! quals we would otherwise evaluate as a `heap_filter`, so a standard `BitmapIndexScan`
//! (e.g. over a PostGIS/btree/GIN index) can be executed as a child of our custom scan
//! and its TIDBitmap intersected with the Tantivy result set.
//!
//! Scope (M1): a single BitmapIndexScan or a BitmapAnd tree; BitmapOr is gated out.
//! Classification invariant: `IndexClause.lossy` => the original predicate must ALWAYS
//! be re-evaluated on bitmap survivors ("always filter"); non-lossy originals only need
//! re-evaluation when the TBM page recheck bit is set ("recheck filter").

use crate::postgres::customscan::qual_inspect::Qual;
use pgrx::{PgList, pg_sys};

/// Collect the expr nodes of every `Qual::HeapExpr` reachable through top-level AND
/// structure. Returns `false` (gate violation) if a HeapExpr appears under OR/NOT:
/// decomposing those would be unsound at leaf granularity, so M1 skips harvesting.
pub fn collect_heap_expr_nodes(qual: &Qual, out: &mut Vec<*mut pg_sys::Node>) -> bool {
    fn contains_heap_expr(qual: &Qual) -> bool {
        match qual {
            Qual::HeapExpr { .. } => true,
            Qual::And(qs) | Qual::Or(qs) => qs.iter().any(contains_heap_expr),
            Qual::Not(q) => contains_heap_expr(q),
            _ => false,
        }
    }

    match qual {
        Qual::HeapExpr { expr_node, .. } => {
            out.push(*expr_node);
            true
        }
        Qual::And(qs) => qs.iter().all(|q| collect_heap_expr_nodes(q, out)),
        Qual::Or(qs) => !qs.iter().any(contains_heap_expr),
        Qual::Not(q) => !contains_heap_expr(q),
        _ => true,
    }
}

struct HarvestedClause {
    index_name: String,
    clause: *mut pg_sys::Node,
    lossy: bool,
    matched_heap_expr: bool,
}

/// Walk a bitmapqual path tree, collecting leaf IndexPaths. Returns None on a
/// BitmapOr (M1 gate) or an unrecognized node.
unsafe fn collect_index_paths(
    path: *mut pg_sys::Path,
    out: &mut Vec<*mut pg_sys::IndexPath>,
) -> Option<()> {
    unsafe {
        match (*path).type_ {
            pg_sys::NodeTag::T_IndexPath => {
                out.push(path.cast());
                Some(())
            }
            pg_sys::NodeTag::T_BitmapAndPath => {
                let and_path: *mut pg_sys::BitmapAndPath = path.cast();
                for sub in PgList::<pg_sys::Path>::from_pg((*and_path).bitmapquals).iter_ptr() {
                    collect_index_paths(sub, out)?;
                }
                Some(())
            }
            _ => None,
        }
    }
}

unsafe fn rel_name(oid: pg_sys::Oid) -> String {
    unsafe {
        let name = pg_sys::get_rel_name(oid);
        if name.is_null() {
            format!("oid={}", oid.to_u32())
        } else {
            std::ffi::CStr::from_ptr(name)
                .to_string_lossy()
                .into_owned()
        }
    }
}

unsafe fn describe_clause(clause: *mut pg_sys::Node) -> String {
    unsafe {
        let cstr = pg_sys::nodeToString(clause.cast());
        let s = std::ffi::CStr::from_ptr(cstr)
            .to_string_lossy()
            .into_owned();
        // nodeToString is verbose; grab the function/operator identity for the log line
        let mut summary: String = s.chars().take(120).collect();
        if s.len() > 120 {
            summary.push('…');
        }
        summary
    }
}

/// Scan `rel->pathlist` (and `partial_pathlist`) for a `BitmapHeapPath` usable as an
/// intersection source: AND-only bitmapqual, not over the BM25 index itself, covering
/// at least one of our HeapExpr clauses. Logs everything it sees (spike deliverable).
pub unsafe fn harvest(
    root: *mut pg_sys::PlannerInfo,
    rel: *mut pg_sys::RelOptInfo,
    bm25_oid: pg_sys::Oid,
    heap_exprs: &[*mut pg_sys::Node],
) -> Option<*mut pg_sys::Path> {
    unsafe {
        let mut candidates = Vec::new();
        for path in PgList::<pg_sys::Path>::from_pg((*rel).pathlist)
            .iter_ptr()
            .chain(PgList::<pg_sys::Path>::from_pg((*rel).partial_pathlist).iter_ptr())
        {
            if (*path).type_ == pg_sys::NodeTag::T_BitmapHeapPath {
                candidates.push(path.cast::<pg_sys::BitmapHeapPath>());
            }
        }

        // A BitmapHeapPath only appears in the pathlist if it won add_path competition
        // (usually against seqscan). When none survived, build one ourselves: for each
        // non-BM25 index, match our HeapExpr clauses to index columns using the exported
        // planner primitives, then create_index_path + create_bitmap_heap_path directly.
        // (Re-invoking create_index_paths proved unreliable: its internal add_path
        // competition against the BM25 index's own path discards what we need.)
        if candidates.is_empty()
            && let Some(bhp) = build_bitmap_path_directly(root, rel, bm25_oid, heap_exprs)
        {
            candidates.push(bhp.cast());
        }

        for bhp in candidates {
            let mut index_paths = Vec::new();
            if collect_index_paths((*bhp).bitmapqual, &mut index_paths).is_none() {
                pgrx::notice!(
                    "[bitmap_harvest] skipping BitmapHeapPath: bitmapqual contains BitmapOr (M1 gate)"
                );
                continue;
            }

            let mut clauses = Vec::new();
            let mut uses_bm25 = false;
            for ip in &index_paths {
                let indexoid = (*(**ip).indexinfo).indexoid;
                if indexoid == bm25_oid {
                    uses_bm25 = true;
                    break;
                }
                let index_name = {
                    let name = pg_sys::get_rel_name(indexoid);
                    if name.is_null() {
                        format!("oid={}", indexoid.to_u32())
                    } else {
                        std::ffi::CStr::from_ptr(name)
                            .to_string_lossy()
                            .into_owned()
                    }
                };
                for iclause in
                    PgList::<pg_sys::IndexClause>::from_pg((**ip).indexclauses).iter_ptr()
                {
                    let rinfo = (*iclause).rinfo;
                    let clause = (*rinfo).clause.cast::<pg_sys::Node>();
                    let matched = heap_exprs
                        .iter()
                        .any(|he| pg_sys::equal(clause.cast(), (*he).cast()));
                    clauses.push(HarvestedClause {
                        index_name: index_name.clone(),
                        clause,
                        lossy: (*iclause).lossy,
                        matched_heap_expr: matched,
                    });
                }
            }

            if uses_bm25 {
                pgrx::notice!(
                    "[bitmap_harvest] skipping BitmapHeapPath: bitmapqual is over the BM25 index itself"
                );
                continue;
            }

            let matched_count = clauses.iter().filter(|c| c.matched_heap_expr).count();
            for c in &clauses {
                pgrx::notice!(
                    "[bitmap_harvest] index={} lossy={} ({}) matched_heap_expr={} clause={}",
                    c.index_name,
                    c.lossy,
                    if c.lossy {
                        "always_filter: original must re-run on every survivor"
                    } else {
                        "recheck_filter: original re-runs only on TBM-recheck pages"
                    },
                    c.matched_heap_expr,
                    describe_clause(c.clause),
                );
            }

            if matched_count == 0 {
                pgrx::notice!(
                    "[bitmap_harvest] skipping BitmapHeapPath: covers no heap_filter clause (no benefit)"
                );
                continue;
            }

            pgrx::notice!(
                "[bitmap_harvest] HARVESTED BitmapHeapPath: {} index clause(s), {} matched heap_filter clause(s), \
                 est rows={:.0} cost={:.2}",
                clauses.len(),
                matched_count,
                (*bhp).path.rows,
                (*bhp).path.total_cost,
            );
            return Some(bhp.cast());
        }

        None
    }
}

/// Deterministically build a BitmapHeapPath over the best non-BM25 index covering our
/// HeapExpr clauses, using exported planner primitives. Handles direct matches
/// (indexkey OP pseudoconstant, operator in the index's opfamily); support-function
/// derivation (LIKE prefix, PostGIS ST_DWithin) is Phase 2.
unsafe fn build_bitmap_path_directly(
    root: *mut pg_sys::PlannerInfo,
    rel: *mut pg_sys::RelOptInfo,
    bm25_oid: pg_sys::Oid,
    heap_exprs: &[*mut pg_sys::Node],
) -> Option<*mut pg_sys::BitmapHeapPath> {
    unsafe {
        for ioi in PgList::<pg_sys::IndexOptInfo>::from_pg((*rel).indexlist).iter_ptr() {
            if (*ioi).indexoid == bm25_oid || !(*ioi).amhasgetbitmap {
                continue;
            }
            // Partial indexes need predicate-implication checks; out of spike scope.
            if !(*ioi).indpred.is_null() {
                continue;
            }

            let mut iclauses = PgList::<pg_sys::IndexClause>::new();
            for ri in PgList::<pg_sys::RestrictInfo>::from_pg((*ioi).indrestrictinfo).iter_ptr() {
                let clause = (*ri).clause.cast::<pg_sys::Node>();
                if !heap_exprs
                    .iter()
                    .any(|he| pg_sys::equal(clause.cast(), (*he).cast()))
                {
                    continue;
                }
                if (*clause).type_ != pg_sys::NodeTag::T_OpExpr {
                    continue;
                }
                let opexpr = clause.cast::<pg_sys::OpExpr>();
                let args = PgList::<pg_sys::Node>::from_pg((*opexpr).args);
                if args.len() != 2 {
                    continue;
                }
                let (mut leftop, rightop) = (args.get_ptr(0)?, args.get_ptr(1)?);
                if (*leftop).type_ == pg_sys::NodeTag::T_RelabelType {
                    leftop = (*leftop.cast::<pg_sys::RelabelType>()).arg.cast();
                }
                if contain_var_clause_checked(rightop)
                    || pg_sys::contain_volatile_functions(rightop)
                {
                    continue;
                }

                for indexcol in 0..(*ioi).nkeycolumns {
                    if pg_sys::match_index_to_operand(leftop, indexcol, ioi)
                        && pg_sys::op_in_opfamily(
                            (*opexpr).opno,
                            *(*ioi).opfamily.add(indexcol as usize),
                        )
                    {
                        let ic: *mut pg_sys::IndexClause =
                            pg_sys::palloc0(std::mem::size_of::<pg_sys::IndexClause>()).cast();
                        (*ic).type_ = pg_sys::NodeTag::T_IndexClause;
                        (*ic).rinfo = ri;
                        let mut quals = PgList::<pg_sys::RestrictInfo>::new();
                        quals.push(ri);
                        (*ic).indexquals = quals.into_pg();
                        (*ic).lossy = false;
                        (*ic).indexcol = indexcol as _;
                        (*ic).indexcols = std::ptr::null_mut();
                        iclauses.push(ic);
                        break;
                    }
                }
            }

            if iclauses.is_empty() {
                continue;
            }

            pgrx::notice!(
                "[bitmap_harvest] directly built IndexPath over {} with {} clause(s)",
                rel_name((*ioi).indexoid),
                iclauses.len()
            );
            let ipath = pg_sys::create_index_path(
                root,
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
            let bhp = pg_sys::create_bitmap_heap_path(
                root,
                rel,
                ipath.cast(),
                (*rel).lateral_relids,
                1.0,
                0,
            );
            return Some(bhp);
        }
        None
    }
}

unsafe fn contain_var_clause_checked(node: *mut pg_sys::Node) -> bool {
    unsafe { pg_sys::contain_var_clause(node) }
}

/// SPIKE execution half: run the child BitmapIndexScan (planned from the harvested
/// path) via MultiExecProcNode and report what the TIDBitmap contains.
pub unsafe fn consume_and_report(child: *mut pg_sys::PlanState) {
    unsafe {
        let result = pg_sys::MultiExecProcNode(child);
        if result.is_null() {
            pgrx::notice!("[bitmap_harvest] MultiExecProcNode returned NULL");
            return;
        }
        assert_eq!(
            (*result).type_,
            pg_sys::NodeTag::T_TIDBitmap,
            "MultiExecProcNode must return a TIDBitmap"
        );
        let tbm: *mut pg_sys::TIDBitmap = result.cast();

        let mut pages = 0u64;
        let mut exact_tuples = 0u64;
        let mut lossy_pages = 0u64;
        let mut recheck_pages = 0u64;

        #[cfg(feature = "pg18")]
        {
            let iter = pg_sys::tbm_begin_private_iterate(tbm);
            let mut res = pg_sys::TBMIterateResult::default();
            let mut offsets = [0 as pg_sys::OffsetNumber; 1024];
            while pg_sys::tbm_private_iterate(iter, &mut res) {
                pages += 1;
                if res.lossy {
                    lossy_pages += 1;
                } else {
                    let n = pg_sys::tbm_extract_page_tuple(&mut res, offsets.as_mut_ptr(), 1024);
                    exact_tuples += n as u64;
                }
                if res.recheck {
                    recheck_pages += 1;
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
                pages += 1;
                if (*res).ntuples < 0 {
                    lossy_pages += 1;
                } else {
                    exact_tuples += (*res).ntuples as u64;
                }
                if (*res).recheck {
                    recheck_pages += 1;
                }
            }
            pg_sys::tbm_end_iterate(iter);
        }

        pgrx::notice!(
            "[bitmap_harvest] TIDBitmap: {pages} pages ({exact_tuples} exact tuples, \
             {lossy_pages} lossy pages, {recheck_pages} recheck pages)"
        );
        pg_sys::tbm_free(tbm);
    }
}
