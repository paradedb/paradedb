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

use pgrx::shmem::PgSharedMemoryInitialization;
use pgrx::{pg_guard, pg_sys};
use std::cell::Cell;
use std::sync::atomic::{AtomicI32, AtomicPtr, AtomicU64, Ordering};

#[repr(C)]
struct Receiver {
    key: AtomicU64,
    pid: AtomicI32,
}

struct Registry(AtomicPtr<Receiver>);
static REGISTRY: Registry = Registry(AtomicPtr::new(std::ptr::null_mut()));

impl PgSharedMemoryInitialization for Registry {
    type Value = bool;

    unsafe fn on_shmem_request(&'static self) {
        unsafe {
            pg_sys::RequestAddinShmemSpace(pg_sys::MaxBackends as usize * size_of::<Receiver>());
        }
    }

    unsafe fn on_shmem_startup(&'static self, _: bool) {
        unsafe {
            let lock = &raw mut (*pg_sys::MainLWLockArray.add(21)).lock;
            pg_sys::LWLockAcquire(lock, pg_sys::LWLockMode::LW_EXCLUSIVE);
            let mut found = false;
            let receivers = pg_sys::ShmemInitStruct(
                c"pg_search mutable wakeups".as_ptr(),
                pg_sys::MaxBackends as usize * size_of::<Receiver>(),
                &mut found,
            )
            .cast::<Receiver>();
            if !found {
                for slot in 0..pg_sys::MaxBackends as usize {
                    receivers.add(slot).write(Receiver {
                        key: AtomicU64::new(0),
                        pid: AtomicI32::new(0),
                    });
                }
            }
            self.0.store(receivers, Ordering::Release);
            pg_sys::LWLockRelease(lock);
        }
    }
}

#[allow(
    unexpected_cfgs,
    reason = "pgrx also expands branches for unsupported PG versions"
)]
pub(crate) fn init() {
    pgrx::pg_shmem_init!(REGISTRY = false);
}

thread_local! {
    static EXIT_REGISTERED: Cell<bool> = const { Cell::new(false) };
}

pub(crate) struct Registration(usize);

impl Registration {
    pub(crate) unsafe fn new(index_oid: pg_sys::Oid) -> Self {
        unsafe {
            #[cfg(any(feature = "pg17", feature = "pg18"))]
            let slot = pg_sys::MyProcNumber as usize;
            #[cfg(any(feature = "pg15", feature = "pg16"))]
            let slot = (*pg_sys::MyProc).pgprocno as usize;
            assert!(slot < pg_sys::MaxBackends as usize);
            let receiver = &*REGISTRY.0.load(Ordering::Acquire).add(slot);
            EXIT_REGISTERED.with(|registered| {
                if !registered.get() {
                    pg_sys::before_shmem_exit(Some(clear_on_exit), slot.into());
                    registered.set(true);
                }
            });
            receiver.pid.store(pg_sys::MyProcPid, Ordering::Relaxed);
            receiver.key.store(key(index_oid), Ordering::Release);
            Self(slot)
        }
    }
}

#[pg_guard]
unsafe extern "C-unwind" fn clear_on_exit(_code: i32, slot: pg_sys::Datum) {
    unsafe {
        (*REGISTRY.0.load(Ordering::Acquire).add(slot.value()))
            .key
            .store(0, Ordering::Release);
    }
}

crate::impl_safe_drop!(Registration, |self| {
    unsafe {
        clear_on_exit(0, self.0.into());
    }
});

unsafe fn key(index_oid: pg_sys::Oid) -> u64 {
    unsafe { ((u32::from(pg_sys::MyDatabaseId) as u64) << 32) | u32::from(index_oid) as u64 }
}

pub(crate) unsafe fn notify(index_oid: pg_sys::Oid) {
    unsafe {
        let key = key(index_oid);
        let receivers = REGISTRY.0.load(Ordering::Acquire);
        if receivers.is_null() {
            return;
        }
        for slot in 0..pg_sys::MaxBackends as usize {
            let receiver = &*receivers.add(slot);
            if receiver.key.load(Ordering::Acquire) != key {
                continue;
            }
            let pid = receiver.pid.load(Ordering::Relaxed);
            let proc = (*pg_sys::ProcGlobal).allProcs.add(slot);
            // Recheck publication and PGPROC reuse before touching the latch.
            if receiver.key.load(Ordering::Acquire) == key && (*proc).pid == pid {
                pg_sys::SetLatch(&raw mut (*proc).procLatch);
            }
        }
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use pgrx::prelude::*;

    #[pg_guard]
    unsafe extern "C-unwind" fn later_exit_callback(_code: i32, _arg: pg_sys::Datum) {}

    #[pg_test]
    unsafe fn frozen_mutable_wakes_only_its_registered_collector() {
        Spi::run(
            "CREATE TABLE wake_target (id bigint PRIMARY KEY, body text);
             CREATE TABLE wake_other (LIKE wake_target INCLUDING ALL);
             CREATE INDEX wake_target_idx ON wake_target USING paradedb (id, body)
                 WITH (mutable_segment_rows = 4, background_layer_sizes = '0');
             CREATE INDEX wake_other_idx ON wake_other USING paradedb (id, body)
                 WITH (mutable_segment_rows = 4, background_layer_sizes = '0');
             INSERT INTO wake_target VALUES (1, 'batch');
             INSERT INTO wake_other VALUES (1, 'batch');",
        )
        .unwrap();
        let index = Spi::get_one::<pg_sys::Oid>("SELECT 'wake_target_idx'::regclass::oid")
            .unwrap()
            .unwrap();
        let registration = Registration::new(index);
        let latch = &raw mut (*pg_sys::MyProc).procLatch;
        let is_set = || std::ptr::read_volatile(&raw const (*latch).is_set) != 0;

        pg_sys::ResetLatch(latch);
        Spi::run("INSERT INTO wake_other SELECT id, 'batch' FROM generate_series(2, 4) id")
            .unwrap();
        assert!(!is_set(), "a different index woke the collector");
        Spi::run("INSERT INTO wake_target VALUES (2, 'batch')").unwrap();
        assert!(!is_set(), "an unfrozen segment interrupted collection");
        Spi::run("INSERT INTO wake_target VALUES (3, 'batch'), (4, 'batch')").unwrap();
        assert!(
            is_set(),
            "freezing the registered index did not wake collection"
        );
        assert_eq!(
            Spi::get_one::<i64>("SELECT count(*) FROM wake_target WHERE body ||| 'batch'").unwrap(),
            Some(4)
        );

        pg_sys::before_shmem_exit(Some(later_exit_callback), 0.into());
        drop(registration);
        pg_sys::cancel_before_shmem_exit(Some(later_exit_callback), 0.into());
        pg_sys::ResetLatch(latch);
        notify(index);
        assert!(!is_set(), "dropped registration still received a wakeup");
        let registration = Registration::new(index);
        notify(index);
        assert!(is_set(), "the backend could not register again");
        drop(registration);
    }
}
