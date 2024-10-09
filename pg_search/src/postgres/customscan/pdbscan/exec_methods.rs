use crate::index::score::SearchIndexScore;
use crate::index::SearchIndex;
use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::pdbscan::{make_tuple_table_slot, PdbScan};
use pgrx::pg_sys;
use pgrx::pg_sys::TupleTableSlot;

pub enum ExecState {
    Found {
        scored: SearchIndexScore,
        slot: *mut TupleTableSlot,
    },
    Invisible {
        scored: SearchIndexScore,
    },
    Eof,
}

pub type NormalScanExecState = ();

#[inline(always)]
pub fn normal_scan_exec(
    state: &mut CustomScanStateWrapper<PdbScan>,
    _: *mut std::ffi::c_void,
) -> ExecState {
    match state.custom_state_mut().search_results.next() {
        None => ExecState::Eof,
        Some((scored, _)) => {
            let scanslot = state.scanslot();
            let bslot = state.scanslot() as *mut pg_sys::BufferHeapTupleTableSlot;

            match make_tuple_table_slot(state, &scored, bslot) {
                None => ExecState::Invisible { scored },
                Some(slot) => ExecState::Found { scored, slot },
            }
        }
    }
}

#[derive(Default, Debug)]
pub struct TopNScanExecState {
    last_ctid: u64,
    pub limit: usize,
    found: usize,
    chunk_size: usize,
}

#[inline(always)]
pub fn top_n_scan_exec(
    state: &mut CustomScanStateWrapper<PdbScan>,
    isc: *mut std::ffi::c_void,
) -> ExecState {
    unsafe {
        let isc = isc.cast::<TopNScanExecState>().as_mut().unwrap();

        let mut next = state.custom_state_mut().search_results.next();

        loop {
            match next {
                None => {
                    if isc.found == isc.limit {
                        // we found all the matching rows
                        pgrx::warning!("done: {isc:?}");
                        return ExecState::Eof;
                    }
                }
                Some((scored, _)) => {
                    let scanslot = state.scanslot();
                    let bslot = state.scanslot() as *mut pg_sys::BufferHeapTupleTableSlot;

                    isc.last_ctid = scored.ctid;

                    return match make_tuple_table_slot(state, &scored, bslot) {
                        None => {
                            pgrx::warning!("found invisible tuple");
                            ExecState::Invisible { scored }
                        }
                        Some(slot) => {
                            pgrx::warning!("found visible tuple");
                            isc.found += 1;
                            ExecState::Found { scored, slot }
                        }
                    };
                }
            }

            // we underflowed our tuples, so go get some more, if there are any

            // go ask for 2x as many as we got last time
            isc.chunk_size = (isc.chunk_size * 2).max(isc.limit * 2);
            pgrx::warning!("going to get {} more rows", isc.chunk_size);
            let mut results = state
                .custom_state()
                .search_reader
                .as_ref()
                .unwrap()
                .search_top_n(
                    SearchIndex::executor(),
                    state.custom_state().query.as_ref().unwrap(),
                    state
                        .custom_state()
                        .sort_direction
                        .as_ref()
                        .cloned()
                        .unwrap()
                        .into(),
                    isc.chunk_size,
                );

            // fast forward and stop on the ctid we last found
            while let Some((scored, _)) = results.next() {
                if scored.ctid == isc.last_ctid {
                    // we've now advanced to the last ctid we found
                    break;
                }
            }

            // this should be the next valid tuple after that
            next = match results.next() {
                // ... and there it is!
                Some(next) => Some(next),

                // there wasn't one, so we've now read all possible matches
                None => return ExecState::Eof,
            };

            // we now have a new iterator of results to use going forward
            state.custom_state_mut().search_results = results;

            // but we'll loop back around and evaluate whatever `next` is now pointing to
            continue;
        }
    }
}
