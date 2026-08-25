// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

//! Minimal, targeted `log` → Postgres bridge for IVF build timings and
//! quantization-calibration stability.
//!
//! tantivy's IVF build emits one `log::info!` line per vector field on the
//! target `paradedb::ivf_build`. This forwards that target to `pgrx::notice!`,
//! so the timings surface during `CREATE INDEX` and are captured by
//! NOTICE-reading clients (e.g. the benchmark harness via psycopg).
//! `paradedb::quantization_calibration` carries caller-query stability records
//! and is forwarded to the server LOG: the record remains Info-level in Rust,
//! while nondeterministic measurements do not pollute SQL result streams.
//! Every other record is rejected at `enabled()`, so this never acts as a
//! general logger or reconfigures logging broadly. Both producers run
//! synchronously on the backend thread, where calling Postgres logging is safe.

const IVF_BUILD_TARGET: &str = "paradedb::ivf_build";
pub(crate) const QUANTIZATION_CALIBRATION_TARGET: &str = "paradedb::quantization_calibration";

struct IvfBuildLogger;

impl log::Log for IvfBuildLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        matches!(
            metadata.target(),
            IVF_BUILD_TARGET | QUANTIZATION_CALIBRATION_TARGET
        ) && metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        match record.target() {
            IVF_BUILD_TARGET => pgrx::notice!("{}", record.args()),
            QUANTIZATION_CALIBRATION_TARGET => pgrx::log!("{}", record.args()),
            _ => unreachable!("enabled rejects every other logging target"),
        }
    }

    fn flush(&self) {}
}

static IVF_BUILD_LOGGER: IvfBuildLogger = IvfBuildLogger;

/// Install the bridge. Idempotent and non-fatal: if another logger is already
/// registered (e.g. `env_logger` under the `debug-logging` feature) we leave it
/// in place. On success we raise the max level to Info so the targeted record
/// is evaluated — non-matching records are still dropped at `enabled()`.
pub fn init() {
    if log::set_logger(&IVF_BUILD_LOGGER).is_ok() {
        log::set_max_level(log::LevelFilter::Info);
    }
}
