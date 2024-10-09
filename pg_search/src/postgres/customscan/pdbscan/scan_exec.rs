use crate::index::score::SearchIndexScore;
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
    EOF,
}

#[inline(always)]
pub fn normal_scan_exec(state: &mut CustomScanStateWrapper<PdbScan>) -> ExecState {
    match state.custom_state_mut().search_results.next() {
        None => ExecState::EOF,
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

#[inline(always)]
pub fn top_n_scan_exec(state: &mut CustomScanStateWrapper<PdbScan>) -> ExecState {
    match state.custom_state_mut().search_results.next() {
        None => ExecState::EOF,
        Some((scored, _)) => {
            let scanslot = state.scanslot();
            let bslot = state.scanslot() as *mut pg_sys::BufferHeapTupleTableSlot;

            match make_tuple_table_slot(state, &scored, bslot) {
                // TODO:  do something entirely different if the tuple is invisible to us
                None => {
                    pgrx::warning!("HERE");
                    ExecState::Invisible { scored }
                }
                Some(slot) => ExecState::Found { scored, slot },
            }
        }
    }
}
