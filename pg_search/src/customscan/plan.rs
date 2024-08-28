use pgrx::{pg_guard, pg_sys};

/// Allocate a CustomScanState for this CustomScan. The actual allocation will often be larger than
/// required for an ordinary CustomScanState, because many providers will wish to embed that as the
/// first field of a larger structure. The value returned must have the node tag and methods set
/// appropriately, but other fields should be left as zeroes at this stage; after ExecInitCustomScan
/// performs basic initialization, the BeginCustomScan callback will be invoked to give the custom
/// scan provider a chance to do whatever else is needed.
#[pg_guard]
pub extern "C" fn create_custom_scan_state(cscan: *mut pg_sys::CustomScan) -> *mut pg_sys::Node {
    todo!("create_custom_scan_state")
}
