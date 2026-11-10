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
#![allow(unsafe_op_in_unsafe_fn)]

use pgrx::*;

#[cfg(not(any(feature = "pg17", feature = "pg18")))]
pub use index_write::postgres::fake_aminsertcleanup;
#[cfg(any(test, feature = "pg_test"))]
pub use index_write::postgres::merge;
pub use index_write::postgres::{build, build_logging, delete, insert, mutable_sweep, vacuum};
pub use planner::postgres::{
    ParallelScanState, ScanStrategy, access, catalog, composite, cost, datetime, heap, index, node,
    options, parallel, pdb_owned_value, pgstat, planner_warnings, rel, rel_get_bm25_index,
    search_operator_relations, storage, types, utils, validate, var,
};

#[cfg(any(test, feature = "pg_test"))]
pub mod coordination_test;
pub mod customscan;
pub(crate) mod index_only;
mod scan;
pub(crate) mod sequentialscan;

#[pg_extern(sql = "
CREATE FUNCTION bm25_handler(internal) RETURNS index_am_handler PARALLEL SAFE IMMUTABLE STRICT COST 0.0001 LANGUAGE c AS 'MODULE_PATHNAME', '@FUNCTION_NAME@';
CREATE ACCESS METHOD paradedb TYPE INDEX HANDLER bm25_handler;
CREATE ACCESS METHOD bm25 TYPE INDEX HANDLER bm25_handler;
COMMENT ON ACCESS METHOD bm25 IS 'backwards-compatible alias for the paradedb index access method';
")]
fn bm25_handler(_fcinfo: pg_sys::FunctionCallInfo) -> PgBox<pg_sys::IndexAmRoutine> {
    let mut amroutine =
        unsafe { PgBox::<pg_sys::IndexAmRoutine>::alloc_node(pg_sys::NodeTag::T_IndexAmRoutine) };

    amroutine.amstrategies = 2;
    amroutine.amsupport = 0;
    amroutine.amcanmulticol = true;
    amroutine.amoptionalkey = true;
    amroutine.amsearcharray = true;

    amroutine.amkeytype = pg_sys::InvalidOid;

    amroutine.amvalidate = Some(validate::amvalidate);
    amroutine.ambuild = Some(build::ambuild);
    amroutine.ambuildempty = Some(build::ambuildempty);
    amroutine.aminsert = Some(insert::aminsert);
    #[cfg(any(feature = "pg17", feature = "pg18"))]
    {
        amroutine.aminsertcleanup = Some(insert::aminsertcleanup);
        amroutine.amcanbuildparallel = true;
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
    amroutine.amcanreturn = Some(index_only::amcanreturn);

    amroutine.amcanparallel = true;
    amroutine.aminitparallelscan = Some(parallel::aminitparallelscan);
    amroutine.amestimateparallelscan = Some(parallel::amestimateparallelscan);
    amroutine.amparallelrescan = Some(parallel::amparallelrescan);

    amroutine.into_pg_boxed()
}

#[pg_extern]
unsafe fn fsm_info(
    index: access::IndexOid,
) -> iter::TableIterator<
    'static,
    (
        name!(xid, pg_sys::TransactionId),
        name!(fsm_blockno, i64),
        name!(tag, i64),
        name!(free_blockno, i64),
    ),
> {
    storage::fsm::fsm_info(index)
}

#[pg_extern]
unsafe fn fsm_size(index: access::IndexOid) -> i64 {
    storage::fsm::fsm_size(index)
}

#[allow(unused_variables)]
#[pg_extern]
unsafe fn reset_bgworker_state(index: PgRelation) {
    storage::metadata::reset_bgworker_state(index)
}

#[allow(unused_variables)]
#[pg_extern]
unsafe fn bgmerger_state(
    index: PgRelation,
) -> iter::TableIterator<'static, (name!(pid, i32), name!(state, String))> {
    storage::metadata::bgmerger_state(index)
}
