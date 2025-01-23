// Copyright (c) 2023-2025 Retake, Inc.
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

use crate::index::reader::index::SearchIndexReader;
use crate::postgres::parallel::Spinlock;
use pgrx::*;
use std::ptr::{addr_of, addr_of_mut};
use tantivy::index::SegmentId;

mod build;
mod cost;
mod delete;
mod insert;
pub mod options;
mod range;
mod scan;
mod vacuum;
mod validate;

pub mod customscan;
pub mod datetime;
#[cfg(not(feature = "pg17"))]
pub mod fake_aminsertcleanup;
pub mod index;
mod parallel;
pub mod storage;
pub mod types;
pub mod utils;
pub mod visibility_checker;

#[repr(u16)] // b/c that's what [`pg_sys::StrategyNumber`] is
pub enum ScanStrategy {
    TextQuery = 1,
    SearchQueryInput = 2,
    // NB:  Any additions here **mut** update the `amroutine.amstrategies` down below in [`bm25_handler`]
}

impl TryFrom<pg_sys::StrategyNumber> for ScanStrategy {
    type Error = String;

    fn try_from(value: pg_sys::StrategyNumber) -> Result<Self, Self::Error> {
        if value == 1 {
            Ok(ScanStrategy::TextQuery)
        } else if value == 2 {
            Ok(ScanStrategy::SearchQueryInput)
        } else {
            Err(format!("`{value}` is an unknown `ScanStrategy` number"))
        }
    }
}

#[pg_extern(sql = "
CREATE FUNCTION bm25_handler(internal) RETURNS index_am_handler PARALLEL SAFE IMMUTABLE STRICT COST 0.0001 LANGUAGE c AS 'MODULE_PATHNAME', '@FUNCTION_NAME@';
CREATE ACCESS METHOD bm25 TYPE INDEX HANDLER bm25_handler;
COMMENT ON ACCESS METHOD bm25 IS 'bm25 index access method';
")]
fn bm25_handler(_fcinfo: pg_sys::FunctionCallInfo) -> PgBox<pg_sys::IndexAmRoutine> {
    let mut amroutine =
        unsafe { PgBox::<pg_sys::IndexAmRoutine>::alloc_node(pg_sys::NodeTag::T_IndexAmRoutine) };

    amroutine.amstrategies = 2;
    amroutine.amsupport = 0;
    amroutine.amcanmulticol = true;
    amroutine.amsearcharray = true;

    amroutine.amkeytype = pg_sys::InvalidOid;

    amroutine.amvalidate = Some(validate::amvalidate);
    amroutine.ambuild = Some(build::ambuild);
    amroutine.ambuildempty = Some(build::ambuildempty);
    amroutine.aminsert = Some(insert::aminsert);
    #[cfg(feature = "pg17")]
    {
        amroutine.aminsertcleanup = Some(insert::aminsertcleanup);
    }
    amroutine.ambulkdelete = Some(delete::ambulkdelete);
    amroutine.amvacuumcleanup = Some(vacuum::amvacuumcleanup);
    amroutine.amcostestimate = Some(cost::amcostestimate);
    amroutine.amoptions = Some(options::amoptions);
    amroutine.ambeginscan = Some(scan::ambeginscan);
    amroutine.amrescan = Some(scan::amrescan);
    amroutine.amgettuple = Some(scan::amgettuple);
    amroutine.amgetbitmap = Some(scan::amgetbitmap);
    amroutine.amendscan = Some(scan::amendscan);
    amroutine.amcanreturn = Some(scan::amcanreturn);

    amroutine.amcanparallel = true;
    amroutine.aminitparallelscan = Some(parallel::aminitparallelscan);
    amroutine.amestimateparallelscan = Some(parallel::amestimateparallelscan);
    amroutine.amparallelrescan = Some(parallel::amparallelrescan);

    amroutine.into_pg_boxed()
}

pub fn rel_get_bm25_index(relid: pg_sys::Oid) -> Option<(PgRelation, PgRelation)> {
    unsafe {
        let rel = PgRelation::with_lock(relid, pg_sys::AccessShareLock as _);
        for index in rel.indices(pg_sys::AccessShareLock as _) {
            if (*index.rd_indam).ambuild == Some(build::ambuild) {
                return Some((rel, index));
            }
        }
        None
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct ParallelScanState {
    pub mutex: Spinlock,
    pub segment_count: usize,
    pub remaining_segments: usize,
    pub segment_uuids: [u8; 0], // dynamically sized, allocated after end
}

impl ParallelScanState {
    #[inline]
    const fn size_of_with_segments(nsegments: usize) -> usize {
        // a SegmentId, in byte form, is 16 bytes
        size_of::<Self>() + (nsegments * 16)
    }

    unsafe fn get_segment_id(&self, i: usize) -> SegmentId {
        unsafe {
            if i >= self.segment_count {
                panic!(
                    "index {i} out of bounds.  segment_count={}",
                    self.segment_count
                );
            }
            let ptr = addr_of!(self.segment_uuids) as *mut [u8; 16];
            SegmentId::from_bytes(ptr.add(i).read())
        }
    }

    unsafe fn assign_segment_ids(&mut self, search_index_reader: &SearchIndexReader) {
        self.segment_count = search_index_reader.segment_readers().len();
        self.remaining_segments = self.segment_count;

        for (i, segment_reader) in search_index_reader.segment_readers().iter().enumerate() {
            self.set_segment_id(i, segment_reader.segment_id());
        }
    }

    unsafe fn set_segment_id(&mut self, i: usize, segment_id: SegmentId) {
        unsafe {
            if i >= self.segment_count {
                panic!(
                    "segment ordinal is out of bounds.  i={i}, segment_count={}",
                    self.segment_count
                );
            } else if i >= u16::MAX as usize {
                panic!(
                    "index has more than {} segments, which is not supported by parallel scans",
                    u16::MAX
                );
            }

            let ptr = addr_of_mut!(self.segment_uuids) as *mut [u8; 16];
            ptr.add(i).write(*segment_id.uuid_bytes())
        }
    }
}
