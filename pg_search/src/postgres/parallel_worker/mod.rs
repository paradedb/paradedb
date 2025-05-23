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

//! Provides an abstract API over Postgres' parallel workers, allowing for the implementaiton of
//! different strategies for distributing work across multiple workers.

pub mod builder;
pub mod mqueue;

use crate::api::aggregate::ParallelAggregationProcess;
use crate::postgres::parallel_worker::builder::{
    ParallelProcessBuilder, ParallelProcessFinish, TocKeys,
};
use crate::postgres::parallel_worker::mqueue::{attach_to_message_queue, MessageQueueSender};
use parking_lot::RwLock;
use pgrx::{pg_guard, pg_sys};
use rustc_hash::FxHashMap;
use std::ffi::CStr;
use std::ptr::NonNull;
use std::sync::LazyLock;

static PARALLEL_PROCESS_TYPES: LazyLock<RwLock<FxHashMap<&'static str, Box<dyn ParallelProcess>>>> =
    LazyLock::new(Default::default);

pub fn register_parallel_processes() {
    PARALLEL_PROCESS_TYPES
        .write()
        .entry(std::any::type_name::<ParallelAggregationProcess>())
        .or_insert_with(|| Box::new(ParallelAggregationProcess::empty()));
}

fn create_parallel_worker(
    type_name: &'static str,
    state: *mut std::ffi::c_void,
    mq_sender: MessageQueueSender,
) -> Option<impl ParallelWorker> {
    PARALLEL_PROCESS_TYPES
        .read()
        .get(type_name)
        .map(|p| p.create_worker(state, mq_sender))
}

pub trait ParallelProcess: Send + Sync + 'static {
    fn empty() -> Self
    where
        Self: Sized;

    fn message_queue_size(&self) -> usize {
        65536
    }

    fn state(&self) -> Vec<u8>;

    fn create_worker(
        &self,
        state: *mut std::ffi::c_void,
        mq_sender: MessageQueueSender,
    ) -> Box<dyn ParallelWorker>;
}

pub trait ParallelWorker {
    unsafe fn run(&mut self);
}

impl ParallelWorker for Box<dyn ParallelWorker> {
    unsafe fn run(&mut self) {
        self.as_mut().run();
    }
}

pub fn begin_parallel_process<P: ParallelProcess>(
    process: P,
    nworkers: usize,
) -> Option<ParallelProcessFinish> {
    #[no_mangle]
    #[pg_guard]
    pub unsafe extern "C-unwind" fn pg_search_parallel_worker_main(
        seg: *mut pg_sys::dsm_segment,
        toc: *mut pg_sys::shm_toc,
    ) {
        register_parallel_processes();

        let type_name = get_toc_entry(toc, TocKeys::TypeName)
            .map(|value| CStr::from_ptr(value.as_ptr().cast()))
            .expect("type name should exist in toc");
        let state = get_toc_entry(toc, TocKeys::SharedState)
            .map(|value| value.as_ptr())
            .expect("worker state should exist in toc");
        let mqueue_size = get_toc_entry(toc, TocKeys::MessageQueueSize)
            .map(|value| value.as_ptr())
            .expect("message queue size should exist in toc");
        let mqueues_base = get_toc_entry(toc, TocKeys::MessageQueues)
            .map(|value| value.as_ptr())
            .expect("message queue should exist in toc");

        let mqueue_size = usize::from_ne_bytes(mqueue_size.cast::<[u8; 8]>().read());
        let mq = mqueues_base
            .add(pg_sys::ParallelWorkerNumber as usize * mqueue_size)
            .cast::<pg_sys::shm_mq>();

        let mq_sender = attach_to_message_queue(seg, mq);
        let mut worker = create_parallel_worker(type_name.to_str().unwrap(), state, mq_sender)
            .expect("should be able to create a worker");

        worker.run();
    }

    register_parallel_processes();

    let builder = ParallelProcessBuilder::<P>::new(process, nworkers);
    let launcher = builder.build()?;
    launcher.launch()?.wait_for_attach()
}

unsafe fn get_toc_entry(
    shm_toc: *mut pg_sys::shm_toc,
    key: impl Into<u64>,
) -> Option<NonNull<std::ffi::c_void>> {
    unsafe {
        let ptr = pg_sys::shm_toc_lookup(shm_toc, key.into(), true);
        NonNull::new(ptr)
    }
}
