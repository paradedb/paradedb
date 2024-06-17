use anyhow::Result;
use async_std::sync::RwLock;
use duckdb::{Arrow, Connection};
use std::cell::RefCell;
use std::sync::Arc;

thread_local! {
    static THREAD_LOCAL_DATA: RefCell<Arc<Connection>> = RefCell::new(
        Arc::new(Connection::open_in_memory().expect("failed to open duckdb connection")
    ));
}

pub fn duckdb_connection() -> Arc<Connection> {
    THREAD_LOCAL_DATA.with(|data| data.borrow().clone())
}

pub struct ConnectionWrapper<'a> {
    inner_connection: Arc<Connection>,
    pub arrow: Option<Arc<RwLock<Arrow<'a>>>>,
}

impl<'a> ConnectionWrapper<'a> {
    pub fn new() -> Result<Self> {
        Ok(Self {
            inner_connection: Arc::new(Connection::open_in_memory()?),
            arrow: None,
        })
    }
}
