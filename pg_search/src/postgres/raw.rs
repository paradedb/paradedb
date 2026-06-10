//! Direct `extern "C"` bindings for Postgres functions that bypass the pgrx `pg_guard` wrapper.
//!
//! Functions declared here MUST only be called when the caller has independently established
//! that the underlying C function will not `ereport`. The `pg_guard` wrapper exists to translate
//! Postgres `longjmp` into a Rust panic so destructors run; bypassing it on a code path that
//! could elog risks leaking pinned buffers, locks, and other RAII-tracked resources.
//!
//! See `heap::VisibilityChecker::is_block_all_visible` for an example of a caller that
//! speculatively checks the C function's fast-path precondition before bypassing the wrapper.

use pgrx::pg_sys::{self, BlockNumber, Buffer, Relation};

/// Mirrors `#define HEAPBLOCKS_PER_BYTE` from `src/include/access/visibilitymap.h`:
/// `BITS_PER_BYTE / BITS_PER_HEAPBLOCK`.
pub const HEAPBLOCKS_PER_BYTE: u32 = 8 / pg_sys::BITS_PER_HEAPBLOCK;

/// Mirrors `#define HEAPBLOCKS_PER_PAGE` from `src/include/access/visibilitymap.h`:
/// `BLCKSZ * HEAPBLOCKS_PER_BYTE`. Number of heap blocks covered by one VM page.
pub const HEAPBLOCKS_PER_PAGE: u32 = pg_sys::BLCKSZ * HEAPBLOCKS_PER_BYTE;

extern "C" {
    /// Raw binding to Postgres `visibilitymap_get_status`. Safe to call without the
    /// pgrx wrapper ONLY when the caller has confirmed `*buf` is valid and already
    /// holds the correct mapBlock for `heapBlk` — i.e. the C function will take its
    /// fast bit-math branch and will not invoke `vm_readbuf`.
    pub fn visibilitymap_get_status(rel: Relation, heapBlk: BlockNumber, buf: *mut Buffer) -> u8;
}
