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

//! Symbol stubs that let `cargo test --tests` link and load the pg_search
//! unit test binary on Linux.
//!
//! pgrx 0.18 removed the `pgrx_embed` indirection that used to keep
//! Postgres' globals out of test-binary link paths. Every `#[pg_extern]`
//! wrapper now expands to code that references `CurrentMemoryContext` and
//! friends directly. Those globals live in the Postgres backend image and
//! only exist once pg_search is `dlopen`'d by a running Postgres. A
//! standalone `cargo test` binary has no Postgres process above it, so:
//!
//! * `ld` refuses to produce an executable with undefined data symbols
//!   (Linux treats that as a hard error for ELF executables), and
//! * even with `-Wl,--unresolved-symbols=ignore-all` at link time,
//!   glibc's x86_64 loader rejects the binary on startup with
//!   `undefined symbol: CurrentMemoryContext`.
//!
//! This module provides local null definitions of the Postgres globals
//! that pgrx's compile-time-emitted code reaches. Because it's
//! `#[cfg(test)]`, the production cdylib (`cargo pgrx install`) does not
//! include any of these — its references stay as undefined imports that
//! Postgres resolves at `dlopen` time against its own process image.
//!
//! If a future pgrx or pg_search change makes the test binary reach a new
//! Postgres global and you see `undefined symbol: X` from the loader,
//! add `X` to the list below.

#![allow(non_upper_case_globals)]

use pgrx::pg_sys::{ErrorContextCallback, MemoryContext, sigjmp_buf};

macro_rules! stub_ptr {
    ($($name:ident: $ty:ty),* $(,)?) => {
        $(
            #[no_mangle]
            pub static mut $name: $ty = core::ptr::null_mut();
        )*
    };
}

// MemoryContext globals referenced by pgrx's `#[pg_extern]` wrappers and
// the memory-context helpers in `pgrx::memcxt` / `pgrx::memcx`.
stub_ptr! {
    CurrentMemoryContext: MemoryContext,
    TopMemoryContext: MemoryContext,
    ErrorContext: MemoryContext,
    CacheMemoryContext: MemoryContext,
    MessageContext: MemoryContext,
    TopTransactionContext: MemoryContext,
    CurTransactionContext: MemoryContext,
    PortalContext: MemoryContext,
    PostmasterContext: MemoryContext,
    error_context_stack: *mut ErrorContextCallback,
    PG_exception_stack: *mut sigjmp_buf,
}
