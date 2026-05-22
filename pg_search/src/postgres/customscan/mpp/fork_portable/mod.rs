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

//! Distributed-runtime primitives that could plausibly live in the
//! [`datafusion-distributed`](https://github.com/paradedb/datafusion-distributed)
//! fork.
//!
//! Everything in this submodule is structurally embedding-agnostic — no
//! `pgrx::*`, no Postgres-specific concepts. Currently:
//!
//! - [`frame`] — multi-channel Arrow IPC frame codec ([`frame::MppFrameHeader`] +
//!   `encode_frame_into` / `encode_eof_frame_into` / `decode_frame`). Generic
//!   over any wire transport — Postgres `shm_mq`, in-proc `std::sync::mpsc`,
//!   hypothetically gRPC.
//!
//! What's deliberately **not** here (lives in [`super::transport`]):
//!
//! - [`super::transport::MppSender`]: the cooperative-spin send loop calls
//!   `pgrx::check_for_interrupts!()` and is therefore PG-backend-thread bound.
//! - The channel-buffer / drain-handle infrastructure (`DrainBuffer`,
//!   `DrainHandle`, `BatchChannelReceiver`/`Sender`, `CooperativeDrainSet`,
//!   in-proc channel pair): structurally portable, but the existing test
//!   module in `super::transport` reaches into private fields via `cfg(test)`
//!   inherent impls. Moving the production code without restructuring the
//!   ~850-LOC test module would require either a parallel test relocation or
//!   exposing the private fields to a wider scope. Out of R14 scope; revisit
//!   in a follow-up if the fork ever needs these primitives.
//!
//! See also [`super::mesh`] (shm_mq FFI, PG-bound) and [`super::dsm`]
//! (DSM coordinate layout, PG-bound) for the genuinely PG-tied pieces.
//!
//! Promotion to a real fork PR would lift `fork_portable` upstream as-is and
//! flip the import paths from `crate::postgres::customscan::mpp::fork_portable::*`
//! to `datafusion_distributed::*` at the few call sites that consume these
//! primitives.

pub mod frame;
