use pgrx::{pg_guard, pg_sys};

/// Convert a custom path to a finished plan. The return value will generally be a CustomScan object,
/// which the callback must allocate and initialize. See Section 61.2 for more details.
#[pg_guard]
pub extern "C" fn plan_custom_path(
    root: *mut pg_sys::PlannerInfo,
    rel: *mut pg_sys::RelOptInfo,
    best_path: *mut pg_sys::CustomPath,
    tlist: *mut pg_sys::List,
    clauses: *mut pg_sys::List,
    custom_plans: *mut pg_sys::List,
) -> *mut pg_sys::Plan {
    todo!("plan_custom_path")
}

/// This callback is called while converting a path parameterized by the top-most parent of the
/// given child relation child_rel to be parameterized by the child relation. The callback is used
/// to reparameterize any paths or translate any expression nodes saved in the given custom_private
/// member of a CustomPath. The callback may use reparameterize_path_by_child,
/// adjust_appendrel_attrs or adjust_appendrel_attrs_multilevel as required.
#[pg_guard]
pub extern "C" fn reparameterize_custom_path_by_child(
    root: *mut pg_sys::PlannerInfo,
    custom_prive: *mut pg_sys::List,
    child_rel: *mut pg_sys::RelOptInfo,
) -> *mut pg_sys::List {
    todo!("reparameterize_custom_path_by_child")
}
