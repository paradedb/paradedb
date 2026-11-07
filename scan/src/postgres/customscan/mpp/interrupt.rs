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

pub use common::postgres::customscan::mpp::interrupt::*;

use std::task::Poll;

use datafusion::arrow::record_batch::RecordBatch;
use datafusion::common::DataFusionError;
use datafusion::execution::SendableRecordBatchStream;
use futures::StreamExt;

/// Execute fallible `f` synchronously on the Postgres main thread, converting any main-thread
/// panic (`pgrx::error!` / `ereport(ERROR)`) on a parallel worker into
/// `Err(DataFusionError::Execution(...))`.
pub fn try_run_on_main_thread<F, R>(f: F) -> datafusion::common::Result<R>
where
    F: FnOnce(MainThreadToken) -> datafusion::common::Result<R>,
{
    try_run_on_main_thread_with(f, DataFusionError::Execution)
}

/// Run `f`, catching any panic on a parallel worker and converting it into
/// `Err(DataFusionError::Execution(...))` so Tonic streams the error back to the coordinator.
/// On the leader backend, panics propagate normally.
pub fn catch_worker_panic<F, R>(f: F) -> datafusion::common::Result<R>
where
    F: FnOnce() -> datafusion::common::Result<R>,
{
    catch_worker_panic_with(f, DataFusionError::Execution)
}

/// Error an MPP loop returns when it bails on a pending cancel/die. The dispatcher discards
/// it and calls [`check_for_interrupts`] once `block_on` has returned.
pub fn interrupted() -> DataFusionError {
    DataFusionError::Execution("mpp: query interrupted".into())
}

/// Returns `Err(interrupted())` when a query-cancel or backend-die is pending so DataFusion
/// scan streams unwind cooperatively out of [`PgTokioRuntime::block_on`].
pub fn check_for_cancel() -> Result<(), DataFusionError> {
    if cancel_pending() {
        Err(interrupted())
    } else {
        Ok(())
    }
}

/// Drive one batch out of a DataFusion `stream` on the Postgres main thread under
/// [`HeldInterrupts`], while servicing any [`run_on_main_thread`] requests from background worker
/// tasks and checking for Postgres interrupts. Shared by `JoinScan`, `AggregateScan`, and
/// `BaseScan`.
pub fn block_on_next(
    runtime: &PgTokioRuntime,
    stream: &mut SendableRecordBatchStream,
) -> Option<Result<RecordBatch, DataFusionError>> {
    runtime.block_on(std::future::poll_fn(|cx| {
        if cancel_pending() {
            return Poll::Ready(Some(Err(interrupted())));
        }
        stream.poll_next_unpin(cx)
    }))
}
