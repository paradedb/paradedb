use anyhow::{bail, Result};
use async_std::sync::RwLock;
use async_std::task;
use duckdb::{params, Connection, Error};
use std::collections::{hash_map::Entry::Vacant, HashMap};
use std::sync::Arc;

pub struct ConnectionCache {
    connection: RwLock<Option<Arc<Connection>>>,
}

impl ConnectionCache {
    pub fn connection(&self) -> Result<Arc<Connection>> {
        {
            let mut write_lock = task::block_on(self.connection.write());
            if write_lock.is_none() {
                write_lock.replace(Arc::new(Connection::open_in_memory()?));
            }
        }

        Ok(task::block_on(self.connection.read())
            .as_ref()
            .unwrap()
            .clone())
    }
}
