use crate::customscan::CustomScan;
use pgrx::{pg_sys, PgList, PgMemoryContexts};
use std::fmt::{Debug, Formatter};

pub struct Args {
    pub cscan: *mut pg_sys::CustomScan,
}

#[derive(Default)]
#[repr(C)]
pub struct CustomScanStateWrapper<CS: CustomScan> {
    pub csstate: pg_sys::CustomScanState,
    pub custom_state: CS::State,
}

impl<CS: CustomScan> Debug for CustomScanStateWrapper<CS>
where
    CS::State: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(&format!(
            "CustomScanStateWrapper<{}>",
            std::any::type_name::<CS>()
        ))
        .field("state_type", &std::any::type_name::<CS::State>())
        .field("csstate", &self.csstate)
        .field("custom_state", &self.custom_state)
        .finish()
    }
}

impl<CS: CustomScan> CustomScanStateWrapper<CS> {
    #[inline(always)]
    pub fn scanslot(&self) -> *mut pg_sys::TupleTableSlot {
        self.csstate.ss.ss_ScanTupleSlot
    }

    #[inline(always)]
    pub fn projection_info(&self) -> *mut pg_sys::ProjectionInfo {
        self.csstate.ss.ps.ps_ProjInfo
    }

    #[inline(always)]
    pub fn set_projection_slot(&mut self, slot: *mut pg_sys::TupleTableSlot) {
        unsafe { (*(*self.csstate.ss.ps.ps_ProjInfo).pi_exprContext).ecxt_scantuple = slot }
    }
}

pub struct CustomScanStateBuilder<CS: CustomScan> {
    args: Args,

    custom_state: CS::State,
}

impl<CS: CustomScan> CustomScanStateBuilder<CS> {
    pub fn new(cscan: *mut pg_sys::CustomScan) -> Self {
        Self {
            args: Args { cscan },

            custom_state: CS::State::default(),
        }
    }

    pub fn args(&self) -> &Args {
        &self.args
    }

    pub fn private_data(&self) -> PgList<pg_sys::Node> {
        unsafe { PgList::from_pg((*self.args.cscan).custom_private) }
    }

    pub fn custom_state(&mut self) -> &mut CS::State {
        &mut self.custom_state
    }

    pub fn build(self) -> CustomScanStateWrapper<CS> {
        let mut scan_state = pg_sys::ScanState::default();

        scan_state.ps = pg_sys::PlanState::default();
        scan_state.ps.type_ = pg_sys::NodeTag::T_CustomScanState;

        let mut wrapper = CustomScanStateWrapper {
            csstate: pg_sys::CustomScanState {
                ss: scan_state,
                flags: 0,
                custom_ps: std::ptr::null_mut(),
                pscan_len: 0,
                methods: PgMemoryContexts::CurrentMemoryContext
                    .leak_and_drop_on_delete(CS::exec_methods()),
                slotOps: std::ptr::null_mut(),
            },
            custom_state: self.custom_state,
        };

        wrapper
    }
}
