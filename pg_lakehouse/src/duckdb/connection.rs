use duckdb::Connection;
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
