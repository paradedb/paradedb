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

//! Symbol stubs that let standalone `cargo test` binaries link and load the
//! pg_search unit test binary.
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
//! * on Darwin the executable can be produced but `dyld` still rejects it
//!   on startup once it hits an unresolved Postgres global such as
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

#[cfg(not(test))]
compile_error!("tests_link_stubs must never ship in non-test builds");

use core::ffi::{c_char, c_int, c_long, c_void};
use core::mem::MaybeUninit;
use pgrx::pg_sys::{
    self, sigjmp_buf, tree_walker_callback, varlena, AttrNumber, BackendType, Buffer,
    BufferAccessStrategy, BufferAccessStrategyType, Datum, ErrorContextCallback, ErrorData,
    ExprContext, FunctionCallInfo, HeapTuple, IndexInfo, JsonbContainer, JsonbIterator,
    JsonbIteratorToken, JsonbValue, MemoryContext, Node, Oid, ParamListInfo, ParserSetupHook,
    Relation, SPIExecuteOptions, SPIPlanPtr, SPIPrepareOptions, SPITupleTable, Size, Snapshot,
    SnapshotData, TransactionId, TupleDesc, TupleTableSlot, TupleTableSlotOps,
};

const fn zeroed<T>() -> T {
    unsafe { MaybeUninit::<T>::zeroed().assume_init() }
}

fn leak_zeroed<T>() -> *mut T {
    Box::into_raw(Box::new(zeroed()))
}

fn leak_zeroed_bytes(size: Size) -> *mut c_void {
    let mut bytes = vec![0_u8; size.max(1)];
    let ptr = bytes.as_mut_ptr();
    Box::leak(bytes.into_boxed_slice());
    ptr.cast()
}

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
    BufferBlocks: *mut c_char,
    LocalBufferBlockPointers: *mut pg_sys::Block,
    SPI_tuptable: *mut SPITupleTable,
    error_context_stack: *mut ErrorContextCallback,
    PG_exception_stack: *mut sigjmp_buf,
}

#[no_mangle]
pub static mut InterruptHoldoffCount: pg_sys::uint32 = zeroed();
#[no_mangle]
pub static mut InterruptPending: pg_sys::sig_atomic_t = 0;
#[no_mangle]
pub static mut NBuffers: c_int = 0;
#[no_mangle]
pub static mut NLocBuffer: c_int = 0;
#[no_mangle]
pub static mut MyBackendType: BackendType::Type = zeroed();
#[no_mangle]
pub static mut CheckXidAlive: TransactionId = zeroed();
#[no_mangle]
pub static mut bsysscan: bool = false;
#[no_mangle]
pub static mut QueryCancelPending: pg_sys::sig_atomic_t = 0;
#[no_mangle]
pub static mut SnapshotAnyData: SnapshotData = zeroed();
#[no_mangle]
pub static TTSOpsBufferHeapTuple: TupleTableSlotOps = zeroed();
#[no_mangle]
pub static mut SPI_processed: pg_sys::uint64 = 0;
#[no_mangle]
pub static mut SPI_result: c_int = 0;

#[no_mangle]
pub unsafe extern "C" fn errstart(_elevel: c_int, _domain: *const c_char) -> bool {
    false
}

#[no_mangle]
pub unsafe extern "C" fn errstart_cold(_elevel: c_int, _domain: *const c_char) -> bool {
    false
}

#[no_mangle]
pub unsafe extern "C" fn errfinish(
    _filename: *const c_char,
    _lineno: c_int,
    _funcname: *const c_char,
) {
}

#[no_mangle]
pub unsafe extern "C" fn errcode(_sqlerrcode: c_int) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn errmsg(_fmt: *const c_char) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn errmsg_internal(_fmt: *const c_char) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn errdetail(_fmt: *const c_char) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn errhint(_fmt: *const c_char) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn errcontext_msg(_fmt: *const c_char) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn CopyErrorData() -> *mut ErrorData {
    leak_zeroed()
}

#[no_mangle]
pub unsafe extern "C" fn FreeErrorData(_edata: *mut ErrorData) {}

#[no_mangle]
pub unsafe extern "C" fn FlushErrorState() {}

#[no_mangle]
pub unsafe extern "C" fn palloc0(size: Size) -> *mut c_void {
    leak_zeroed_bytes(size)
}

#[no_mangle]
pub unsafe extern "C" fn pfree(_pointer: *mut c_void) {}

#[no_mangle]
pub unsafe extern "C" fn pg_detoast_datum(datum: *mut varlena) -> *mut varlena {
    datum
}

#[no_mangle]
pub unsafe extern "C" fn copyObjectImpl(from: *const c_void) -> *mut c_void {
    from.cast_mut()
}

#[no_mangle]
pub unsafe extern "C" fn nodeToString(_obj: *const c_void) -> *mut c_char {
    core::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn nodeToStringWithLocations(_obj: *const c_void) -> *mut c_char {
    core::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn heap_freetuple(_htup: HeapTuple) {}

#[no_mangle]
pub unsafe extern "C" fn slot_getsomeattrs_int(_slot: *mut TupleTableSlot, _attnum: c_int) {}

#[no_mangle]
pub unsafe extern "C" fn JsonbIteratorInit(_container: *mut JsonbContainer) -> *mut JsonbIterator {
    leak_zeroed()
}

#[no_mangle]
pub unsafe extern "C" fn JsonbIteratorNext(
    _it: *mut *mut JsonbIterator,
    _val: *mut JsonbValue,
    _skip_nested: bool,
) -> JsonbIteratorToken::Type {
    zeroed()
}

#[no_mangle]
pub unsafe extern "C" fn FreeExprContext(_econtext: *mut ExprContext, _is_commit: bool) {}

#[no_mangle]
pub unsafe extern "C" fn IsTransactionState() -> bool {
    false
}

#[no_mangle]
pub unsafe extern "C" fn GetActiveSnapshot() -> Snapshot {
    &raw mut SnapshotAnyData
}

#[no_mangle]
pub unsafe extern "C" fn ReleaseBuffer(_buffer: Buffer) {}

#[no_mangle]
pub unsafe extern "C" fn MarkBufferDirty(_buffer: Buffer) {}

#[no_mangle]
pub unsafe extern "C" fn GetAccessStrategy(
    _btype: BufferAccessStrategyType::Type,
) -> BufferAccessStrategy {
    core::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn HotStandbyActive() -> bool {
    false
}

#[no_mangle]
pub unsafe extern "C" fn BuildIndexInfo(_index: Relation) -> *mut IndexInfo {
    leak_zeroed()
}

#[no_mangle]
pub unsafe extern "C" fn SPI_connect() -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn SPI_connect_ext(_options: c_int) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn SPI_finish() -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn SPI_execute(
    _src: *const c_char,
    _read_only: bool,
    _tcount: c_long,
) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn SPI_execute_extended(
    _src: *const c_char,
    _options: *const SPIExecuteOptions,
) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn SPI_execute_plan(
    _plan: SPIPlanPtr,
    _values: *mut Datum,
    _nulls: *const c_char,
    _read_only: bool,
    _tcount: c_long,
) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn SPI_execute_plan_extended(
    _plan: SPIPlanPtr,
    _options: *const SPIExecuteOptions,
) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn SPI_execute_plan_with_paramlist(
    _plan: SPIPlanPtr,
    _params: ParamListInfo,
    _read_only: bool,
    _tcount: c_long,
) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn SPI_execute_snapshot(
    _plan: SPIPlanPtr,
    _values: *mut Datum,
    _nulls: *const c_char,
    _snapshot: Snapshot,
    _crosscheck_snapshot: Snapshot,
    _read_only: bool,
    _fire_triggers: bool,
    _tcount: c_long,
) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn SPI_execute_with_args(
    _src: *const c_char,
    _nargs: c_int,
    _argtypes: *mut Oid,
    _values: *mut Datum,
    _nulls: *const c_char,
    _read_only: bool,
    _tcount: c_long,
) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn SPI_prepare(
    _src: *const c_char,
    _nargs: c_int,
    _argtypes: *mut Oid,
) -> SPIPlanPtr {
    core::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn SPI_prepare_cursor(
    _src: *const c_char,
    _nargs: c_int,
    _argtypes: *mut Oid,
    _cursor_options: c_int,
) -> SPIPlanPtr {
    core::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn SPI_prepare_extended(
    _src: *const c_char,
    _options: *const SPIPrepareOptions,
) -> SPIPlanPtr {
    core::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn SPI_prepare_params(
    _src: *const c_char,
    _parser_setup: ParserSetupHook,
    _parser_setup_arg: *mut c_void,
    _cursor_options: c_int,
) -> SPIPlanPtr {
    core::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn SPI_keepplan(_plan: SPIPlanPtr) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn SPI_fnumber(_tupdesc: TupleDesc, _fname: *const c_char) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn expression_tree_walker_impl(
    _node: *mut Node,
    _walker: tree_walker_callback,
    _context: *mut c_void,
) -> bool {
    false
}

#[no_mangle]
pub unsafe extern "C" fn numeric_in(_fcinfo: FunctionCallInfo) -> Datum {
    zeroed()
}

#[no_mangle]
pub unsafe extern "C" fn numeric_out(_fcinfo: FunctionCallInfo) -> Datum {
    zeroed()
}

#[no_mangle]
pub unsafe extern "C" fn int2_numeric(_fcinfo: FunctionCallInfo) -> Datum {
    zeroed()
}

#[no_mangle]
pub unsafe extern "C" fn int4_numeric(_fcinfo: FunctionCallInfo) -> Datum {
    zeroed()
}

#[no_mangle]
pub unsafe extern "C" fn int8_numeric(_fcinfo: FunctionCallInfo) -> Datum {
    zeroed()
}

#[no_mangle]
pub unsafe extern "C" fn float4_numeric(_fcinfo: FunctionCallInfo) -> Datum {
    zeroed()
}

#[no_mangle]
pub unsafe extern "C" fn float8_numeric(_fcinfo: FunctionCallInfo) -> Datum {
    zeroed()
}

#[no_mangle]
pub unsafe extern "C" fn numeric_int2(_fcinfo: FunctionCallInfo) -> Datum {
    zeroed()
}

#[no_mangle]
pub unsafe extern "C" fn numeric_int4(_fcinfo: FunctionCallInfo) -> Datum {
    zeroed()
}

#[no_mangle]
pub unsafe extern "C" fn numeric_int8(_fcinfo: FunctionCallInfo) -> Datum {
    zeroed()
}

#[no_mangle]
pub unsafe extern "C" fn numeric_float4(_fcinfo: FunctionCallInfo) -> Datum {
    zeroed()
}

#[no_mangle]
pub unsafe extern "C" fn numeric_float8(_fcinfo: FunctionCallInfo) -> Datum {
    zeroed()
}

#[no_mangle]
pub unsafe extern "C" fn time_timetz(_fcinfo: FunctionCallInfo) -> Datum {
    zeroed()
}

#[no_mangle]
pub unsafe extern "C" fn timetz_time(_fcinfo: FunctionCallInfo) -> Datum {
    zeroed()
}

#[no_mangle]
pub unsafe extern "C" fn timetz_zone(_fcinfo: FunctionCallInfo) -> Datum {
    zeroed()
}

#[no_mangle]
pub unsafe extern "C" fn timetz_izone(_fcinfo: FunctionCallInfo) -> Datum {
    zeroed()
}

#[no_mangle]
pub unsafe extern "C" fn extract_date(_fcinfo: FunctionCallInfo) -> Datum {
    zeroed()
}

#[no_mangle]
pub unsafe extern "C" fn extract_time(_fcinfo: FunctionCallInfo) -> Datum {
    zeroed()
}

#[no_mangle]
pub unsafe extern "C" fn extract_timetz(_fcinfo: FunctionCallInfo) -> Datum {
    zeroed()
}

#[no_mangle]
pub unsafe extern "C" fn extract_timestamp(_fcinfo: FunctionCallInfo) -> Datum {
    zeroed()
}

#[no_mangle]
pub unsafe extern "C" fn extract_timestamptz(_fcinfo: FunctionCallInfo) -> Datum {
    zeroed()
}

#[no_mangle]
pub unsafe extern "C" fn regprocedurein(_fcinfo: FunctionCallInfo) -> Datum {
    zeroed()
}

#[no_mangle]
pub unsafe extern "C" fn quote_identifier(ident: *const c_char) -> *const c_char {
    ident
}

#[no_mangle]
pub unsafe extern "C" fn get_array_type(_typid: Oid) -> Oid {
    0.into()
}

#[no_mangle]
pub unsafe extern "C" fn GetSysCacheOid(
    _cache_id: c_int,
    _oidcol: AttrNumber,
    _key1: Datum,
    _key2: Datum,
    _key3: Datum,
    _key4: Datum,
) -> Oid {
    0.into()
}

#[test]
fn stubs_link_cleanly() {
    let _ = &raw mut CurrentMemoryContext;
    let _ = &raw mut TopMemoryContext;
    let _ = &raw mut ErrorContext;
    let _ = &raw mut CacheMemoryContext;
    let _ = &raw mut MessageContext;
    let _ = &raw mut TopTransactionContext;
    let _ = &raw mut CurTransactionContext;
    let _ = &raw mut PortalContext;
    let _ = &raw mut PostmasterContext;
    let _ = &raw mut BufferBlocks;
    let _ = &raw mut LocalBufferBlockPointers;
    let _ = &raw mut SPI_tuptable;
    let _ = &raw mut error_context_stack;
    let _ = &raw mut PG_exception_stack;
    let _ = &raw mut InterruptHoldoffCount;
    let _ = &raw mut InterruptPending;
    let _ = &raw mut NBuffers;
    let _ = &raw mut NLocBuffer;
    let _ = &raw mut MyBackendType;
    let _ = &raw mut CheckXidAlive;
    let _ = &raw mut bsysscan;
    let _ = &raw mut QueryCancelPending;
    let _ = &raw mut SnapshotAnyData;
    let _ = &raw const TTSOpsBufferHeapTuple;
    let _ = &raw mut SPI_processed;
    let _ = &raw mut SPI_result;

    let _ = errstart as usize;
    let _ = errstart_cold as usize;
    let _ = errfinish as usize;
    let _ = errcode as usize;
    let _ = errmsg as usize;
    let _ = errmsg_internal as usize;
    let _ = errdetail as usize;
    let _ = errhint as usize;
    let _ = errcontext_msg as usize;
    let _ = CopyErrorData as usize;
    let _ = FreeErrorData as usize;
    let _ = FlushErrorState as usize;
    let _ = palloc0 as usize;
    let _ = pfree as usize;
    let _ = pg_detoast_datum as usize;
    let _ = copyObjectImpl as usize;
    let _ = nodeToString as usize;
    let _ = nodeToStringWithLocations as usize;
    let _ = heap_freetuple as usize;
    let _ = slot_getsomeattrs_int as usize;
    let _ = JsonbIteratorInit as usize;
    let _ = JsonbIteratorNext as usize;
    let _ = FreeExprContext as usize;
    let _ = IsTransactionState as usize;
    let _ = GetActiveSnapshot as usize;
    let _ = ReleaseBuffer as usize;
    let _ = MarkBufferDirty as usize;
    let _ = GetAccessStrategy as usize;
    let _ = HotStandbyActive as usize;
    let _ = BuildIndexInfo as usize;
    let _ = SPI_connect as usize;
    let _ = SPI_connect_ext as usize;
    let _ = SPI_finish as usize;
    let _ = SPI_execute as usize;
    let _ = SPI_execute_extended as usize;
    let _ = SPI_execute_plan as usize;
    let _ = SPI_execute_plan_extended as usize;
    let _ = SPI_execute_plan_with_paramlist as usize;
    let _ = SPI_execute_snapshot as usize;
    let _ = SPI_execute_with_args as usize;
    let _ = SPI_prepare as usize;
    let _ = SPI_prepare_cursor as usize;
    let _ = SPI_prepare_extended as usize;
    let _ = SPI_prepare_params as usize;
    let _ = SPI_keepplan as usize;
    let _ = SPI_fnumber as usize;
    let _ = expression_tree_walker_impl as usize;
    let _ = numeric_in as usize;
    let _ = numeric_out as usize;
    let _ = int2_numeric as usize;
    let _ = int4_numeric as usize;
    let _ = int8_numeric as usize;
    let _ = float4_numeric as usize;
    let _ = float8_numeric as usize;
    let _ = numeric_int2 as usize;
    let _ = numeric_int4 as usize;
    let _ = numeric_int8 as usize;
    let _ = numeric_float4 as usize;
    let _ = numeric_float8 as usize;
    let _ = time_timetz as usize;
    let _ = timetz_time as usize;
    let _ = timetz_zone as usize;
    let _ = timetz_izone as usize;
    let _ = extract_date as usize;
    let _ = extract_time as usize;
    let _ = extract_timetz as usize;
    let _ = extract_timestamp as usize;
    let _ = extract_timestamptz as usize;
    let _ = regprocedurein as usize;
    let _ = quote_identifier as usize;
    let _ = get_array_type as usize;
    let _ = GetSysCacheOid as usize;
}
