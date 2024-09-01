use crate::customscan::scan::create_custom_scan_state;
use crate::customscan::{list, node, CustomScan};
use pgrx::list::List;
use pgrx::memcx::MemCx;
use pgrx::{node_to_string, pg_sys, PgList, PgMemoryContexts};
use std::ffi::c_void;
use std::fmt::{Debug, Formatter};

pub struct Args {
    pub root: *mut pg_sys::PlannerInfo,
    pub rel: *mut pg_sys::RelOptInfo,
    pub best_path: *mut pg_sys::CustomPath,
    pub tlist: PgList<pg_sys::TargetEntry>,
    pub clauses: *mut pg_sys::List,
    pub custom_plans: *mut pg_sys::List,
}

impl Debug for Args {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Args")
            .field("root", &self.root)
            .field("rel", &self.rel)
            .field("best_path", &self.best_path)
            .field(
                "tlist",
                &self
                    .tlist
                    .iter_ptr()
                    .map(|te| unsafe { node_to_string(te.cast()) }.unwrap_or("<null>"))
                    .collect::<Vec<_>>(),
            )
            .field("clauses", &self.clauses)
            .field("custom_plans", &self.custom_plans)
            .finish()
    }
}

impl Args {
    pub fn target_list(&self) -> impl Iterator<Item = Option<&pg_sys::TargetEntry>> + '_ {
        self.tlist.iter_ptr().map(|entry| unsafe { entry.as_ref() })
    }
}

pub struct CustomScanBuilder<'mcx> {
    mcx: &'mcx MemCx<'mcx>,
    args: Args,

    custom_scan_node: pg_sys::CustomScan,
    custom_paths: List<'mcx, *mut c_void>,
    custom_private: PgList<pg_sys::Node>,
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

        scan.flags = unsafe { (*best_path).flags };
        scan.scan.plan.type_ = pg_sys::NodeTag::T_CustomScan;
        scan.scan.plan.targetlist = tlist;

        scan.scan.scanrelid = unsafe { *rel }.relid;

        scan.custom_private = unsafe { *best_path }.custom_private;
        scan.custom_plans = custom_plans;
        scan.methods = PgMemoryContexts::CurrentMemoryContext.leak_and_drop_on_delete(
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
                tlist: unsafe { PgList::from_pg(tlist) },
                clauses,
                custom_plans,
            },
            custom_scan_node: scan,
            custom_paths: unsafe { list(mcx, custom_plans) },
            custom_private: unsafe { PgList::from_pg(scan.custom_private) },
        }
    }

    pub fn args(&self) -> &Args {
        &self.args
    }

    pub fn add_private_data(mut self, data: *mut pg_sys::Node) -> Self {
        self.custom_private.push(data);
        self
    }

    pub fn build(mut self) -> pg_sys::CustomScan {
        self.custom_scan_node
    }
}
