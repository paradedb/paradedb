// Copyright (c) 2023-2024 Retake, Inc.
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
/// The client is agnostic to which index we're writing to, so keeping a global one
/// ensures that the instance can be re-used if a single transaction needs to write to
/// multiple indexes. Note this must NOT be accessed before the server is started.
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
