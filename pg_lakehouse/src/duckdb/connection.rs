use anyhow::{bail, Result};
use async_std::sync::RwLock;
use async_std::task;
use duckdb::{params, Connection, Error};
use std::cell::RefCell;
use std::collections::{hash_map::Entry::Vacant, HashMap};
use std::sync::Arc;
use std::thread;

thread_local! {
    static THREAD_LOCAL_DATA: RefCell<Arc<Connection>> = RefCell::new(
        Arc::new(Connection::open_in_memory().expect("failed to open duckdb connection")
    ));
}

pub fn duckdb_connection() -> Arc<Connection> {
    THREAD_LOCAL_DATA.with(|data| data.borrow().clone())
}
