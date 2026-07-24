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

//! Minimal, targeted `log` → Postgres NOTICE bridge for IVF build timings.
//!
//! tantivy's IVF build emits one `log::info!` line per vector field on the
//! target `paradedb::ivf_build`. This forwards ONLY that target to
//! `pgrx::notice!`, so the timings surface during `CREATE INDEX` and are
//! captured by NOTICE-reading clients (e.g. the benchmark harness via
//! psycopg). Every other record is rejected at `enabled()`, so this never acts
//! as a general logger or reconfigures logging broadly. The IVF build runs
//! synchronously in the backend during `CREATE INDEX`, so emitting via
//! `ereport` (NOTICE) here is on the backend thread.

const IVF_BUILD_TARGET: &str = "paradedb::ivf_build";

struct IvfBuildLogger;

impl log::Log for IvfBuildLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.target() == IVF_BUILD_TARGET && metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            pgrx::notice!("{}", record.args());
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
