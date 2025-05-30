// Copyright (c) 2023-2025 ParadeDB, Inc.
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

//! Provides an abstract API over Postgres' parallel workers, allowing for the implementation of
//! different strategies for distributing work across processes.

pub mod builder;
pub mod mqueue;

use crate::parallel_worker::mqueue::MessageQueueSender;
use pgrx::pg_sys;
use std::error::Error;
use std::fmt::Display;
use std::ptr::NonNull;

#[repr(u64)]
enum TocKeys {
    MessageQueues = 1,
    UserStateLength = 2,
    UserState = 3,
}

impl From<TocKeys> for u64 {
    fn from(key: TocKeys) -> Self {
        key as _
    }
}

pub trait ParallelStateType: Copy {}

pub trait ParallelState {
    /// A binary representation of the header information necessary to store/retrieve [`ParallelState`]
    /// in shared memory
    fn info(&self) -> Vec<u8> {
        let mut info = Vec::new();
        let type_name = self.type_name();
        let nbytes = size_of::<usize>() + size_of::<usize>() + type_name.len();
        info.extend(nbytes.to_ne_bytes());
        info.extend(self.array_len().to_ne_bytes());
        info.extend(self.type_name().as_bytes());

        info
    }

    /// Returns the rust type name of the implementing type, or element if an array/vec
    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Returns the size of the type, in bytes, as represented in memory
    fn size_of(&self) -> usize {
        size_of_val(self)
    }

    /// Returns the number of elements in the array/vec, or `1` if it's not an array/vec
    fn array_len(&self) -> usize {
        1
    }

    /// Return a byte slice pointing to the raw bytes of this instance in memory
    fn as_bytes(&self) -> &[u8];
}

impl ParallelStateType for u8 {}
impl ParallelStateType for u16 {}
impl ParallelStateType for u32 {}
impl ParallelStateType for u64 {}
impl ParallelStateType for usize {}
impl ParallelStateType for i8 {}
impl ParallelStateType for i16 {}
impl ParallelStateType for i32 {}
impl ParallelStateType for i64 {}
impl ParallelStateType for isize {}
impl ParallelStateType for f32 {}
impl ParallelStateType for f64 {}
impl ParallelStateType for bool {}
impl ParallelStateType for () {}

impl<T: ParallelStateType> ParallelState for T {
    fn as_bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self as *const _ as *const u8, self.size_of()) }
    }
}

impl<T: ParallelStateType> ParallelState for Vec<T> {
    fn type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }
    fn size_of(&self) -> usize {
        self.len() * size_of::<T>()
    }
    fn array_len(&self) -> usize {
        self.len()
    }
    fn as_bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.as_ptr() as *const u8, self.size_of()) }
    }
}

pub trait ParallelProcess {
    fn state_values(&self) -> Vec<&dyn ParallelState>;
}

pub trait ParallelWorker {
    fn new(state_manager: ParallelStateManager) -> Self
    where
        Self: Sized;

    fn run(self, mq_sender: &MessageQueueSender, worker_number: i32) -> anyhow::Result<()>;
}

#[derive(Copy, Clone)]
pub struct ParallelStateManager {
    toc: NonNull<pg_sys::shm_toc>,
    len: usize,
}

#[derive(Debug)]
pub enum ValueError {
    WrongType(String, String),
}

impl Error for ValueError {}

impl Display for ValueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueError::WrongType(wanted, got) => {
                write!(f, "Wrong type, expected `{}`, got `{}`", wanted, got)
            }
        }
    }
}

impl ParallelStateManager {
    pub(crate) fn new(toc: *mut pg_sys::shm_toc) -> Self {
        assert!(!toc.is_null());
        unsafe {
            let ptr = pg_sys::shm_toc_lookup(toc, TocKeys::UserStateLength.into(), true);
            let len = *ptr.cast();

            Self {
                toc: NonNull::new(toc).unwrap_unchecked(),
                len,
            }
        }
    }

    unsafe fn decode_info<T: ParallelStateType>(
        &self,
        i: usize,
    ) -> Result<Option<(usize, &str)>, ValueError> {
        let idx: u64 = TocKeys::UserState.into();
        let idx = idx + i as u64;
        let ptr = pg_sys::shm_toc_lookup(self.toc.as_ptr(), idx, true);
        if ptr.is_null() {
            return Ok(None);
        }

        let info_len: usize = *ptr.cast();
        let len: usize = *ptr.add(size_of::<usize>()).cast();
        let type_name_len = info_len - (size_of::<usize>() * 2);
        let type_name = std::str::from_utf8(std::slice::from_raw_parts(
            ptr.add(size_of::<usize>() * 2).cast(),
            type_name_len,
        ))
        .expect("type_name should be valid UTF-8");

        if type_name != std::any::type_name::<T>() {
            return Err(ValueError::WrongType(
                type_name.to_owned(),
                std::any::type_name::<T>().to_owned(),
            ));
        }

        Ok(Some((len, type_name)))
    }

    /// Retrieve a **mutable** reference to the object stored in the state at index `i`.
    ///
    /// Returns a [`ValueError`] if the type `T` doesn't match the type of the object at index `i`.
    /// Returns `None` if `i` is out of bounds
    pub fn object<T: ParallelStateType>(
        &self,
        i: usize,
    ) -> Result<Option<&'static mut T>, ValueError> {
        if i >= self.len {
            return Ok(None);
        }

        let i = i * 2;
        unsafe {
            let Some((_, _)) = self.decode_info::<T>(i)? else {
                return Ok(None);
            };

            let idx: u64 = TocKeys::UserState.into();
            let idx = idx + i as u64 + 1;
            let ptr = pg_sys::shm_toc_lookup(self.toc.as_ptr(), idx, true);
            if ptr.is_null() {
                return Ok(None);
            }
            Ok(Some(&mut *ptr.cast()))
        }
    }

    /// Retrieve a slice to the Vec of objects stored in the state at index `i`.
    ///
    /// Returns a [`ValueError`] if the type `T` doesn't match the type of the slice elements at index `i`.
    /// Returns `None` if `i` is out of bounds
    pub fn slice<T: ParallelStateType>(
        &self,
        i: usize,
    ) -> Result<Option<&'static [T]>, ValueError> {
        if i >= self.len {
            return Ok(None);
        }

        let i = i * 2;
        unsafe {
            let Some((len, _)) = self.decode_info::<T>(i)? else {
                return Ok(None);
            };

            let idx: u64 = TocKeys::UserState.into();
            let idx = idx + i as u64 + 1;
            let ptr = pg_sys::shm_toc_lookup(self.toc.as_ptr(), idx, true);
            if ptr.is_null() {
                return Ok(None);
            }
            let slice = std::slice::from_raw_parts(ptr.cast(), len);
            Ok(Some(slice))
        }
    }
}

/// This macro facilitates the creation and execution of a parallel process within the PostgreSQL environment using the `pgx` framework.
///
/// # Overview
/// The `launch_parallel_process` macro abstracts the boilerplate required to register, setup, and launch parallel processes with a specified state type and worker type. It ensures that the provided worker type and state type conform to the required traits and are appropriately verified and utilized in parallel execution.
///
/// # Syntax
///
/// ```text
/// launch_parallel_process!(
///     ParallelProcessType<ParallelWorkerType>,
///     parallel_process_instance,
///     number_of_workers,
///     message_queue_size
/// )
/// ```
///
/// - **ParallelProcessType**: A type that implements the [`ParallelProcess`] trait. Represents the process logic.
/// - **ParallelWorkerType**: The worker type that performs the parallel task. Must implement the [`ParallelWorker`] trait and must be zero-sized.
/// - **parallel_process_instance**: An instance of the specified [`ParallelProcess`].
/// - **number_of_workers**: The number of workers to be spawned for this parallel process.
/// - **message_queue_size**: The size of the shared message queue used for communication back to the leader
///
/// # Constraints
/// This macro enforces the following constraints to ensure runtime safety:
/// - **Trait Implementation Check**: `ParallelProcessType` must implement the  [`ParallelProcess`] trait. This checks is done via compile-time assertions.
///
/// # Generated Output
///
/// The macro generates:
/// - A `#[no_mangle]` and `#[pgrx::pg_guard]` exported function, which serves as the parallel entry point for PostgreSQL.
/// - A launch function that initializes the parallel process using the `ParallelProcessBuilder` and returns a handle to the running process, allowing the main thread to wait for worker attachment and execution.
///
/// # Example
///
/// ```rust,no_run
/// use pg_search::launch_parallel_process;
/// use pg_search::parallel_worker::{ParallelProcess, ParallelState, ParallelStateType, ParallelWorker};
/// use pg_search::parallel_worker::ParallelStateManager;
/// use pg_search::parallel_worker::mqueue::MessageQueueSender;
///
/// // Define a ParallelState type
/// #[repr(C)]
/// #[derive(Copy, Clone)]
/// struct MyState {
///     junk: u32
/// }
///
/// impl ParallelStateType for MyState {}
///
/// // Define the Worker type
/// struct MyWorker<'a> {
///    state: &'a MyState
/// };
/// impl ParallelWorker for MyWorker<'_> {
///
///     fn new(state_manager: ParallelStateManager) -> Self {
///         Self { state: state_manager.object(0).expect("wrong type").expect("null value") }
///     }
///
///     fn run(mut self, mq_sender: &MessageQueueSender, worker_number: i32) -> anyhow::Result<()> {
///         pgrx::warning!("junk={}", self.state.junk);
///         Ok(())
///     }
/// }
///
/// struct MyProcess {
///    state: MyState,
/// };
///
/// impl ParallelProcess for MyProcess {
///     fn state_values(&self) -> Vec<&dyn ParallelState> {
///         vec![&self.state]
///     }
/// }
///
/// impl MyProcess {
///     fn new(junk: u32) -> Self {
///         Self {
///             state: MyState {
///                 junk
///             }
///         }
///     }
/// }
///
/// let my_process = MyProcess::new(42);
///
/// // Launch the parallel process
/// let launched = launch_parallel_process!(
///     MyProcess<MyWorker>,
///     my_process,
///     4,     // Number of workers
///     1024  // Message queue size in bytes
/// )
/// .expect("Failed to launch parallel process");
///
/// // wait for the processes to finish
/// launched.wait_for_finish();
/// ```
#[macro_export]
macro_rules! launch_parallel_process {
    ($parallel_process_type:ident<$parallel_worker_type:ty>, $process:expr, $nworkers:expr, $mq_size:literal) => {{
        {
            const _: () = {
                const fn assert_is_parallel_worker<T: ParallelProcess>() {}
                assert_is_parallel_worker::<$parallel_process_type>();
            };

            #[allow(non_snake_case)]
            #[no_mangle]
            #[pgrx::pg_guard]
            pub unsafe extern "C-unwind" fn $parallel_process_type(
                seg: *mut pg_sys::dsm_segment,
                toc: *mut pg_sys::shm_toc,
            ) {
                let (state_manager, mq_sender) =
                    $crate::parallel_worker::generic_parallel_worker_entry_point(
                        seg,
                        toc,
                        $mq_size as usize,
                    );

                <$parallel_worker_type>::new(state_manager)
                    .run(&mq_sender, unsafe { pgrx::pg_sys::ParallelWorkerNumber })
                    .unwrap_or_else(|e| ::std::panic::panic_any(e));
            }
        }

        $crate::parallel_worker::builder::ParallelProcessBuilder::build(
            $process,
            stringify!($parallel_process_type),
            $nworkers,
            $mq_size,
        )
        .map(|launcher| launcher.launch())
        .flatten()
        .map(|waiter| waiter.wait_for_attach())
        .flatten()
    }};
}

#[doc(hidden)]
pub unsafe fn generic_parallel_worker_entry_point(
    seg: *mut pg_sys::dsm_segment,
    toc: *mut pg_sys::shm_toc,
    mq_size: usize,
) -> (ParallelStateManager, MessageQueueSender) {
    let mqueues_base = get_toc_entry(toc, TocKeys::MessageQueues)
        .map(|value| value.as_ptr())
        .expect("message queue should exist in toc");

    let mq_size = MAXALIGN_DOWN(mq_size);
    let mq = mqueues_base
        .add(pg_sys::ParallelWorkerNumber as usize * mq_size)
        .cast::<pg_sys::shm_mq>();

    let state_manager = ParallelStateManager::new(toc);
    let mq_sender = MessageQueueSender::new(seg, mq);
    (state_manager, mq_sender)
}

#[inline(always)]
unsafe fn get_toc_entry(
    shm_toc: *mut pg_sys::shm_toc,
    key: impl Into<u64>,
) -> Option<NonNull<std::ffi::c_void>> {
    unsafe { NonNull::new(pg_sys::shm_toc_lookup(shm_toc, key.into(), true)) }
}

/*
#define shm_toc_estimate_chunk(e, sz) \
    ((e)->space_for_chunks = add_size((e)->space_for_chunks, BUFFERALIGN(sz)))
*/
#[doc(hidden)]
unsafe fn estimate_chunk(pcxt: *mut pg_sys::ParallelContext, sz: usize) {
    const BUFFERALIGN: fn(usize) -> usize =
        |len: usize| unsafe { pg_sys::TYPEALIGN(pg_sys::ALIGNOF_BUFFER as usize, len) };

    let estimator = &mut (*pcxt).estimator;
    estimator.space_for_chunks = pg_sys::add_size(estimator.space_for_chunks, BUFFERALIGN(sz));
}

/*
#define shm_toc_estimate_keys(e, cnt) \
    ((e)->number_of_keys = add_size((e)->number_of_keys, cnt))
*/
#[doc(hidden)]
unsafe fn estimate_keys(pcxt: *mut pg_sys::ParallelContext, cnt: usize) {
    let estimator = &mut (*pcxt).estimator;
    estimator.number_of_keys = pg_sys::add_size(estimator.number_of_keys, cnt);
}

/*
#define MAXALIGN_DOWN(LEN)		TYPEALIGN_DOWN(MAXIMUM_ALIGNOF, (LEN))
 */
#[allow(non_snake_case)]
const fn MAXALIGN_DOWN(LEN: usize) -> usize {
    TYPEALIGN_DOWN(pg_sys::MAXIMUM_ALIGNOF as usize, LEN)
}

/*
#define TYPEALIGN_DOWN(ALIGNVAL,LEN)  \
    (((uintptr_t) (LEN)) & ~((uintptr_t) ((ALIGNVAL) - 1)))
 */
#[allow(non_snake_case)]
const fn TYPEALIGN_DOWN(ALIGNVAL: usize, LEN: usize) -> usize {
    LEN & !(ALIGNVAL - 1)
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use crate::parallel_worker::mqueue::MessageQueueSender;
    use crate::parallel_worker::{
        ParallelProcess, ParallelState, ParallelStateManager, ParallelStateType, ParallelWorker,
    };
    use pgrx::pg_test;
    use pgrx::prelude::*;

    #[pg_test]
    fn test_parallel_workers() {
        #[repr(C)]
        #[derive(Copy, Clone)]
        struct MyState {
            junk: u32,
        }

        impl ParallelStateType for MyState {}

        // Define the Worker type
        struct MyWorker<'a> {
            state: &'a MyState,
        }

        impl ParallelWorker for MyWorker<'_> {
            fn new(state_manager: ParallelStateManager) -> Self {
                Self {
                    state: state_manager
                        .object(0)
                        .expect("wrong type")
                        .expect("null value"),
                }
            }

            fn run(self, mq_sender: &MessageQueueSender, worker_number: i32) -> anyhow::Result<()> {
                assert_eq!(self.state.junk, 42);
                Ok(mq_sender.send(worker_number.to_ne_bytes())?)
            }
        }

        struct MyProcess {
            state: MyState,
        }

        impl ParallelProcess for MyProcess {
            fn state_values(&self) -> Vec<&dyn ParallelState> {
                vec![&self.state]
            }
        }

        impl MyProcess {
            fn new(junk: u32) -> Self {
                Self {
                    state: MyState { junk },
                }
            }
        }

        // Launch the parallel process
        let process = launch_parallel_process!(
            MyProcess<MyWorker>,
            MyProcess::new(42),
            2,    // Number of workers
            1024  // Message queue size in bytes
        )
        .expect("Failed to launch parallel process");

        assert_eq!(
            process.launched_workers(),
            2,
            "didn't launch correct number of workers"
        );
        for (worker_number, message) in process {
            let reply_worker_number = i32::from_ne_bytes(message.as_slice().try_into().unwrap());
            assert_eq!(
                worker_number as i32, reply_worker_number,
                "mix-matched reply from worker"
            )
        }
    }
}
