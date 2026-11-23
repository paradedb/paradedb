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

//! Route build notices, merge progress, and optional dependency logs.

use crate::postgres::merge_progress::MergeProgressLogger;
use log::{Level, LevelFilter, Log, Metadata, Record, set_logger, set_max_level};
use std::sync::OnceLock;

const IVF_BUILD_TARGET: &str = "paradedb::ivf_build";

struct IvfBuildLogger;

impl Log for IvfBuildLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.target() == IVF_BUILD_TARGET && metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            pgrx::notice!("{}", record.args());
        }
    }

    fn flush(&self) {}
}

struct IndexLogger {
    loggers: Vec<Box<dyn Log>>,
}

impl Log for IndexLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.loggers.iter().any(|logger| logger.enabled(metadata))
    }

    fn log(&self, record: &Record) {
        for logger in &self.loggers {
            if logger.enabled(record.metadata()) {
                logger.log(record);
            }
        }
    }

    fn flush(&self) {
        for logger in &self.loggers {
            logger.flush();
        }
    }
}

static INDEX_LOGGER: OnceLock<IndexLogger> = OnceLock::new();

pub fn init() {
    init_with_logger(IvfBuildLogger);
}

pub fn init_with_logger(logger: impl Log + 'static) {
    let logger = INDEX_LOGGER.get_or_init(|| IndexLogger {
        loggers: vec![Box::new(MergeProgressLogger), Box::new(logger)],
    });
    if set_logger(logger).is_ok() {
        set_max_level(LevelFilter::Info);
    }
}
