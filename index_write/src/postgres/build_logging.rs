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

//! Targeted logging bridge for IVF build timings and synchronous merge phases.

use crate::postgres::merge_progress::{MERGER_LOG_TARGET, report_record};
use log::{Level, LevelFilter, Log, Metadata, Record, set_logger, set_max_level};
use std::sync::OnceLock;

const IVF_BUILD_TARGET: &str = "paradedb::ivf_build";

struct IndexLogger {
    fallback: Option<Box<dyn Log>>,
}

impl Log for IndexLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        (metadata.target() == IVF_BUILD_TARGET && metadata.level() <= Level::Info)
            || (metadata.target() == MERGER_LOG_TARGET && metadata.level() == Level::Debug)
            || self
                .fallback
                .as_ref()
                .is_some_and(|logger| logger.enabled(metadata))
    }

    fn log(&self, record: &Record) {
        report_record(record);
        if let Some(logger) = &self.fallback {
            if logger.enabled(record.metadata()) {
                logger.log(record);
            }
        } else if record.target() == IVF_BUILD_TARGET && record.level() <= Level::Info {
            pgrx::notice!("{}", record.args());
        }
    }

    fn flush(&self) {
        if let Some(logger) = &self.fallback {
            logger.flush();
        }
    }
}

static INDEX_LOGGER: OnceLock<IndexLogger> = OnceLock::new();

/// Retain optional dependency logging while observing merge records on the backend thread.
pub fn init() {
    init_inner(None);
}

pub fn init_with_logger(logger: impl Log + 'static) {
    init_inner(Some(Box::new(logger)));
}

fn init_inner(fallback: Option<Box<dyn Log>>) {
    let logger = INDEX_LOGGER.get_or_init(|| IndexLogger { fallback });
    if set_logger(logger).is_ok() {
        set_max_level(LevelFilter::Info);
    }
}
