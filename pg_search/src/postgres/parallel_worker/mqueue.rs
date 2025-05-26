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
use pgrx::pg_sys;
use std::ptr::NonNull;

struct MessageQueueHandle {
    handle: NonNull<pg_sys::shm_mq_handle>,
}

impl Drop for MessageQueueHandle {
    fn drop(&mut self) {
        unsafe {
            pg_sys::shm_mq_detach(self.handle.as_ptr());
        }
    }
}

impl MessageQueueHandle {
    unsafe fn attach_sender(seg: *mut pg_sys::dsm_segment, mq: *mut pg_sys::shm_mq) -> Self {
        unsafe {
            pg_sys::shm_mq_set_sender(mq, pg_sys::MyProc);
            let handle = pg_sys::shm_mq_attach(mq, seg, std::ptr::null_mut());
            MessageQueueHandle {
                handle: NonNull::new_unchecked(handle),
            }
        }
    }

    unsafe fn attach_receiver(
        pcxt: NonNull<pg_sys::ParallelContext>,
        mq: *mut pg_sys::shm_mq,
    ) -> Self {
        unsafe {
            pg_sys::shm_mq_set_receiver(mq, pg_sys::MyProc);
            let handle = pg_sys::shm_mq_attach(mq, (*pcxt.as_ptr()).seg, std::ptr::null_mut());
            MessageQueueHandle {
                handle: NonNull::new_unchecked(handle),
            }
        }
    }

    fn as_ptr(&self) -> *mut pg_sys::shm_mq_handle {
        self.handle.as_ptr()
    }
}

pub struct MessageQueueSender {
    handle: MessageQueueHandle,
}

impl MessageQueueSender {
    #[doc(hidden)]
    pub(super) unsafe fn new(seg: *mut pg_sys::dsm_segment, mq: *mut pg_sys::shm_mq) -> Self {
        unsafe {
            Self {
                handle: MessageQueueHandle::attach_sender(seg, mq),
            }
        }
    }

    pub fn send<B: AsRef<[u8]>>(&self, msg: B) -> Result<(), pg_sys::shm_mq_result::Type> {
        unsafe {
            let msg = msg.as_ref();
            let result = pg_sys::shm_mq_send(
                self.handle.as_ptr(),
                msg.len(),
                msg.as_ptr() as *mut std::ffi::c_void,
                false,
                true,
            );

            match result {
                pg_sys::shm_mq_result::SHM_MQ_SUCCESS => Ok(()),
                other => Err(other),
            }
        }
    }

    pub fn try_send(&self, msg: &[u8]) -> Result<Option<()>, pg_sys::shm_mq_result::Type> {
        unsafe {
            let result = pg_sys::shm_mq_send(
                self.handle.as_ptr(),
                msg.len(),
                msg.as_ptr() as *mut std::ffi::c_void,
                true,
                true,
            );

            match result {
                pg_sys::shm_mq_result::SHM_MQ_SUCCESS => Ok(Some(())),
                pg_sys::shm_mq_result::SHM_MQ_WOULD_BLOCK => Ok(None),
                other => Err(other),
            }
        }
    }
}

pub struct MessageQueueReceiver {
    handle: MessageQueueHandle,
}

impl MessageQueueReceiver {
    pub(super) unsafe fn new(
        pcxt: NonNull<pg_sys::ParallelContext>,
        address: *mut std::ffi::c_void,
        size: usize,
    ) -> Self {
        unsafe {
            let mq = pg_sys::shm_mq_create(address, size);
            Self {
                handle: MessageQueueHandle::attach_receiver(pcxt, mq),
            }
        }
    }

    pub fn recv(&self) -> Result<Vec<u8>, pg_sys::shm_mq_result::Type> {
        unsafe {
            let mut len = 0usize;
            let mut msg = std::ptr::null_mut();
            let result = pg_sys::shm_mq_receive(self.handle.as_ptr(), &mut len, &mut msg, false);

            match result {
                pg_sys::shm_mq_result::SHM_MQ_SUCCESS => {
                    let result = Ok(std::slice::from_raw_parts(msg as *mut u8, len).to_vec());
                    result
                }
                other => Err(other),
            }
        }
    }

    pub fn try_recv(&self) -> Result<Option<Vec<u8>>, pg_sys::shm_mq_result::Type> {
        unsafe {
            let mut len = 0usize;
            let mut msg = std::ptr::null_mut();
            let result = pg_sys::shm_mq_receive(self.handle.as_ptr(), &mut len, &mut msg, true);

            match result {
                pg_sys::shm_mq_result::SHM_MQ_SUCCESS => {
                    let result = Ok(Some(
                        std::slice::from_raw_parts(msg as *mut u8, len).to_vec(),
                    ));
                    result
                }
                pg_sys::shm_mq_result::SHM_MQ_WOULD_BLOCK => Ok(None),
                other => Err(other),
            }
        }
    }
}
