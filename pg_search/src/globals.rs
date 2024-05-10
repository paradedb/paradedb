use once_cell::sync::Lazy;
use pgrx::{PGRXSharedMemory, PgLwLock};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use crate::writer::{self, WriterRequest};

// This is global shared state for the writer background worker.
pub static WRITER_GLOBAL: PgLwLock<WriterGlobal> = PgLwLock::new();

/// A global singleton for the instance of the client to the background writer process.
/// The client is agnotistic to which index we're writing to, so keeping a global one
/// ensures that the instance can be re-used if a single transaction needs to write to
/// multiple indexes. Note this must NOT be accesssed before the server is started.
pub static mut SEARCH_INDEX_WRITER_CLIENT: Lazy<Arc<Mutex<writer::Client<writer::WriterRequest>>>> =
    Lazy::new(|| Arc::new(Mutex::new(writer::Client::from_global())));

#[derive(Copy, Clone, Default)]
pub struct WriterGlobal {
    pub addr: Option<SocketAddr>,
}

impl WriterGlobal {
    pub fn addr(&self) -> SocketAddr {
        self.addr
            .expect("could not access writer status, writer server may not have started.")
    }

    pub fn set_addr(&mut self, addr: SocketAddr) {
        self.addr = Some(addr);
    }

    pub fn client() -> Arc<Mutex<writer::Client<WriterRequest>>> {
        unsafe { SEARCH_INDEX_WRITER_CLIENT.clone() }
    }
}

unsafe impl PGRXSharedMemory for WriterGlobal {}

pub static SECONDS_IN_DAY: i64 = 24 * 60 * 60;
pub static MICROSECONDS_IN_SECOND: u32 = 1_000_000;
