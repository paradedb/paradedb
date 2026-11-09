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
#![allow(
    clippy::missing_safety_doc,
    clippy::not_unsafe_ptr_arg_deref,
    unsafe_op_in_unsafe_fn
)]

use crate::api::tokenizers::{DatumWithType, type_is_alias, type_is_tokenizer};
use crate::api::version::Version;
use crate::nodecast;
use crate::postgres::composite::CompositeSlotValues;
use crate::postgres::datetime::PostgresDateTime;
use crate::postgres::node::NodeExt;
use crate::postgres::types::{TantivyValue, TantivyValueError};

use crate::schema::{CategorizedFieldData, SearchFieldType};
use anyhow::Result;
use pgrx::itemptr::item_pointer_get_both;
use pgrx::*;

use std::collections::BTreeMap;
use std::str::FromStr;
use tokenizers::SearchNormalizer;

pub fn text_lower_funcoid() -> pg_sys::Oid {
    pg_sys::F_LOWER_TEXT.into()
}

unsafe extern "C-unwind" {
    // SAFETY: `IsTransactionState()` doesn't raise an ERROR.  As such, we can avoid the pgrx
    // sigsetjmp overhead by linking to the function directly.
    pub fn IsTransactionState() -> bool;
}

/// `IsA()` for a node pointer that may be NULL, as the `pgrx::is_a` that pgrx 0.20 removed.
///
/// # Safety
///
/// `node` must be NULL or point to a live `Node`.
#[inline]
pub unsafe fn is_a(node: *mut pg_sys::Node, tag: pg_sys::NodeTag) -> bool {
    unsafe { <pg_sys::Node as pg_sys::PgNode>::is_a_ptr(node, tag) }
}

/// Argument `num` of `fcinfo`, or `None` when it is SQL NULL, as the `pg_getarg_datum` that
/// pgrx 0.20 removed.
///
/// # Safety
///
/// `fcinfo` must point to a live `FunctionCallInfo` whose arguments are initialized.
#[inline]
pub unsafe fn pg_getarg_datum(
    fcinfo: pg_sys::FunctionCallInfo,
    num: usize,
) -> Option<pg_sys::Datum> {
    unsafe { pgrx::fcinfo::pg_get_nullable_datum(fcinfo, num) }.into()
}

pub fn vacuum_delay_point() {
    unsafe {
        #[cfg(any(feature = "pg15", feature = "pg16", feature = "pg17"))]
        pg_sys::vacuum_delay_point();
        #[cfg(not(any(feature = "pg15", feature = "pg16", feature = "pg17")))]
        pg_sys::vacuum_delay_point(false);
    }
}

/// Implements `Drop` for a Postgres resource that must be cleaned up on the Postgres main thread,
/// skipping cleanup during panic unwinding or process exit.
///
/// Because panics on the Postgres main thread are used to propagate PostgreSQL errors via pgrx, it
/// is almost never safe to interact with PostgreSQL APIs during panic unwinding - doing so can
/// cause a double-panic which results in SIGABRT. This macro also enforces that the destructor runs
/// on the Postgres main thread (printing via `eprintln!` so an off-main-thread drop during
/// unwinding never double-panics, panicking in debug builds when not unwinding, and skipping the
/// body in release builds). Types that can be dropped on background threads must wrap their inner
/// Postgres resource in `crate::postgres::customscan::mpp::MainThread` or forward their cleanup
/// payload via `crate::postgres::customscan::mpp::drop_on_main_thread` from a plain `impl Drop`.
///
/// PostgreSQL's transaction abort mechanism will clean up resources (buffers, relations, etc.)
/// when the transaction is aborted due to the error.
///
/// Cleanup is also skipped once `proc_exit` has started: a FATAL doesn't unwind, so `panicking()`
/// can't see it, and the abort that `proc_exit` runs releases those resources too. Skipping the body
/// doesn't stop Rust dropping the fields afterwards; values whose fields a FATAL can leave mid-use
/// are handled where they are leaked, see `PgMemoryContextsExt::leak_and_drop_unless_exiting`.
///
/// # Example
/// ```ignore
/// impl_safe_drop!(MyStruct, |self| {
///     unsafe {
///         if crate::postgres::utils::IsTransactionState() {
///             pg_sys::some_cleanup_function(self.handle);
///         }
///     }
/// });
/// ```
///
/// A generic type puts its impl parameters in brackets first:
/// `impl_safe_drop!([T: Trait] MyStruct<T>, |self| { ... })`.
#[macro_export]
macro_rules! impl_safe_drop {
    ([$($generics:tt)*] $ty:ty, |$self:ident| $body:block) => {
        impl<$($generics)*> Drop for $ty {
            fn drop(&mut $self) {
                if !$crate::postgres::customscan::mpp::interrupt::is_main_thread() {
                    eprintln!(
                        "BUG: Drop for {} ran off the Postgres main thread",
                        stringify!($ty)
                    );
                    #[cfg(debug_assertions)]
                    if !std::thread::panicking() {
                        panic!(
                            "BUG: Drop for {} ran off the Postgres main thread",
                            stringify!($ty)
                        );
                    }
                    return;
                }
                if std::thread::panicking() || $crate::postgres::utils::proc_exit_in_progress() {
                    return;
                }
                $body
            }
        }
    };
    ($ty:ty, |$self:ident| $body:block) => {
        $crate::impl_safe_drop!([] $ty, |$self| $body);
    };
}

/// Adds `leak_and_drop_unless_exiting` to pgrx's `PgMemoryContexts`.
pub trait PgMemoryContextsExt {
    /// pgrx's `leak_and_drop_on_delete`, except the drop is skipped once `proc_exit` has started.
    ///
    /// For the scan states that go with a query's memory: each custom scan's
    /// (`CustomScanStateWrapper`) and the index AM's (`IndexScanDesc::opaque`). A FATAL can stop
    /// any call into them, and a `Drop` on the state can't cover that: it skips only its own body, and
    /// Rust still drops the fields:
    ///
    /// 1. A FATAL (e.g. from `pg_terminate_backend`) is raised at a `CHECK_FOR_INTERRUPTS()`, which
    ///    can be deep inside a Rust call. Postgres calls `proc_exit` right there instead of
    ///    unwinding, so that call never returns and `panicking()` is false.
    /// 2. `proc_exit` aborts the transaction, which frees the query's memory. The callback on that
    ///    memory drops the scan state, including values the interrupted call was still using.
    /// 3. A value left mid-use panics when dropped: std's `OnceLock` while `get_or_init` is
    ///    running (the readers hold many, in `FFHelper`, `LinkedBytesList`, `DeferredScorer` and
    ///    tantivy's `SegmentReader`), tokio's `Runtime` while `block_on` holds its core, and the
    ///    columnar scan's DataFusion stream mid-poll, which frees memory it doesn't own. A Rust
    ///    panic can't pass through the C callback, so pgrx raises it as a Postgres ERROR.
    /// 4. Postgres turns an ERROR raised during `proc_exit` into a FATAL, which calls `proc_exit`
    ///    again. That nested exit skips the rest of the first abort, so the process exits still
    ///    holding its buffer pins and locks.
    ///
    /// Skipping the drop lets the abort finish and release them. Nothing is lost: the abort also
    /// releases the snapshots and temp files the state holds, and the `Box` goes with the process.
    /// After a plain ERROR the drop still runs, since the backend keeps going and the state would
    /// otherwise leak.
    ///
    /// The whole state is skipped, not just some of its values, because the values above are
    /// std, tokio, tantivy and DataFusion types: there is no `Drop` of ours on them to put
    /// `impl_safe_drop!` on. Our own `Drop`s are checked by `.github/scripts/check_impl_drop.py`.
    ///
    /// The body mirrors pgrx 0.19.2's `leak_and_drop_on_delete` (`memcxt.rs`) plus the
    /// `proc_exit` check; keep it in sync when bumping pgrx.
    fn leak_and_drop_unless_exiting<T>(&mut self, v: T) -> *mut T;
}

impl PgMemoryContextsExt for PgMemoryContexts {
    fn leak_and_drop_unless_exiting<T>(&mut self, v: T) -> *mut T {
        #[pg_guard]
        unsafe extern "C-unwind" fn drop_unless_exiting<T>(ptr: *mut std::ffi::c_void) {
            if proc_exit_in_progress() {
                return;
            }
            // SAFETY: `ptr` came from the `Box::leak` below, and Postgres runs a reset callback
            // once.
            drop(unsafe { Box::from_raw(ptr.cast::<T>()) });
        }

        let leaked = Box::leak(Box::new(v));
        unsafe {
            // SAFETY: the callback struct is plain pointers, all set before it is registered.
            let callback = self.palloc_struct::<pg_sys::MemoryContextCallback>();
            (*callback).func = Some(drop_unless_exiting::<T>);
            (*callback).arg = (leaked as *mut T).cast();
            pg_sys::MemoryContextRegisterResetCallback(self.value(), callback);
        }
        leaked
    }
}

/// Whether this backend is running `proc_exit`, where a FATAL may have left Rust state mid-use.
#[cfg(not(test))]
pub fn proc_exit_in_progress() -> bool {
    // SAFETY: plain read of a backend-local flag on the backend thread.
    unsafe { pgrx::pg_sys::proc_exit_inprogress }
}

/// The lib-test binary doesn't link the PG globals, and no test runs `proc_exit`.
#[cfg(test)]
pub fn proc_exit_in_progress() -> bool {
    false
}

/// RAII guard for PostgreSQL standalone expression context
/// Automatically frees the context when dropped
///
/// let context_guard = ExprContextGuard::new();
/// // Use context_guard.as_ptr() to get the raw pointer
/// // Context is automatically freed when context_guard goes out of scope
#[derive(Debug)]
pub struct ExprContextGuard(*mut pg_sys::ExprContext);

impl ExprContextGuard {
    /// Creates a new standalone expression context
    pub fn new() -> Self {
        unsafe { Self(pg_sys::CreateStandaloneExprContext()) }
    }

    /// Returns the raw pointer to the expression context
    pub fn as_ptr(&self) -> *mut pg_sys::ExprContext {
        self.0
    }
}

impl_safe_drop!(ExprContextGuard, |self| {
    unsafe {
        if IsTransactionState() && !self.0.is_null() {
            pg_sys::FreeExprContext(self.0, true);
        }
    }
});

impl Default for ExprContextGuard {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII guard that ensures an active snapshot is set for the life of the guard.
///
/// Most callers (ordinary DML, queries, VACUUM, background merge workers) already run with an
/// active snapshot. The logical-replication apply worker, however, applies remote changes and runs
/// pre-commit cleanup without pushing one or having an active portal, while operations like SPI
/// queries, expression planning (`RelationGetIndexExpressions` -> `eval_const_expressions`), and
/// out-of-line TOAST detoasting (`get_toast_snapshot`) require an active snapshot. This guard
/// pushes the transaction snapshot only when the caller lacks an active snapshot, and pops it on
/// normal scope exit.
///
/// On a panic / PostgreSQL ERROR the surrounding transaction aborts, which itself resets the
/// active-snapshot stack; popping again here would underflow it, so the drop implementation
/// uses `impl_safe_drop!` to skip cleanup while unwinding.
pub struct ActiveSnapshotGuard {
    pushed: bool,
}

impl ActiveSnapshotGuard {
    pub fn ensure() -> Self {
        let pushed = unsafe { !pg_sys::ActiveSnapshotSet() };
        if pushed {
            unsafe { pg_sys::PushActiveSnapshot(pg_sys::GetTransactionSnapshot()) };
        }
        Self { pushed }
    }
}

impl_safe_drop!(ActiveSnapshotGuard, |self| {
    if self.pushed {
        unsafe { pg_sys::PopActiveSnapshot() };
    }
});

/// Recursively sort all object keys in JSON for deterministic output
pub fn sort_json_keys(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            // Collect entries, sort by key, and rebuild
            let sorted: BTreeMap<String, serde_json::Value> = map
                .iter()
                .map(|(k, v)| {
                    let mut v = v.clone();
                    sort_json_keys(&mut v);
                    (k.clone(), v)
                })
                .collect();
            *map = sorted.into_iter().collect();
        }
        serde_json::Value::Array(arr) => {
            for item in arr.iter_mut() {
                sort_json_keys(item);
            }
        }
        _ => {}
    }
}

/// TransactionIdPrecedesOrEquals --- is id1 logically <= id2?
///
/// Ported to rust from the Postgres sources to avoid the pgrx FFI overhead
#[allow(non_snake_case)]
#[inline]
pub fn TransactionIdPrecedesOrEquals(
    id1: pg_sys::TransactionId,
    id2: pg_sys::TransactionId,
) -> bool {
    //
    // #define TransactionIdIsNormal(xid)		((xid) >= FirstNormalTransactionId)
    //
    // /*
    //  * TransactionIdPrecedesOrEquals --- is id1 logically <= id2?
    //  */
    // bool
    // TransactionIdPrecedesOrEquals(TransactionId id1, TransactionId id2)
    // {
    //     /*
    //      * If either ID is a permanent XID then we can just do unsigned
    //      * comparison.  If both are normal, do a modulo-2^32 comparison.
    //      */
    //     int32		diff;
    //
    //     if (!TransactionIdIsNormal(id1) || !TransactionIdIsNormal(id2))
    //         return (id1 <= id2);
    //
    //     diff = (int32) (id1 - id2);
    //     return (diff <= 0);
    // }

    #[inline]
    fn is_transaction_id_normal(xid: pg_sys::TransactionId) -> bool {
        xid >= pg_sys::FirstNormalTransactionId
    }

    if !is_transaction_id_normal(id1) || !is_transaction_id_normal(id2) {
        return id1 <= id2;
    }

    // Compare as i32 to handle wraparound
    unsafe {
        // SAFETY: `pg_sysTransactionId` is a #[repr(transparent)] wrapper around a `u32`
        let id1: i32 = std::mem::transmute(id1);
        let id2: i32 = std::mem::transmute(id2);
        let diff = id1.wrapping_sub(id2);
        diff <= 0
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use crate::postgres::utils::TransactionIdPrecedesOrEquals;
    use pgrx::{pg_sys, pg_test};

    #[pg_test]
    fn xid_precedes_or_equals_test() {
        use proptest::prelude::*;

        proptest!(|(xid1 in 0..u32::MAX, xid2 in 0..u32::MAX)| {
            let us = TransactionIdPrecedesOrEquals(pg_sys::TransactionId::from(xid1), pg_sys::TransactionId::from(xid2));
            let pg = unsafe { pg_sys::TransactionIdPrecedesOrEquals(pg_sys::TransactionId::from(xid1), pg_sys::TransactionId::from(xid2)) };
            prop_assert_eq!(us, pg);
        });
    }

    #[pg_test]
    fn convert_pg_date_string_test() {
        use crate::postgres::utils::convert_pg_date_string;
        use pgrx::PgBuiltInOids;

        let dt = convert_pg_date_string(PgBuiltInOids::DATEOID.into(), "2024-01-01");
        assert!(!dt.is_infinity());
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Ctid(u64);

impl Ctid {
    pub unsafe fn from_fcinfo(
        fcinfo: pg_sys::FunctionCallInfo,
        argument_index: usize,
    ) -> Option<Self> {
        let datum = unsafe { pg_getarg_datum(fcinfo, argument_index) }?;
        unsafe { pg_sys::ItemPointerData::from_datum(datum, false) }.map(Self::from)
    }

    pub fn is_valid(self) -> bool {
        self.0 as pg_sys::OffsetNumber != pg_sys::InvalidOffsetNumber
    }
}

impl From<pg_sys::ItemPointerData> for Ctid {
    fn from(value: pg_sys::ItemPointerData) -> Self {
        Self(item_pointer_to_u64(value))
    }
}

impl From<Ctid> for u64 {
    fn from(value: Ctid) -> Self {
        value.0
    }
}

/// Rather than using pgrx' version of this function, we use our own, which doesn't leave 2
/// empty bytes in the middle of the 64bit representation.  A ctid being only 48bits means
/// if we leave the upper 16 bits (2 bytes) empty, tantivy will have a better chance of
/// bitpacking or compressing these values.
#[inline(always)]
pub fn item_pointer_to_u64(ctid: pg_sys::ItemPointerData) -> u64 {
    let (blockno, offno) = item_pointer_get_both(ctid);
    let blockno = blockno as u64;
    let offno = offno as u64;

    // shift the BlockNumber left 16 bits -- the length of the OffsetNumber we OR onto the end
    // pgrx's version shifts left 32, which is wasteful
    (blockno << 16) | offno
}

/// Rather than using pgrx' version of this function, we use our own, which doesn't leave 2
/// empty bytes in the middle of the 64bit representation.  A ctid being only 48bits means
/// if we leave the upper 16 bits (2 bytes) empty, tantivy will have a better chance of
/// bitpacking or compressing these values.
#[inline(always)]
pub fn u64_to_item_pointer(value: u64, tid: &mut pg_sys::ItemPointerData) {
    let blockno = u64_ctid_block_number(value);
    let offno = value as pg_sys::OffsetNumber;
    item_pointer_set_all(tid, blockno, offno);
}

/// Stores `blockno` and `offno` in `tid`, as the `item_pointer_set_all` that pgrx 0.20 removed.
#[inline(always)]
pub fn item_pointer_set_all(
    tid: &mut pg_sys::ItemPointerData,
    blockno: pg_sys::BlockNumber,
    offno: pg_sys::OffsetNumber,
) {
    tid.ip_blkid.bi_hi = (blockno >> 16) as u16;
    tid.ip_blkid.bi_lo = blockno as u16;
    tid.ip_posid = offno;
}

/// The block number packed into a ctid by [`item_pointer_to_u64`].
#[inline(always)]
pub fn u64_ctid_block_number(value: u64) -> pg_sys::BlockNumber {
    // shift right 16 bits to pop off the OffsetNumber, leaving only the BlockNumber
    // pgrx's version must shift right 32 bits to be in parity with `item_pointer_to_u64()`
    (value >> 16) as pg_sys::BlockNumber
}

/// Formats a packed `u64` ctid as a Postgres tuple string `"(block,offset)"`.
///
/// TODO: introduce a newtype for packed ctids to encapsulate packing and formatting.
pub fn format_u64_ctid(value: u64) -> String {
    let blockno = u64_ctid_block_number(value);
    let offno = value as pg_sys::OffsetNumber;
    format!("({blockno},{offno})")
}

/// Returns `true` if the block referenced by `ctid` (u64-packed form) exists
/// in `rel`. A `false` result means VACUUM has truncated the page.
#[inline(always)]
pub unsafe fn ctid_satisfies_nblocks(
    ctid: u64,
    rel: pg_sys::Relation,
    fork: pg_sys::ForkNumber::Type,
) -> bool {
    let blockno = (ctid >> 16) as pg_sys::BlockNumber;
    blockno < pg_sys::RelationGetNumberOfBlocksInFork(rel, fork)
}

#[derive(Copy, Clone, Debug)]
pub enum FieldSource {
    /// Direct column from heap tuple
    Heap { attno: usize },

    /// Expression index (scalar result)
    Expression {
        att_idx: usize,
        attno: Option<usize>,
    },

    /// Field extracted from a composite type
    CompositeField {
        /// Index attribute number (slot in values[] array for aminsert/build_callback)
        index_attno: usize,

        /// Expression index (position in expr_results[] array for MVCC build)
        expression_idx: usize,

        /// Field position within the composite (0-indexed)
        field_idx: usize,

        /// OID of the named composite type
        composite_type_oid: pg_sys::Oid,
    },
}

impl FieldSource {
    /// The zero-based heap attribute the field holds. A tokenizer cast leaves the
    /// column's value as it is, so it names that column; a computed expression or a
    /// composite field has no single column to read back.
    pub fn heap_attno(&self) -> Option<usize> {
        match *self {
            FieldSource::Heap { attno } => Some(attno),
            FieldSource::Expression { attno, .. } => attno,
            FieldSource::CompositeField { .. } => None,
        }
    }
}

/// Collect composite slot info from categorized fields for upfront unpacking.
///
/// Returns a lazy iterator of (slot_index, datum, is_null, type_oid) for each unique
/// composite slot in the categorized fields.
///
/// # Safety
/// - `values` and `isnull` pointers are dereferenced lazily during iteration,
///   not at call time. Caller must ensure these pointers remain valid until
///   the returned iterator is fully consumed.
/// - The iterator should be consumed immediately (e.g., via `from_composites()`).
///   Storing the iterator and consuming it later risks UB if the underlying
///   slot arrays have been invalidated.
pub unsafe fn collect_composites_for_unpacking<'a>(
    categorized_fields: impl Iterator<Item = &'a CategorizedFieldData> + 'a,
    values: *mut pg_sys::Datum,
    isnull: *mut bool,
) -> impl Iterator<Item = (usize, pg_sys::Datum, bool, pg_sys::Oid)> + 'a {
    let mut seen = rustc_hash::FxHashSet::default();

    categorized_fields.filter_map(move |cat| {
        if let FieldSource::CompositeField {
            index_attno,
            composite_type_oid,
            ..
        } = cat.source
            && seen.insert(index_attno)
        {
            let datum = *values.add(index_attno);
            let is_null = *isnull.add(index_attno);
            return Some((index_attno, datum, is_null, composite_type_oid));
        }
        None
    })
}

/// Helper to extract field value from values[] array, handling composite fields.
///
/// **Works for**: aminsert (INSERT) and build_callback (CREATE INDEX) paths.
/// **Does NOT work for**: MVCC build (which uses expr_results[] instead).
///
/// # Arguments
/// * `source` - The field source (Heap, Expression, or CompositeField)
/// * `index_attno` - Index attribute position (slot in values[] array)
/// * `values` - Array of datums from PostgreSQL
/// * `isnull` - Array of null flags from PostgreSQL
/// * `unpacked_composites` - Pre-unpacked composite values
///
/// # Returns
/// Tuple of (datum, is_null) for the field
///
/// # Safety
/// Caller must ensure values and isnull pointers are valid.
pub unsafe fn get_field_value(
    source: &FieldSource,
    index_attno: usize,
    values: *mut pg_sys::Datum,
    isnull: *mut bool,
    unpacked_composites: &CompositeSlotValues,
) -> (pg_sys::Datum, bool) {
    match source {
        FieldSource::Heap { .. } | FieldSource::Expression { .. } => {
            (*values.add(index_attno), *isnull.add(index_attno))
        }
        FieldSource::CompositeField {
            index_attno: comp_index_attno,
            field_idx,
            ..
        } => unpacked_composites.get(*comp_index_attno, *field_idx),
    }
}

/// Resolves an index field's `(datum, is_null)` from a deformed heap tuple and its evaluated
/// index expressions. The boundary sampler and the mutable-segment materialization both project
/// the same field sources, so they share this instead of matching `FieldSource` in each place.
/// This is not [`get_field_value`], which indexes a single formed-tuple array by index attno.
pub unsafe fn resolve_field_value(
    source: &FieldSource,
    values: &[pg_sys::Datum],
    isnull: &[bool],
    expr_results: &[(pg_sys::Datum, bool)],
    unpacked_composites: &CompositeSlotValues,
) -> (pg_sys::Datum, bool) {
    match source {
        FieldSource::Heap { attno } => (values[*attno], isnull[*attno]),
        FieldSource::Expression { att_idx, .. } => expr_results[*att_idx],
        FieldSource::CompositeField {
            expression_idx,
            field_idx,
            ..
        } => unpacked_composites.get(*expression_idx, *field_idx),
    }
}

/// Represents the metadata extracted from an index attribute
#[derive(Debug)]
pub struct ExtractedFieldAttribute {
    /// its ordinal position in the index attribute list
    pub attno: usize,

    /// its source in the heap: either a heap attno, or an expression index
    pub source: FieldSource,

    /// its original Postgres type OID
    pub pg_type: PgOid,

    /// the type we'll use for indexing in tantivy
    pub tantivy_type: SearchFieldType,

    pub inner_typoid: pg_sys::Oid,

    pub normalizer: Option<SearchNormalizer>,
}

/// Recursively strips tokenizer casts (e.g. `pdb.literal`, `pdb.alias`) from an expression.
///
/// This is used both for schema building (determining the inner type of an indexed expression)
/// and for Top-K query planning (canonicalizing an indexed expression to match against an ORDER BY clause).
pub fn strip_tokenizer_cast(node: *mut pg_sys::Node) -> *mut pg_sys::Node {
    if node.is_null() {
        return node;
    }

    if let Some(func) = unsafe { nodecast!(FuncExpr, T_FuncExpr, node) } {
        let func = unsafe { &*func };
        if type_is_tokenizer(func.funcresulttype) {
            let args = unsafe { PgList::<pg_sys::Node>::from_pg(func.args) };
            if let Some(arg) = args.get_ptr(0) {
                return strip_tokenizer_cast(arg);
            }
        }
    } else if let Some(relabel) = unsafe { nodecast!(RelabelType, T_RelabelType, node) } {
        // RelabelType doesn't change physical datum representation, so it is safe to strip.
        return strip_tokenizer_cast(unsafe { (*relabel).arg }.cast());
    } else if let Some(coerce) = unsafe { nodecast!(CoerceToDomain, T_CoerceToDomain, node) } {
        // CoerceToDomain doesn't change physical datum representation, so it is safe to strip.
        return strip_tokenizer_cast(unsafe { (*coerce).arg }.cast());
    } else if let Some(coerce) = unsafe { nodecast!(CoerceViaIO, T_CoerceViaIO, node) } {
        // CoerceViaIO (e.g. `text::int`) CAN change the underlying physical representation.
        // We must only strip it if it explicitly targets a tokenizer type.
        let coerce = unsafe { &*coerce };
        if type_is_tokenizer(coerce.resulttype) {
            return strip_tokenizer_cast(coerce.arg.cast());
        }
    }

    node
}

/// Strips the `RelabelType` wrappers from an expression.
///
/// # Safety
/// `node` must be null or point to a valid node.
pub unsafe fn strip_relabel(mut node: *mut pg_sys::Node) -> *mut pg_sys::Node {
    while !node.is_null() && (*node).type_ == pg_sys::NodeTag::T_RelabelType {
        node = (*node.cast::<pg_sys::RelabelType>()).arg.cast();
    }
    node
}

/// Recursively strips `UNNEST` function calls and basic type coercion wrappers
/// (`RelabelType`, `CoerceToDomain`, `CoerceViaIO`) from an expression.
///
/// Returns a tuple containing the stripped node and a boolean indicating if `UNNEST` was found.
pub fn strip_unnest_and_relabel(mut node: *mut pg_sys::Node) -> (*mut pg_sys::Node, bool) {
    let mut found_unnest = false;
    loop {
        if node.is_null() {
            return (node, found_unnest);
        }

        if let Some(relabel) = unsafe { nodecast!(RelabelType, T_RelabelType, node) } {
            node = unsafe { (*relabel).arg }.cast();
            continue;
        }
        if let Some(coerce) = unsafe { nodecast!(CoerceToDomain, T_CoerceToDomain, node) } {
            node = unsafe { (*coerce).arg }.cast();
            continue;
        }
        if let Some(coerce) = unsafe { nodecast!(CoerceViaIO, T_CoerceViaIO, node) } {
            node = unsafe { (*coerce).arg }.cast();
            continue;
        }
        if let Some(phv) = unsafe { nodecast!(PlaceHolderVar, T_PlaceHolderVar, node) } {
            node = unsafe { (*phv).phexpr }.cast();
            continue;
        }
        if let Some(func) = unsafe { nodecast!(FuncExpr, T_FuncExpr, node) }
            && let func = unsafe { &*func }
            && is_unnest_func(func.funcid)
        {
            let args = unsafe { PgList::<pg_sys::Node>::from_pg(func.args) };
            if args.len() != 1 {
                return (std::ptr::null_mut(), false);
            }
            found_unnest = true;
            if let Some(arg) = args.get_ptr(0) {
                node = arg;
                continue;
            }
        }
        break;
    }
    (node, found_unnest)
}

/// Identifies if the function identified by `funcid` is `pg_catalog.unnest(anyarray)`.
pub fn is_unnest_func(funcid: pg_sys::Oid) -> bool {
    funcid.to_u32() == pg_sys::F_UNNEST_ANYARRAY
}

/// Create a text Const node from a string
///
/// # Safety
/// This function must be called within a PostgreSQL memory context that will persist
/// for the lifetime of the plan tree. The returned Const node will be allocated in the
/// current memory context and should not be freed manually.
pub unsafe fn make_text_const(text: &str) -> *mut pg_sys::Const {
    let text_datum = text
        .into_datum()
        .expect("failed to convert string to datum");

    pg_sys::makeConst(
        pg_sys::TEXTOID,
        -1,
        pg_sys::DEFAULT_COLLATION_OID,
        -1,
        text_datum,
        false, // constisnull
        false, // constbyval (text is not passed by value)
    )
}

/// Converts a non-NULL, single-valued (non-array, non-JSON, non-vector) index datum into the
/// value the index stores for its field. `datum` must already have any `pdb.alias` /
/// tokenizer wrapper type stripped, see [`unwrap_alias_datum`].
pub unsafe fn scalar_datum_to_tantivy_value(
    datum: pg_sys::Datum,
    field_type: SearchFieldType,
    base_oid: PgOid,
    created_by_version: Option<Version>,
) -> Result<TantivyValue, TantivyValueError> {
    match field_type {
        SearchFieldType::Numeric64(_, scale) => TantivyValue::try_from_numeric_i64(datum, scale),
        SearchFieldType::NumericBytes(..) => {
            TantivyValue::try_from_numeric_bytes(datum, created_by_version)
        }
        SearchFieldType::Range(oid) if oid == pg_sys::NUMRANGEOID => {
            TantivyValue::try_from_numrange(datum, created_by_version)
        }
        // Legacy pre-v0.22.0 indexes stored NUMERIC as F64 in the tantivy schema.
        SearchFieldType::F64(oid) if oid == pg_sys::NUMERICOID => {
            TantivyValue::try_from_numeric_f64(datum)
        }
        _ => TantivyValue::try_from_datum(datum, base_oid),
    }
}

/// For `pdb.alias` / tokenizer typed index attributes, returns the datum of the underlying
/// type; other datums pass through unchanged.
pub unsafe fn unwrap_alias_datum(datum: pg_sys::Datum, pg_type: PgOid) -> pg_sys::Datum {
    if type_is_alias(pg_type.value()) || type_is_tokenizer(pg_type.value()) {
        DatumWithType::get_underlying_type(datum).0
    } else {
        datum
    }
}

pub fn convert_pg_date_string(typeoid: PgOid, date_string: &str) -> PostgresDateTime {
    match typeoid {
        PgOid::BuiltIn(PgBuiltInOids::DATEOID | PgBuiltInOids::DATERANGEOID) => {
            let d = pgrx::datum::Date::from_str(date_string)
                .expect("must be valid postgres date format");
            PostgresDateTime::try_from(d).expect("date out of range")
        }
        // For TIMESTAMPOID, Used only by legacy indexes as of v0.24.0.
        PgOid::BuiltIn(PgBuiltInOids::TIMESTAMPOID | PgBuiltInOids::TSRANGEOID) => {
            let t = pgrx::datum::Timestamp::from_str(date_string)
                .expect("must be a valid postgres timestamp");
            PostgresDateTime::from(t)
        }
        // For TIMESTAMPTZOID, Used only by legacy indexes as of v0.24.0.
        PgOid::BuiltIn(PgBuiltInOids::TIMESTAMPTZOID | pg_sys::BuiltinOid::TSTZRANGEOID) => {
            let twtz = pgrx::datum::TimestampWithTimeZone::from_str(date_string)
                .expect("must be a valid postgres timestamp with time zone");
            PostgresDateTime::from(twtz)
        }
        PgOid::BuiltIn(PgBuiltInOids::TIMEOID) => {
            let t =
                pgrx::datum::Time::from_str(date_string).expect("must be a valid postgres time");
            PostgresDateTime::from(t)
        }
        PgOid::BuiltIn(PgBuiltInOids::TIMETZOID) => {
            let twtz = pgrx::datum::TimeWithTimeZone::from_str(date_string)
                .expect("must be a valid postgres time with time zone");
            PostgresDateTime::from(twtz)
        }
        _ => panic!("Unsupported typeoid: {typeoid:?}"),
    }
}

/// Extract precision and scale from PostgreSQL NUMERIC typmod.
///
/// PostgreSQL encodes NUMERIC precision and scale in the typmod as:
/// `typmod = ((precision << 16) | (scale & 0x7ff)) + VARHDRSZ`
///
/// The scale is stored in 11 bits (0x7ff = 2047), supporting the range [-1000, 1000].
/// Negative scales are stored in two's complement within those 11 bits.
///
/// # Returns
/// - `(precision, Some(scale))` if typmod specifies precision/scale
/// - `(0, None)` if typmod is -1 (unlimited/unspecified precision)
///
/// # Note
/// - For NUMERIC columns declared without precision (e.g., `NUMERIC` instead of `NUMERIC(10,2)`),
///   PostgreSQL uses typmod = -1, indicating arbitrary precision.
/// - PostgreSQL 15+ supports negative scales (e.g., NUMERIC(5,-3) rounds to nearest 1000).
///
/// # Reference
/// See PostgreSQL's `numeric_typmod_scale()` in src/backend/utils/adt/numeric.c:
/// ```c
/// return (((typmod - VARHDRSZ) & 0x7ff) ^ 1024) - 1024;
/// ```
pub fn extract_numeric_precision_scale(typmod: i32) -> (u16, Option<i16>) {
    // PostgreSQL uses typmod = -1 for unlimited precision NUMERIC
    if typmod < 0 {
        return (0, None);
    }

    // Extract precision and scale from typmod
    // See PostgreSQL's make_numeric_typmod() and numeric_typmod_scale()
    let typmod_val = (typmod - pg_sys::VARHDRSZ) as u32;

    // Precision is in upper 16 bits
    let precision = ((typmod_val >> 16) & 0xFFFF) as u16;

    // Scale is stored in lower 11 bits (0x7ff), using two's complement for negatives.
    // The formula (x ^ 1024) - 1024 sign-extends an 11-bit value to a full integer.
    // This works because 1024 = 2^10, the midpoint of the 11-bit range.
    let stored_scale = (typmod_val & 0x7ff) as i16;
    let scale = (stored_scale ^ 1024) - 1024;

    (precision, Some(scale))
}

// Backport of `store_att_byval()` that works on pg15
// Writes a datum `arg_newdatum` into memory pointed to by `arg_t`
// SAFETY: `arg_t` must point at writable memory of sufficient size to store the
//         Postgres value represented by `arg_newdatum`.
//         `arg_newdatum` must be a valid postgres value with the specified attlen.
pub unsafe fn store_att_byval(
    arg_t: *mut std::ffi::c_void,
    arg_newdatum: pg_sys::Datum,
    arg_attlen: std::ffi::c_int,
) {
    #[cfg(not(feature = "pg15"))]
    unsafe {
        pg_sys::store_att_byval(arg_t, arg_newdatum, arg_attlen)
    }

    #[cfg(feature = "pg15")]
    unsafe {
        match arg_attlen {
            1 => arg_t.cast::<i8>().write(arg_newdatum.value() as i8),
            2 => arg_t.cast::<i16>().write(arg_newdatum.value() as i16),
            4 => arg_t.cast::<i32>().write(arg_newdatum.value() as i32),
            8 => arg_t.cast::<i64>().write(arg_newdatum.value() as i64),
            _ => unreachable!(),
        }
    }
}

// Backport of `fetch_att_byval()` that works on pg15
// Returns a Datum representation of the Postgres value stored in `arg_t`
// SAFETY: The value stored in `arg_t` must be a valid Postgres value with
//         the specified by-val and att-len.
//         If the type is not by-val the provided pointer must live as long as
//         the Datum.
pub unsafe fn fetch_att(
    arg_t: *const ::core::ffi::c_void,
    arg_attbyval: bool,
    arg_attlen: ::core::ffi::c_int,
) -> pg_sys::Datum {
    #[cfg(not(feature = "pg15"))]
    unsafe {
        pg_sys::fetch_att(arg_t, arg_attbyval, arg_attlen)
    }

    #[cfg(feature = "pg15")]
    if arg_attbyval {
        unsafe {
            match arg_attlen {
                1 => pg_sys::Datum::from(arg_t.cast::<i8>().read()),
                2 => pg_sys::Datum::from(arg_t.cast::<i16>().read()),
                4 => pg_sys::Datum::from(arg_t.cast::<i32>().read()),
                8 => pg_sys::Datum::from(arg_t.cast::<i64>().read()),
                _ => unreachable!(),
            }
        }
    } else {
        pg_sys::Datum::from(arg_t)
    }
}

type IsArray = bool;
/// Returns the base type of the given `oid`, and a boolean indicating if the
/// type is an array.
pub fn resolve_base_type(oid: PgOid) -> Option<(PgOid, IsArray)> {
    fn is_domain_type(oid: pg_sys::Oid) -> bool {
        unsafe { pg_sys::get_typtype(oid) as u8 == pg_sys::TYPTYPE_DOMAIN }
    }

    if matches!(oid, PgOid::Invalid) {
        return None;
    }

    // resolve domain type to its base
    let base_oid = if is_domain_type(oid.value()) {
        let resolved_type = unsafe { pg_sys::getBaseType(oid.value()) };
        if resolved_type == pg_sys::InvalidOid {
            return None;
        }
        resolved_type
    } else {
        oid.value()
    };

    // check if it's an array type
    let array_type = PgOid::from(unsafe { pg_sys::get_element_type(base_oid) });

    match array_type {
        // not an array
        PgOid::Invalid => Some((base_oid.into(), false)),

        // built-in array type or custom array type
        PgOid::BuiltIn(_) | PgOid::Custom(_) => {
            let resolved_array_type = if is_domain_type(array_type.value()) {
                let resolved_type = unsafe { pg_sys::getBaseType(array_type.value()) };
                if resolved_type == pg_sys::InvalidOid {
                    return None;
                }
                resolved_type
            } else {
                array_type.value()
            };

            Some((resolved_array_type.into(), true))
        }
    }
}

pub trait ToPalloc: Sized {
    fn palloc(self) -> *mut Self {
        self.palloc_in(PgMemoryContexts::CurrentMemoryContext)
    }

    fn palloc_in(self, mcxt: PgMemoryContexts) -> *mut Self;
}

impl<T> ToPalloc for T {
    fn palloc_in(mut self, mut mcxt: PgMemoryContexts) -> *mut Self {
        unsafe { mcxt.copy_ptr_into((&mut self as *mut T).cast(), size_of::<T>()) }
    }
}

/// Look up a function in the pdb schema by name and argument types.
/// Returns InvalidOid if the function doesn't exist yet (e.g., during extension creation).
pub fn lookup_pdb_function(func_name: &str, arg_types: &[pg_sys::Oid]) -> pg_sys::Oid {
    unsafe {
        // Look up the pdb schema
        let pdb_schema = pg_sys::get_namespace_oid(c"pdb".as_ptr(), true);
        if pdb_schema == pg_sys::InvalidOid {
            return pg_sys::InvalidOid;
        }

        // Build the qualified function name list: pdb.<func_name>
        let mut func_name_list = PgList::<pg_sys::Node>::new();
        func_name_list.push(pg_sys::makeString(c"pdb".as_ptr() as *mut std::ffi::c_char) as *mut _);
        // Convert func_name to CString for makeString
        let func_name_cstr = std::ffi::CString::new(func_name).unwrap();
        func_name_list
            .push(pg_sys::makeString(func_name_cstr.as_ptr() as *mut std::ffi::c_char) as *mut _);

        // LookupFuncName returns InvalidOid if function doesn't exist (with missing_ok = true)
        pg_sys::LookupFuncName(
            func_name_list.as_ptr(),
            arg_types.len() as i32,
            arg_types.as_ptr(),
            true, // missing_ok = true, don't error if not found
        )
    }
}

/// Returns true if the pg_search extension is installed in the current database.
///
/// This is used to guard shared_preload_libraries hooks from touching schemas/types/functions
/// that only exist after `CREATE EXTENSION pg_search`.
pub fn pg_search_extension_installed() -> bool {
    unsafe { pg_sys::get_extension_oid(c"pg_search".as_ptr(), true) != pg_sys::InvalidOid }
}

/// RAII wrapper for `pg_sys::List` that automatically frees the list on drop.
///
/// This is useful when you need to create a temporary PostgreSQL list for use with
/// PostgreSQL functions and want to ensure it's properly freed even if the code
/// returns early. While unwinding or exiting it is left to its memory context.
///
/// # Example
/// ```ignore
/// let temp_list = TempPgList::new();
/// temp_list.push(some_node as *mut std::ffi::c_void);
/// let result = pg_sys::some_function(temp_list.as_ptr());
/// // temp_list is automatically freed when it goes out of scope
/// ```
#[derive(Default)]
pub struct TempPgList(*mut pg_sys::List);

impl TempPgList {
    /// Create a new empty temporary list.
    pub fn new() -> Self {
        Self(std::ptr::null_mut())
    }

    /// Append a cell to the list.
    ///
    /// # Safety
    /// The caller must ensure that `datum` is a valid pointer that can be
    /// stored in a PostgreSQL list.
    pub unsafe fn push(&mut self, datum: *mut std::ffi::c_void) {
        self.0 = pg_sys::lappend(self.0, datum);
    }

    /// Get the raw pointer to the list.
    pub fn as_ptr(&self) -> *mut pg_sys::List {
        self.0
    }
}

impl_safe_drop!(TempPgList, |self| {
    unsafe {
        if !self.0.is_null() {
            pg_sys::list_free(self.0);
        }
    }
});

/// Returns `true` if `index_predicate` belongs to a partial index whose predicate
/// is NOT implied by the query's restriction clauses -- i.e. the query is missing
/// the predicate, so the index is missing rows the query needs and cannot safely
/// answer it. Returns `false` for a non-partial index, or a partial index whose
/// predicate the query implies.
pub fn missing_partial_index_predicate(
    index_predicate: *mut pg_sys::List,
    restrict_info: &PgList<pg_sys::RestrictInfo>,
) -> bool {
    // Not a partial index: nothing can be missing.
    if index_predicate.is_null() {
        return false;
    }

    let mut clause_list: *mut pg_sys::List = std::ptr::null_mut();
    for ri in restrict_info.iter_ptr() {
        clause_list =
            unsafe { pg_sys::lappend(clause_list, (*ri).clause as *mut std::ffi::c_void) };
    }
    !unsafe { pg_sys::predicate_implied_by(index_predicate, clause_list, false) }
}

/// Filter out RestrictInfo entries whose clauses are implied by a partial index predicate.
///
/// When using a partial index (e.g., `CREATE INDEX ... WHERE deleted_at IS NULL`),
/// the index only contains rows that satisfy the predicate. If the query's WHERE clause
/// includes the same predicate, we don't need to create a heap filter for it since the
/// partial index already guarantees it.
///
/// This function uses PostgreSQL's `predicate_implied_by` to check if the index predicate
/// implies each query clause. If so, that clause is filtered out.
///
/// This is only the redundant-clause optimization; it does NOT verify the query is
/// compatible with the partial index. Callers must separately gate on
/// [`missing_partial_index_predicate`].
pub fn filter_implied_predicates(
    index_predicate: *mut pg_sys::List,
    restrict_info: &PgList<pg_sys::RestrictInfo>,
) -> PgList<pg_sys::RestrictInfo> {
    // If there's no partial index predicate, return the original list unchanged
    if index_predicate.is_null() {
        return unsafe { PgList::from_pg(restrict_info.as_ptr()) };
    }

    // Build a new list with only the predicates that are NOT implied by the index predicate
    let mut filtered_list: *mut pg_sys::List = std::ptr::null_mut();

    for ri in restrict_info.iter_ptr() {
        let clause = unsafe { (*ri).clause };
        let mut clause_list = TempPgList::new();
        unsafe { clause_list.push(clause as *mut std::ffi::c_void) };

        // Check if the index predicate implies this clause
        // predicate_implied_by(A, B, false) returns true if B => A
        let is_implied =
            unsafe { pg_sys::predicate_implied_by(clause_list.as_ptr(), index_predicate, false) };

        if !is_implied {
            filtered_list = unsafe { pg_sys::lappend(filtered_list, ri as *mut std::ffi::c_void) };
        }
    }

    unsafe { PgList::from_pg(filtered_list) }
}

#[macro_export]
macro_rules! debug1 {
    ($($arg:tt)*) => {{
        if unsafe { pg_sys::message_level_is_interesting(pg_sys::DEBUG1 as _) } {
            pg_sys::debug1!($($arg)*);
        }
    }};
}

#[macro_export]
macro_rules! debug2 {
    ($($arg:tt)*) => {{
        if unsafe { pg_sys::message_level_is_interesting(pg_sys::DEBUG2 as _) } {
            pg_sys::debug2!($($arg)*);
        }
    }};
}

/// Ensure all Var nodes referenced by `expr` are present in `tlist`.
///
/// For each base-relation Var in the expression, if no matching
/// `(varno, varattno)` entry exists yet, a new resjunk `TargetEntry` is
/// appended. This is used to populate `custom_scan_tlist` so that
/// `set_customscan_references` can resolve Var references in
/// `custom_exprs`.
pub fn add_vars_to_tlist(expr: *mut pg_sys::Node, tlist: &mut PgList<pg_sys::TargetEntry>) {
    if expr.is_null() {
        return;
    }

    for var_ptr in expr.collect_nodes::<pg_sys::Var>() {
        push_var_to_tlist_if_absent(var_ptr, tlist);
    }
}

/// Append `var_ptr` as a resjunk `TargetEntry` to `tlist` if `var_ptr` is a positive-attribute
/// base-relation `Var` and no `TargetEntry` for `(varno, varattno)` is already present.
pub fn push_var_to_tlist_if_absent(
    var_ptr: *mut pg_sys::Var,
    tlist: &mut PgList<pg_sys::TargetEntry>,
) {
    let varno = unsafe { (*var_ptr).varno } as pg_sys::Index;
    let varattno = unsafe { (*var_ptr).varattno };

    if varno == 0 || varattno <= 0 {
        return;
    }

    let already_present = tlist.iter_ptr().any(|te| {
        let te = unsafe { &*te };
        if unsafe { (*te.expr).type_ } == pg_sys::NodeTag::T_Var {
            let existing = unsafe { &*(te.expr as *mut pg_sys::Var) };
            existing.varno as pg_sys::Index == varno && existing.varattno == varattno
        } else {
            false
        }
    });

    if !already_present {
        let resno = tlist.len() as pg_sys::AttrNumber + 1;
        let new_var = unsafe { pg_sys::copyObjectImpl(var_ptr.cast()) }.cast::<pg_sys::Var>();
        let te =
            unsafe { pg_sys::makeTargetEntry(new_var.cast(), resno, std::ptr::null_mut(), true) };
        tlist.push(te);
    }
}

/// Recursively peels `RelabelType` and `PlaceHolderVar` wrappers to get the underlying node.
pub fn strip_wrappers(mut node: *mut pg_sys::Node) -> *mut pg_sys::Node {
    loop {
        if node.is_null() {
            return node;
        }
        match unsafe { (*node).type_ } {
            pg_sys::NodeTag::T_RelabelType => {
                node = unsafe { (*(node as *mut pg_sys::RelabelType)).arg }.cast();
            }
            pg_sys::NodeTag::T_PlaceHolderVar => {
                node = unsafe { (*(node as *mut pg_sys::PlaceHolderVar)).phexpr }.cast();
            }
            _ => break,
        }
    }
    node
}

/// Recursively strips all `PlaceHolderVar` wrappers anywhere inside `node`.
///
/// PostgreSQL 15 does not have a `readfuncs.c` reader for `PLACEHOLDERVAR`, so any expression
/// tree serialized via `nodeToString` (for later `stringToNode` deserialization) must have
/// nested `PlaceHolderVar` nodes (such as those introduced by `paradedb.placeholder_support`
/// around `pdb.score(...)` in join/aggregate queries) unwrapped to their `phexpr` first.
pub unsafe fn strip_placeholder_vars_deep(node: *mut pg_sys::Node) -> *mut pg_sys::Node {
    #[pgrx::pg_guard]
    unsafe extern "C-unwind" fn contains_phv_walker(
        node: *mut pg_sys::Node,
        context: *mut core::ffi::c_void,
    ) -> bool {
        if node.is_null() {
            return false;
        }
        if (*node).type_ == pg_sys::NodeTag::T_PlaceHolderVar {
            return true;
        }
        pg_sys::expression_tree_walker(node, Some(contains_phv_walker), context)
    }

    #[pgrx::pg_guard]
    unsafe extern "C-unwind" fn strip_phv_mutator(
        node: *mut pg_sys::Node,
        context: *mut core::ffi::c_void,
    ) -> *mut pg_sys::Node {
        if node.is_null() {
            return std::ptr::null_mut();
        }
        if let Some(phv) = crate::nodecast!(PlaceHolderVar, T_PlaceHolderVar, node) {
            return strip_phv_mutator((*phv).phexpr.cast(), context);
        }

        #[cfg(not(any(feature = "pg16", feature = "pg17", feature = "pg18")))]
        {
            pg_sys::expression_tree_mutator(node, Some(strip_phv_mutator), context)
        }

        #[cfg(any(feature = "pg16", feature = "pg17", feature = "pg18"))]
        {
            pg_sys::expression_tree_mutator_impl(node, Some(strip_phv_mutator), context)
        }
    }

    if !contains_phv_walker(node, std::ptr::null_mut()) {
        return node;
    }
    strip_phv_mutator(node, std::ptr::null_mut())
}

/// Unwraps `PlaceHolderVar` nodes (if any) and checks if the underlying node is a `FuncExpr`
/// whose OID is present in `funcoids`. Returns true if it matches.
pub fn is_search_operator(node: *mut pg_sys::Node, funcoids: &[pg_sys::Oid]) -> bool {
    if node.is_null() {
        return false;
    }

    let check_node = strip_wrappers(node);

    if unsafe { (*check_node).type_ } == pg_sys::NodeTag::T_FuncExpr {
        let funcexpr = unsafe { &*(check_node as *mut pg_sys::FuncExpr) };
        if funcoids.contains(&funcexpr.funcid) {
            return true;
        }
    }
    false
}
