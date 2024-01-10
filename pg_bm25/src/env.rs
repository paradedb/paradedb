use once_cell::sync::Lazy;
use std::{ffi::CStr, path::PathBuf, sync::Mutex};

/// We use this global variable to cache any values that can be re-used
/// after initialization.
static PARADE_ENV: Lazy<ParadeEnv> = Lazy::new(|| ParadeEnv {
    postgres_data_dir: Mutex::new(None),
});

struct ParadeEnv {
    postgres_data_dir: Mutex<Option<PathBuf>>,
}

pub fn postgres_data_dir_path() -> PathBuf {
    PARADE_ENV
        .postgres_data_dir
        .lock()
        .expect("Failed to lock mutex")
        .get_or_insert_with(|| unsafe {
            let data_dir = CStr::from_ptr(pgrx::pg_sys::DataDir)
                .to_string_lossy()
                .into_owned();
            PathBuf::from(data_dir)
        })
        .clone()
}

pub fn paradedb_data_dir_path() -> PathBuf {
    postgres_data_dir_path().join("paradedb")
}

pub fn paradedb_transfer_pipe_path() -> PathBuf {
    paradedb_data_dir_path().join("writer_transfer")
}
