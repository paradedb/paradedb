use crate::customscan::scan::create_custom_scan_state;
use crate::customscan::{list, CustomScan};
use pgrx::list::List;
use pgrx::memcx::MemCx;
use pgrx::{pg_sys, PgMemoryContexts};
use std::ffi::c_void;

#[derive(Debug)]
pub struct Args {
    root: *mut pg_sys::PlannerInfo,
    rel: *mut pg_sys::RelOptInfo,
    best_path: *mut pg_sys::CustomPath,
    tlist: *mut pg_sys::List,
    clauses: *mut pg_sys::List,
    custom_plans: *mut pg_sys::List,
}

pub struct CustomScanBuilder<'mcx> {
    mcx: &'mcx MemCx<'mcx>,
    args: Args,

    custom_scan_node: pg_sys::CustomScan,
    custom_paths: List<'mcx, *mut c_void>,
}

impl<'mcx> CustomScanBuilder<'mcx> {
    pub fn new<CS: CustomScan>(
        mcx: &'mcx MemCx<'mcx>,
        root: *mut pg_sys::PlannerInfo,
        rel: *mut pg_sys::RelOptInfo,
        best_path: *mut pg_sys::CustomPath,
        tlist: *mut pg_sys::List,
        clauses: *mut pg_sys::List,
        custom_plans: *mut pg_sys::List,
    ) -> Self {
        let mut scan = pg_sys::CustomScan::default();

        scan.scan.plan.type_ = pg_sys::NodeTag::T_CustomScan;
        scan.scan.plan.targetlist = tlist;

        scan.scan.scanrelid = unsafe { *rel }.relid;

        scan.custom_plans = custom_plans;
        scan.methods = PgMemoryContexts::For(unsafe { *root }.planner_cxt).leak_and_drop_on_delete(
            pg_sys::CustomScanMethods {
                CustomName: CS::NAME.as_ptr(),
                CreateCustomScanState: Some(create_custom_scan_state::<CS>),
            },
        );

        CustomScanBuilder {
            mcx,
            args: Args {
                root,
                rel,
                best_path,
                tlist,
                clauses,
                custom_plans,
            },
            custom_scan_node: scan,
            custom_paths: unsafe { list(mcx, custom_plans) },
        }
    }

    pub fn args(&self) -> &Args {
        &self.args
    }

    pub fn build(mut self) -> pg_sys::CustomScan {
        self.custom_scan_node
    }
}
