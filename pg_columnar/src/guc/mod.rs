use pgrx::*;

// Initialize extension-specific GUC settings.
pub static PARADE_GUC: ParadeGUC = ParadeGUC::new();

// Vacuum retention days
const DEFAULT_VACUUM_RETENTION_DAYS: i32 = 7;
const MIN_VACUUM_RETENTION_DAYS: i32 = 0;
const MAX_VACUUM_RETENTION_DAYS: i32 = 365;

// Vacuum enforce retention
const DEFAULT_VACUUM_ENFORCE_RETENTION: bool = true;

// Optimize target file size
const DEFAULT_OPTIMIZE_FILE_SIZE_MB: i32 = 100;
const MIN_OPTIMIZE_FILE_SIZE_MB: i32 = 1;
const MAX_OPTIMIZE_FILE_SIZE_MB: i32 = 10000;

pub struct ParadeGUC {
    pub vacuum_retention_days: GucSetting<i32>,
    pub vacuum_enforce_retention: GucSetting<bool>,
    pub optimize_file_size_mb: GucSetting<i32>,
}

impl ParadeGUC {
    pub const fn new() -> Self {
        Self {
            vacuum_retention_days: GucSetting::<i32>::new(DEFAULT_VACUUM_RETENTION_DAYS),
            vacuum_enforce_retention: GucSetting::<bool>::new(DEFAULT_VACUUM_ENFORCE_RETENTION),
            optimize_file_size_mb: GucSetting::<i32>::new(DEFAULT_OPTIMIZE_FILE_SIZE_MB),
        }
    }

    /// You must call this `init` function in the extension's `_PG_init()`.
    /// Make sure you've first called `ParadeGUC::new()` into a static variable.
    /// Example in _PG_init():
    /// ```
    /// PARADE_GUC::init();
    /// ```
    pub fn init(&self) {
        GucRegistry::define_int_guc(
            "paradedb.vacuum_retention_days",
            "Only vacuum data older than this many days.",
            "Entries younger than this will not be vacuumed. Defaults to 7 days.",
            &self.vacuum_retention_days,
            MIN_VACUUM_RETENTION_DAYS,
            MAX_VACUUM_RETENTION_DAYS,
            GucContext::Userset,
            GucFlags::default(),
        );

        GucRegistry::define_bool_guc(
            "paradedb.vacuum_enforce_retention",
            "If set to true, vacuums fail if the specified vacuum_retention_days is less than 7 days.",
            "Defaults to true.",
            &self.vacuum_enforce_retention,
            GucContext::Userset,
            GucFlags::default(),
        );

        GucRegistry::define_int_guc(
            "paradedb.optimize_file_size_mb",
            "The target file size, in MB, when optimizing a table by merging small Parquet files into a large file.",
            "Defaults to 100.",
            &self.optimize_file_size_mb,
            MIN_OPTIMIZE_FILE_SIZE_MB,
            MAX_OPTIMIZE_FILE_SIZE_MB,
            GucContext::Userset,
            GucFlags::default(),
        );
    }
}
