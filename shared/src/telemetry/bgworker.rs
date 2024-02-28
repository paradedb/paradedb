use pgrx::bgworkers::{BackgroundWorker, BackgroundWorkerBuilder, SignalWakeFlags};
use pgrx::*;
use std::ffi::CStr;
use std::process;
use std::time::{Duration, Instant};
use std::thread;

#[pg_guard]
pub fn setup_telemetry_background_worker(extension_name: String) {
    // A background worker to read and send telemetry data to PostHog.
    BackgroundWorkerBuilder::new(&format!("{}_telemetry_worker", &extension_name))
        // Must be the name of a function in this file.
        .set_function("telemetry_worker")
        // Must be the name of the extension it will be loaded from.
        .set_library(&extension_name)
        // We pass the extension name to retrieve the associated data directory to read telemetry data from.
        .set_argument(extension_name.into_datum())
        // Necessary for using plog!.
        // Also, it doesn't seem like bgworkers will start without this.
        .enable_spi_access()
        // RecoveryFinished is the last available stage for bgworker startup.
        // We wait until as late as possible so that we can make sure the
        // paradedb.logs table is created, for the sake of using plog!.
        .set_start_time(bgworkers::BgWorkerStartTime::RecoveryFinished)
        .load();
}

#[pg_guard]
#[no_mangle]
pub extern "C" fn telemetry_worker(arg: pg_sys::Datum) {


    unsafe {
        // Convert Datum to CString
        let text_ptr = pg_sys::pg_detoast_datum(arg.cast_mut_ptr::<pg_sys::varlena>()) as *mut pg_sys::text;
        let c_str = CStr::from_ptr(pg_sys::text_to_cstring(text_ptr));

        // Convert CStr to Rust String
        let rust_string = match c_str.to_str() {
            Ok(s) => s.to_owned(),
            Err(e) => panic!("Failed to convert to string: {:?}", e),
        };

        // Use rust_string as needed
        println!("Extracted string: {}", rust_string);
    
    pgrx::log!("starting {} telemetry reader worker at PID {}", rust_string, process::id());
    // These are the signals we want to receive. If we don't attach the SIGTERM handler, then
    // we'll never be able to exit via an external notification.
    BackgroundWorker::attach_signal_handlers(SignalWakeFlags::SIGTERM);

    let start_time = Instant::now();
    let wait_duration = Duration::from_secs(12 * 3600); // 12 hours

    loop {
        // Check if the wait duration has elapsed
        if Instant::now().duration_since(start_time) >= wait_duration {
            // Perform your telemetry sending logic here

            break;
        }

        // Sleep for a short period to remain responsive
        thread::sleep(Duration::from_secs(1));

        pgrx::log!("telemetry worker is still running");

        if BackgroundWorker::sigterm_received() {
            pgrx::log!("insert worker received sigterm, shutting down");
            return; // Exit the worker
        }
    }
}
}
