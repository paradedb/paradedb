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

use crate::postgres::parallel_worker::mqueue::MessageQueueSender;
use pgrx::pg_sys;
use std::ptr::NonNull;

#[repr(u64)]
enum TocKeys {
    SharedState = 1,
    MessageQueues = 2,
}

impl From<TocKeys> for u64 {
    fn from(key: TocKeys) -> Self {
        key as _
    }
}

#[allow(dead_code)]
pub trait ParallelState {}

pub trait ParallelProcess {
    fn size_of_state(&self) -> usize;

    /// # Safety
    ///
    /// This function is unsafe because we cannot ensure the `dest` pointer being provided is what
    /// your implementation wants.  It's on you to make sure you've defined things properly when
    /// calling the `launch_parallel_process!()` macro
    unsafe fn init_shm_state(&self, dest: *mut std::ffi::c_void);
}

pub trait ParallelWorker {
    type State;
    fn run(
        shm_state: &mut Self::State,
        mq_sender: &MessageQueueSender,
        worker_number: i32,
    ) -> anyhow::Result<()>;
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
///     ParallelProcessType<ParallelStateType, ParallelWorkerType>,
///     parallel_process_instance,
///     number_of_workers,
///     message_queue_size
/// )
/// ```
///
/// - **ParallelProcessType**: A type that implements the [`ParallelProcess`] trait. Represents the process logic.
/// - **ParallelStateType**: The shared state in memory used across workers. Must implement the [`ParallelState`] trait.
/// - **ParallelWorkerType**: The worker type that performs the parallel task. Must implement the [`ParallelWorker`] trait and must be zero-sized.
/// - **parallel_process_instance**: An instance of the specified [`ParallelProcess`].
/// - **number_of_workers**: The number of workers to be spawned for this parallel process.
/// - **message_queue_size**: The size of the shared message queue used for communication back to the leader
///
/// # Constraints
/// This macro enforces the following constraints to ensure runtime safety:
/// - **Zero-Sized Worker Type**: The `ParallelWorkerType` must be a zero-sized type. An assertion enforces this at compile time.
/// - **Trait Implementation Check**: Both the `ParallelStateType` and the `ParallelProcessType` must implement the [`ParallelState` ]and [`ParallelProcess`] traits respectively. These checks are done via compile-time assertions.
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
/// use std::ffi::c_void;
/// use std::ptr::addr_of;
/// use pg_search::launch_parallel_process;
/// use pg_search::postgres::parallel_worker::{ParallelProcess, ParallelState, ParallelWorker};
///
/// use pg_search::postgres::parallel_worker::mqueue::MessageQueueSender;
///
/// // Define a ParallelState type
/// #[repr(C)]
/// struct MyParallelState {
///     junk: u32
/// }
/// impl ParallelState for MyParallelState {}
///
/// // Define the Worker type
/// struct MyWorker;
/// impl ParallelWorker for MyWorker {
///     type State = MyParallelState;
///
///     fn run(state: &mut Self::State, mq_sender: &MessageQueueSender, worker_number: i32) {
///         pgrx::warning!("junk={}", state.junk);
///     }
/// }
///
/// struct MyProcess {
///    state: MyParallelState,
/// };
///
/// impl ParallelProcess for MyProcess {
///     fn size_of_state(&self) -> usize {
///         size_of::<MyParallelState>()
///     }
///
///     unsafe fn init_shm_state(&self, dest: *mut c_void) {
///         unsafe {
///             std::ptr::copy_nonoverlapping(addr_of!(self.state) as *c_void, dest, self.size_of_state());
///         }
///     }
/// }
///
/// impl MyProcess {
///     fn new(junk: u32) -> Self {
///         Self {
///             state: MyParallelState {
///                 junk
///             }   
///         }       
///     }
/// }
///
///
///
/// let my_process = MyProcess::new(42);
///
/// // Launch the parallel process
/// let launched = launch_parallel_process!(
///     MyProcess<MyParallelState, MyWorker>,
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
    ($parallel_process_type:ident<$parallel_state_type:ty, $parallel_worker_type:ty>, $process:expr, $nworkers:expr, $mq_size:literal) => {{
        {
            const _: () = {
                assert!(
                    size_of::<$parallel_worker_type>() == 0,
                    "ParallelWorker must be zero sized"
                );

                const fn assert_is_parallel_worker<T: ParallelProcess>() {}
                assert_is_parallel_worker::<$parallel_process_type>();

                const fn assert_is_parallel_state<T: ParallelState>() {}
                assert_is_parallel_state::<$parallel_state_type>();
            };

            #[allow(non_snake_case)]
            #[no_mangle]
            #[pgrx::pg_guard]
            pub unsafe extern "C-unwind" fn $parallel_process_type(
                seg: *mut pg_sys::dsm_segment,
                toc: *mut pg_sys::shm_toc,
            ) {
                let (state_ptr, mq_sender) =
                    $crate::postgres::parallel_worker::generic_parallel_worker_entry_point(
                        seg,
                        toc,
                        $mq_size as usize,
                    );

                let state = &mut *state_ptr.cast::<$parallel_state_type>();
                let _ = <$parallel_worker_type as $crate::postgres::parallel_worker::ParallelWorker>::run(
                    state, &mq_sender, unsafe { pgrx::pg_sys::ParallelWorkerNumber }
                ).unwrap_or_else(|e| ::std::panic::panic_any(e));
            }
        }

        $crate::postgres::parallel_worker::builder::ParallelProcessBuilder::build::<
            _,
            $parallel_state_type,
        >(
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
) -> (*mut std::ffi::c_void, MessageQueueSender) {
    let shm_state = get_toc_entry(toc, TocKeys::SharedState)
        .map(|value| value.as_ptr())
        .expect("worker state should exist in toc");
    let mqueues_base = get_toc_entry(toc, TocKeys::MessageQueues)
        .map(|value| value.as_ptr())
        .expect("message queue should exist in toc");

    let mq_size = MAXALIGN_DOWN(mq_size);
    let mq = mqueues_base
        .add(pg_sys::ParallelWorkerNumber as usize * mq_size)
        .cast::<pg_sys::shm_mq>();

    let mq_sender = MessageQueueSender::new(seg, mq);
    (shm_state, mq_sender)
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
