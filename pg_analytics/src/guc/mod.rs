use pgrx::*;
use shared::gucs::{GlobalGucSettings, PostgresGlobalGucSettings};

// Initialize extension-specific GUC settings.
pub static PARADE_GUC: PostgresPgAnalyticsGucSettings = PostgresPgAnalyticsGucSettings::new();

// Vacuum retention days
const DEFAULT_VACUUM_RETENTION_DAYS: i32 = 0;
const MIN_VACUUM_RETENTION_DAYS: i32 = 0;
const MAX_VACUUM_RETENTION_DAYS: i32 = 365;

// Vacuum enforce retention
const DEFAULT_VACUUM_ENFORCE_RETENTION: bool = false;

// Optimize target file size
const DEFAULT_OPTIMIZE_FILE_SIZE_MB: i32 = 100;
const MIN_OPTIMIZE_FILE_SIZE_MB: i32 = 1;
const MAX_OPTIMIZE_FILE_SIZE_MB: i32 = 10000;

#[allow(dead_code)]
trait PgAnalyticsGucSettings {
    fn vacuum_retention_days(&self) -> i32;
    fn vacuum_enforce_retention(&self) -> bool;
    fn optimize_file_size_mb(&self) -> i32;
}

pub struct PostgresPgAnalyticsGucSettings {
    pub vacuum_retention_days: GucSetting<i32>,
    pub vacuum_enforce_retention: GucSetting<bool>,
    pub optimize_file_size_mb: GucSetting<i32>,
    pub globals: PostgresGlobalGucSettings,
}

impl Default for PostgresPgAnalyticsGucSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl PostgresPgAnalyticsGucSettings {
    pub const fn new() -> Self {
        Self {
            vacuum_retention_days: GucSetting::<i32>::new(DEFAULT_VACUUM_RETENTION_DAYS),
            vacuum_enforce_retention: GucSetting::<bool>::new(DEFAULT_VACUUM_ENFORCE_RETENTION),
            optimize_file_size_mb: GucSetting::<i32>::new(DEFAULT_OPTIMIZE_FILE_SIZE_MB),
            globals: PostgresGlobalGucSettings::new(),
        }
    }

    /// You must call this `init` function in the extension's `_PG_init()`.
    /// Make sure you've first called `ParadeGUC::new()` into a static variable.
    pub fn init(&self, extension_name: &str) {
        // Initialize global settings first.
        self.globals.init(extension_name);

        GucRegistry::define_int_guc(
            "parade.vacuum_retention_days",
            "Only vacuum data older than this many days.",
            "Entries younger than this will not be vacuumed. Defaults to 7 days.",
            &self.vacuum_retention_days,
            MIN_VACUUM_RETENTION_DAYS,
            MAX_VACUUM_RETENTION_DAYS,
            GucContext::Userset,
            GucFlags::default(),
        );

        GucRegistry::define_bool_guc(
            "parade.vacuum_enforce_retention",
            "If set to true, vacuums fail if the specified vacuum_retention_days is less than 7 days.",
            "Defaults to true.",
            &self.vacuum_enforce_retention,
            GucContext::Userset,
            GucFlags::default(),
        );

        GucRegistry::define_int_guc(
            "parade.optimize_file_size_mb",
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

impl GlobalGucSettings for PostgresPgAnalyticsGucSettings {
    fn telemetry_enabled(&self) -> bool {
        self.globals.telemetry_enabled()
    }

    fn logs_enabled(&self) -> bool {
        self.globals.logs_enabled()
    }
}

impl PgAnalyticsGucSettings for PostgresPgAnalyticsGucSettings {
    fn vacuum_retention_days(&self) -> i32 {
        self.vacuum_retention_days.get()
    }
    fn vacuum_enforce_retention(&self) -> bool {
        self.vacuum_enforce_retention.get()
    }
    fn optimize_file_size_mb(&self) -> i32 {
        self.optimize_file_size_mb.get()
    }
}
