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

use datafusion_common::DataFusionError;
use datafusion_execution::memory_pool::{MemoryPool, MemoryReservation};

/// A memory pool that panics when the memory limit is exceeded.
///
/// This is used to enforce `work_mem` limits in `JoinScan` and prevent
/// DataFusion from attempting to spill to disk (which is not yet implemented safely).
///
/// TODO: Instead of panicking, implement a `MemoryPool` that integrates with PostgreSQL's
/// temporary file management (BufFile/VFD) to allow DataFusion to spill to disk when
/// `work_mem` is exceeded.
#[derive(Debug)]
pub struct PanicOnOOMMemoryPool {
    pool: datafusion_execution::memory_pool::GreedyMemoryPool,
    limit: usize,
}

impl PanicOnOOMMemoryPool {
    pub fn new(limit: usize) -> Self {
        Self {
            pool: datafusion_execution::memory_pool::GreedyMemoryPool::new(limit),
            limit,
        }
    }

    fn check_limit(&self, additional: usize) {
        if self.pool.reserved() + additional > self.limit {
            panic!(
                "JoinScan: Out of memory! Query exceeded work_mem limit of {} bytes.",
                self.limit
            );
        }
    }
}

impl MemoryPool for PanicOnOOMMemoryPool {
    fn grow(&self, reservation: &MemoryReservation, additional: usize) {
        self.check_limit(additional);
        self.pool.grow(reservation, additional);
    }

    fn shrink(&self, reservation: &MemoryReservation, returned: usize) {
        self.pool.shrink(reservation, returned);
    }

    fn try_grow(
        &self,
        reservation: &MemoryReservation,
        additional: usize,
    ) -> Result<(), DataFusionError> {
        self.check_limit(additional);
        // Delegate to inner pool, though we've already done our own check.
        // The inner pool also enforces the limit but returns Error.
        // We want to panic if WE detect it, so we did check_limit above.
        // But for correctness of inner state, we call it.
        self.pool.try_grow(reservation, additional)
    }

    fn reserved(&self) -> usize {
        self.pool.reserved()
    }
}
