use anyhow::{anyhow, Result};
use duckdb::arrow::array::RecordBatch;
use duckdb::{Connection, Params, Statement};
use signal_hook::consts::signal::*;
use signal_hook::iterator::Signals;
use std::cell::UnsafeCell;
use std::sync::{Arc, Once};
use std::thread;

// Global mutable static variables
static mut GLOBAL_CONNECTION: Option<UnsafeCell<Connection>> = None;
static mut GLOBAL_STATEMENT: Option<UnsafeCell<Option<Statement<'static>>>> = None;
static mut GLOBAL_ARROW: Option<UnsafeCell<Option<duckdb::Arrow<'static>>>> = None;
static INIT: Once = Once::new();

fn init_globals() {
    let conn = Connection::open_in_memory().expect("failed to open duckdb connection");
    unsafe {
        GLOBAL_CONNECTION = Some(UnsafeCell::new(conn));
        GLOBAL_STATEMENT = Some(UnsafeCell::new(None));
        GLOBAL_ARROW = Some(UnsafeCell::new(None));
    }

    thread::spawn(move || {
        let mut signals =
            Signals::new(&[SIGTERM, SIGINT, SIGQUIT]).expect("error registering signal listener");
        for _ in signals.forever() {
            unsafe {
                let conn = &mut *get_global_connection().get();
                conn.interrupt();
            };
            pgrx::log!("await_cancel done");
        }
    });
}

fn get_global_connection() -> &'static UnsafeCell<Connection> {
    INIT.call_once(|| {
        init_globals();
    });
    unsafe {
        GLOBAL_CONNECTION
            .as_ref()
            .expect("Connection not initialized")
    }
}

fn get_global_statement() -> &'static UnsafeCell<Option<Statement<'static>>> {
    INIT.call_once(|| {
        init_globals();
    });
    unsafe {
        GLOBAL_STATEMENT
            .as_ref()
            .expect("Statement not initialized")
    }
}

fn get_global_arrow() -> &'static UnsafeCell<Option<duckdb::Arrow<'static>>> {
    INIT.call_once(|| {
        init_globals();
    });
    unsafe { GLOBAL_ARROW.as_ref().expect("Arrow not initialized") }
}

pub fn create_arrow(sql: &str) -> Result<bool> {
    unsafe {
        let conn = &mut *get_global_connection().get();
        let statement = conn.prepare(&sql)?;
        let static_statement: Statement<'static> = std::mem::transmute(statement);

        *get_global_statement().get() = Some(static_statement);

        if let Some(static_statement) = get_global_statement().get().as_mut().unwrap() {
            let arrow = static_statement.query_arrow([])?;
            *get_global_arrow().get() = Some(std::mem::transmute(arrow));
        }
    }

    Ok(true)
}

pub fn clear_arrow() {
    unsafe {
        *get_global_statement().get() = None;
        *get_global_arrow().get() = None;
    }
}

pub fn get_next_batch() -> Result<Option<RecordBatch>> {
    unsafe {
        if let Some(arrow) = get_global_arrow().get().as_mut().unwrap() {
            Ok(arrow.next())
        } else {
            Err(anyhow!("No Arrow batches found in GLOBAL_ARROW"))
        }
    }
}

pub fn get_batches() -> Result<Vec<RecordBatch>> {
    unsafe {
        if let Some(arrow) = get_global_arrow().get().as_mut().unwrap() {
            Ok(arrow.collect())
        } else {
            Err(anyhow!("No Arrow batches found in GLOBAL_ARROW"))
        }
    }
}

pub fn has_results() -> bool {
    unsafe {
        get_global_arrow()
            .get()
            .as_ref()
            .map_or(false, |arrow| arrow.is_some())
    }
}

pub fn execute<P: Params>(sql: &str, params: P) -> Result<usize> {
    unsafe {
        let conn = &*get_global_connection().get();
        conn.execute(sql, params).map_err(|err| anyhow!("{err}"))
    }
}

pub fn view_exists(table_name: &str, schema_name: &str) -> Result<bool> {
    unsafe {
        let conn = &mut *get_global_connection().get();
        let mut statement = conn.prepare(format!("SELECT * from information_schema.tables WHERE table_schema = '{schema_name}' AND table_name = '{table_name}' AND table_type = 'VIEW'").as_str())?;
        match statement.query([])?.next() {
            Ok(Some(_)) => Ok(true),
            _ => Ok(false),
        }
    }
}
