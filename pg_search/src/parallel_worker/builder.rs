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

use crate::parallel_worker::mqueue::MessageQueueReceiver;
use crate::parallel_worker::{
    estimate_chunk, estimate_keys, ParallelProcess, ParallelStateManager, TocKeys, WorkerStyle,
    MAXALIGN_DOWN,
};
use pgrx::pg_sys;
use std::ffi::CString;
use std::ptr::NonNull;

pub struct ParallelProcessBuilder;

impl ParallelProcessBuilder {
    pub fn build<P: ParallelProcess>(
        process: P,
        fn_name: &'static str,
        worker_style: WorkerStyle,
        nworkers: usize,
        mq_size: usize,
    ) -> Option<ParallelProcessLauncher> {
        unsafe {
            let nworkers = nworkers
                .min(pg_sys::max_worker_processes as _)
                .min(worker_style.max())
                .min(pg_sys::max_parallel_workers as _);

            let mq_size = MAXALIGN_DOWN(mq_size);
            let fn_name = CString::new(fn_name).unwrap();

            let state_entries = process.state_values();
            let nmq_bytes = pg_sys::mul_size(mq_size, nworkers);

            pg_sys::EnterParallelMode();
            let pcxt = NonNull::new_unchecked(pg_sys::CreateParallelContext(
                c"pg_search".as_ptr(),
                fn_name.as_ptr(),
                nworkers as _,
            ));

            // for the message queues
            estimate_keys(pcxt.as_ptr(), 1);
            estimate_chunk(pcxt.as_ptr(), nmq_bytes);

            // for user state
            estimate_keys(pcxt.as_ptr(), 1);
            estimate_chunk(pcxt.as_ptr(), size_of::<usize>());
            for entry in &state_entries {
                // for the element length
                estimate_keys(pcxt.as_ptr(), 1);
                estimate_chunk(pcxt.as_ptr(), entry.info().len());

                // for the entry itself
                estimate_keys(pcxt.as_ptr(), 1);
                estimate_chunk(pcxt.as_ptr(), entry.size_of());
            }

            // initialize shared memory
            pg_sys::InitializeParallelDSM(pcxt.as_ptr());
            if (*pcxt.as_ptr()).seg.is_null() {
                // failed to initialize DSM
                pg_sys::DestroyParallelContext(pcxt.as_ptr());
                pg_sys::ExitParallelMode();
                return None;
            }

            // copy user state into shared memory
            let user_state_length_address =
                pg_sys::shm_toc_allocate((*pcxt.as_ptr()).toc, size_of::<usize>());
            std::ptr::copy_nonoverlapping(
                state_entries.len().to_ne_bytes().as_ptr().cast(),
                user_state_length_address,
                size_of::<usize>(),
            );
            pg_sys::shm_toc_insert(
                (*pcxt.as_ptr()).toc,
                TocKeys::UserStateLength.into(),
                user_state_length_address,
            );
            for (i, entry) in state_entries.into_iter().enumerate() {
                let i = i * 2;
                let idx: u64 = TocKeys::UserState.into();

                let info = entry.info();
                let info_address = pg_sys::shm_toc_allocate((*pcxt.as_ptr()).toc, info.len());
                std::ptr::copy_nonoverlapping(info.as_ptr(), info_address.cast(), info.len());
                pg_sys::shm_toc_insert((*pcxt.as_ptr()).toc, idx + i as u64, info_address);

                let nbytes = entry.size_of();
                let state_address = pg_sys::shm_toc_allocate((*pcxt.as_ptr()).toc, nbytes);
                std::ptr::copy_nonoverlapping(
                    entry.as_bytes().as_ptr().cast(),
                    state_address,
                    nbytes,
                );
                pg_sys::shm_toc_insert((*pcxt.as_ptr()).toc, idx + i as u64 + 1, state_address);
            }

            // setup the message queues
            let mut mq_receivers = Vec::with_capacity(nworkers);
            let mq_start_address = pg_sys::shm_toc_allocate((*pcxt.as_ptr()).toc, nmq_bytes);
            for i in 0..nworkers {
                let address = mq_start_address.add(i * mq_size);
                let receiver = MessageQueueReceiver::new(pcxt, address, mq_size);
                mq_receivers.push(receiver);
            }
            pg_sys::shm_toc_insert(
                (*pcxt.as_ptr()).toc,
                TocKeys::MessageQueues.into(),
                mq_start_address,
            );

            Some(ParallelProcessLauncher {
                pcxt,
                mq_handles: mq_receivers,
                state_manager: ParallelStateManager::new((*pcxt.as_ptr()).toc),
            })
        }
    }
}

pub struct ParallelProcessLauncher {
    pcxt: NonNull<pg_sys::ParallelContext>,
    state_manager: ParallelStateManager,
    mq_handles: Vec<MessageQueueReceiver>,
}

impl ParallelProcessLauncher {
    pub fn launch(self) -> Option<ParallelProcessAttach> {
        unsafe {
            let pcxt = self.pcxt.as_ptr();
            pg_sys::LaunchParallelWorkers(pcxt);

            // if workers were launched
            if (*pcxt).nworkers_launched > 0

                // or none were launched because caller didn't ask for any, but the leader is supposed to participate
                || ((*pcxt).nworkers_launched == 0
                && pg_sys::parallel_leader_participation)
            {
                // then we have a valid parallel process machine
                return Some(ParallelProcessAttach { launcher: self });
            }

            // no workers launched
            pg_sys::DestroyParallelContext(pcxt);
            pg_sys::ExitParallelMode();
            None
        }
    }
}

#[repr(transparent)]
pub struct ParallelProcessAttach {
    launcher: ParallelProcessLauncher,
}
impl ParallelProcessAttach {
    pub fn wait_for_attach(self) -> Option<ParallelProcessFinish> {
        unsafe {
            pg_sys::WaitForParallelWorkersToAttach(self.launcher.pcxt.as_ptr());
            Some(ParallelProcessFinish {
                launcher: self.launcher,
            })
        }
    }
}

#[repr(transparent)]
pub struct ParallelProcessFinish {
    launcher: ParallelProcessLauncher,
}

impl ParallelProcessFinish {
    pub fn launched_workers(&self) -> usize {
        unsafe { (*self.launcher.pcxt.as_ptr()).nworkers_launched as usize }
    }

    #[allow(dead_code)]
    pub fn state_manager(&self) -> &ParallelStateManager {
        &self.launcher.state_manager
    }
    pub fn state_manager_mut(&mut self) -> &mut ParallelStateManager {
        &mut self.launcher.state_manager
    }

    pub fn recv(&self) -> Option<Vec<(usize, Vec<u8>)>> {
        let nlaunched = unsafe { (*self.launcher.pcxt.as_ptr()).nworkers_launched as usize };
        let mut messages = Vec::with_capacity(nlaunched);

        // this is a blocking call and we'll keep trying to recv until all message queues are detached
        loop {
            let mut detached_cnt = 0;
            for (i, receiver) in self.launcher.mq_handles.iter().enumerate().take(nlaunched) {
                if let Ok(message) = receiver.recv() {
                    messages.push((i, message));
                } else {
                    detached_cnt += 1;
                }
            }

            if detached_cnt == nlaunched {
                break;
            }
        }

        if messages.is_empty() {
            // everyone is detached
            return None;
        }

        Some(messages)
    }

    pub fn try_recv(&self) -> Option<Vec<(usize, Vec<u8>)>> {
        let nlaunched = unsafe { (*self.launcher.pcxt.as_ptr()).nworkers_launched as usize };
        let mut detached_cnt = 0;
        let mut messages = Vec::with_capacity(nlaunched);
        for (i, receiver) in self.launcher.mq_handles.iter().enumerate().take(nlaunched) {
            match receiver.try_recv() {
                Ok(Some(message)) => messages.push((i, message)),
                Ok(None) => continue,
                Err(_) => {
                    detached_cnt += 1;
                }
            }
        }

        if detached_cnt == nlaunched {
            // all message queues are detached
            assert!(
                messages.is_empty(),
                "when all message queues are detached, messages should be empty"
            );
            return None;
        }

        Some(messages)
    }

    pub fn wait_for_finish(self) -> Vec<(usize, Vec<u8>)> {
        unsafe {
            let pcxt = self.launcher.pcxt.as_ptr();

            let messages = self.recv().unwrap_or_default();
            drop(self.launcher);

            pg_sys::WaitForParallelWorkersToFinish(pcxt);
            pg_sys::DestroyParallelContext(pcxt);
            pg_sys::ExitParallelMode();

            messages
        }
    }
}

pub struct ParallelProcessMessageQueue {
    finisher: Option<ParallelProcessFinish>,
    batch: Vec<(usize, Vec<u8>)>,
}

impl IntoIterator for ParallelProcessFinish {
    type Item = (usize, Vec<u8>);
    type IntoIter = ParallelProcessMessageQueue;

    fn into_iter(self) -> Self::IntoIter {
        ParallelProcessMessageQueue {
            finisher: Some(self),
            batch: Vec::new(),
        }
    }
}

impl Iterator for ParallelProcessMessageQueue {
    type Item = (usize, Vec<u8>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(next) = self.batch.pop() {
                return Some(next);
            }

            match self.finisher.as_ref()?.try_recv() {
                None => {
                    self.batch = self.finisher.take().unwrap().wait_for_finish();
                }
                Some(batch) => {
                    self.batch = batch;
                }
            }
        }
    }
}
