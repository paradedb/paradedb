use crate::postgres::parallel_worker::mqueue::{create_message_queue, MessageQueueReceiver};
use crate::postgres::parallel_worker::ParallelProcess;
use pgrx::pg_sys;
use rustc_hash::FxHashMap;
use std::ffi::CString;
use std::ptr::NonNull;

#[repr(u64)]
pub enum TocKeys {
    TypeName = 1,
    SharedState = 2,
    MessageQueueSize = 3,
    MessageQueues = 4,
}

impl TocKeys {
    pub const fn last() -> u64 {
        TocKeys::MessageQueues as u64
    }
}

impl From<TocKeys> for u64 {
    fn from(key: TocKeys) -> Self {
        key as _
    }
}

pub struct ParallelProcessBuilder<P: ParallelProcess> {
    pcxt: NonNull<pg_sys::ParallelContext>,
    entries: FxHashMap<u64, Vec<u8>>,
    nworkers: usize,
    process: P,
}

impl<P: ParallelProcess> ParallelProcessBuilder<P> {
    pub fn new(process: P, nworkers: usize) -> Self {
        unsafe {
            let type_name = CString::new(std::any::type_name::<P>()).expect("valid type name");
            let mut entries = FxHashMap::default();
            entries.insert(
                TocKeys::TypeName.into(),
                type_name.to_bytes_with_nul().to_vec(),
            );
            entries.insert(TocKeys::SharedState.into(), process.state());
            entries.insert(
                TocKeys::MessageQueueSize.into(),
                process.message_queue_size().to_ne_bytes().to_vec(),
            );

            pg_sys::EnterParallelMode();

            let pcxt = NonNull::new(pg_sys::CreateParallelContext(
                c"pg_search".as_ptr(),
                c"pg_search_parallel_worker_main".as_ptr(),
                nworkers as _,
            ))
            .expect("should be able to create a ParallelContext");

            Self {
                pcxt,
                entries,
                nworkers,
                process,
            }
        }
    }

    #[allow(dead_code)]
    pub fn push_state(&mut self, key: u64, value: &[u8]) {
        assert_ne!(key, 0, "key must be greater than 0");
        self.entries
            .insert(key + TocKeys::last() + 1, value.to_vec());
    }

    pub fn build(self) -> Option<ParallelProcessLauncher> {
        unsafe {
            let message_queue_size = self.process.message_queue_size();

            let pcxt = self.pcxt.as_ptr();

            for value in self.entries.values() {
                estimate_keys(pcxt, 1);
                estimate_chunk(pcxt, value.len());
            }

            // for the message queues
            estimate_keys(pcxt, 1);
            estimate_chunk(pcxt, pg_sys::mul_size(message_queue_size, self.nworkers));

            // initialize shared memory
            pg_sys::InitializeParallelDSM(pcxt);
            if (*pcxt).seg.is_null() {
                // failed to initialize DSM
                pg_sys::DestroyParallelContext(pcxt);
                pg_sys::ExitParallelMode();
                return None;
            }

            // copy the entries into shared memory
            for (key, value) in self.entries {
                let address = pg_sys::shm_toc_allocate((*pcxt).toc, value.len() as _);
                address.copy_from(value.as_ptr().cast(), value.len());
                pg_sys::shm_toc_insert((*pcxt).toc, key, address);
            }

            // setup the message queues
            let mut mq_handles = Vec::with_capacity(self.nworkers);
            let mq_start_address = pg_sys::shm_toc_allocate(
                (*pcxt).toc,
                pg_sys::mul_size(message_queue_size, self.nworkers),
            );
            for i in 0..self.nworkers {
                let address = mq_start_address.add(i * message_queue_size);
                let receiver = create_message_queue(self.pcxt, address, message_queue_size);
                mq_handles.push(receiver);
            }
            pg_sys::shm_toc_insert((*pcxt).toc, TocKeys::MessageQueues.into(), mq_start_address);

            Some(ParallelProcessLauncher {
                pcxt: self.pcxt,
                message_queue_handles: mq_handles,
            })
        }
    }
}

pub struct ParallelProcessLauncher {
    pcxt: NonNull<pg_sys::ParallelContext>,
    message_queue_handles: Vec<MessageQueueReceiver>,
}

impl ParallelProcessLauncher {
    pub fn launch(self) -> Option<ParallelProcessAttach> {
        unsafe {
            let pcxt = self.pcxt.as_ptr();
            pg_sys::LaunchParallelWorkers(pcxt);
            if (*self.pcxt.as_ptr()).nworkers_launched == 0 {
                // no workers launched
                pg_sys::DestroyParallelContext(pcxt);
                pg_sys::ExitParallelMode();
                return None;
            }
            Some(ParallelProcessAttach { launcher: self })
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
    pub fn recv(&self) -> Option<Vec<(usize, Vec<u8>)>> {
        let nlaunched = unsafe { (*self.launcher.pcxt.as_ptr()).nworkers_launched as usize };
        let mut messages = Vec::with_capacity(nlaunched);

        // this is a blocking call and we'll keep trying to recv until all message queues are detached
        loop {
            let mut detached_cnt = 0;
            for (i, receiver) in self
                .launcher
                .message_queue_handles
                .iter()
                .enumerate()
                .take(nlaunched)
            {
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
        for (i, receiver) in self
            .launcher
            .message_queue_handles
            .iter()
            .enumerate()
            .take(nlaunched)
        {
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

/*
#define shm_toc_estimate_chunk(e, sz) \
    ((e)->space_for_chunks = add_size((e)->space_for_chunks, BUFFERALIGN(sz)))
*/
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
unsafe fn estimate_keys(pcxt: *mut pg_sys::ParallelContext, cnt: usize) {
    let estimator = &mut (*pcxt).estimator;
    estimator.number_of_keys = pg_sys::add_size(estimator.number_of_keys, cnt);
}
