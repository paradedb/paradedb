use crate::once_cell::sync::Lazy;
use anyhow::{anyhow, Result};
use async_std::stream::StreamExt;
use duckdb::arrow::array::RecordBatch;
use duckdb::{Arrow, Connection, Params, Rows, Statement};
use signal_hook::consts::signal::*;
use signal_hook_async_std::Signals;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;

thread_local! {
    static THREAD_LOCAL_CONNECTION: Rc<RefCell<Arc<Connection>>> = Rc::new(RefCell::new(
        Arc::new(Connection::open_in_memory().expect("failed to open duckdb connection"))
    ));
    static THREAD_LOCAL_STATEMENT: Rc<RefCell<Option<Statement<'static>>>> = Rc::new(RefCell::new(None));
    static THREAD_LOCAL_ARROW: Rc<RefCell<Option<duckdb::Arrow<'static>>>> = Rc::new(RefCell::new(None));
}

async fn await_cancel(sender: Sender<bool>) -> Result<()> {
    pgrx::info!("await_cancel");
    let mut signals = Signals::new([SIGTERM, SIGINT, SIGQUIT])?;
    signals.next().await;
    sender.send(false)?;
    Ok(())
}

fn create_arrow_impl(sql: &str, sender: Sender<bool>) -> Result<()> {
    let sql = sql.to_string();

    THREAD_LOCAL_CONNECTION.with(|connection| -> Result<()> {
        let conn = connection.borrow_mut();
        let statement = conn.prepare(&sql)?;
        let static_statement: Statement<'static> = unsafe { std::mem::transmute(statement) };

        THREAD_LOCAL_STATEMENT.with(|stmt| {
            *stmt.borrow_mut() = Some(static_statement);
        });

        THREAD_LOCAL_STATEMENT.with(|stmt| -> Result<()> {
            let mut borrowed_statement = stmt.borrow_mut();
            if let Some(static_statement) = borrowed_statement.as_mut() {
                let arrow = static_statement.query_arrow([])?;
                THREAD_LOCAL_ARROW.with(|arr| {
                    *arr.borrow_mut() = Some(unsafe {
                        std::mem::transmute::<duckdb::Arrow<'_>, duckdb::Arrow<'_>>(arrow)
                    });
                });
            }
            Ok(())
        })?;

        sender.send(true)?;
        Ok(())
    })
}

pub fn inner_connection() -> Arc<Connection> {
    THREAD_LOCAL_CONNECTION.with(|connection| connection.borrow().clone())
}

pub async fn create_arrow(sql: &str) -> Result<bool> {
    let (sender, receiver): (Sender<bool>, Receiver<bool>) = mpsc::channel();
    let sender_clone = sender.clone();

    thread::spawn(move || async_std::task::block_on(await_cancel(sender_clone)));
    create_arrow_impl(sql, sender)?;
    
    match receiver.recv() {
        Ok(result) => {
            pgrx::info!("create_arrow result: {result}");
            Ok(result)
        }
        Err(err) => Err(anyhow!("{err}")),
    }
}

pub fn clear_arrow() {
    THREAD_LOCAL_STATEMENT.with(|stmt| {
        *stmt.borrow_mut() = None;
    });
    THREAD_LOCAL_ARROW.with(|arrow| {
        *arrow.borrow_mut() = None;
    });
}

pub fn get_next_batch() -> Result<Option<RecordBatch>> {
    THREAD_LOCAL_ARROW.with(|arrow| {
        let mut arrow = arrow.borrow_mut();
        if let Some(arrow) = arrow.as_mut() {
            Ok(arrow.next())
        } else {
            Err(anyhow!("No Arrow batches found in THREAD_LOCAL_ARROW"))
        }
    })
}

pub fn get_batches() -> Result<Vec<RecordBatch>> {
    THREAD_LOCAL_ARROW.with(|arrow| {
        let mut arrow = arrow.borrow_mut();
        if let Some(arrow) = arrow.as_mut() {
            Ok(arrow.collect())
        } else {
            Err(anyhow!("No Arrow batches found in THREAD_LOCAL_ARROW"))
        }
    })
}

pub fn has_results() -> bool {
    THREAD_LOCAL_ARROW.with(|arrow| arrow.borrow().is_some())
}

pub fn execute<P: Params>(sql: &str, params: P) -> Result<usize> {
    THREAD_LOCAL_CONNECTION.with(|connection| {
        let conn = connection.borrow();
        conn.execute(sql, params).map_err(|err| anyhow!("{err}"))
    })
}

pub fn view_exists(table_name: &str, schema_name: &str) -> Result<bool> {
    THREAD_LOCAL_CONNECTION.with(|connection| {
        let conn = connection.borrow_mut();
        let mut statement = conn.prepare(format!("SELECT * from information_schema.tables WHERE table_schema = '{schema_name}' AND table_name = '{table_name}' AND table_type = 'VIEW'").as_str())?;
        match statement.query([])?.next() {
            Ok(Some(_)) => Ok(true),
            _ => Ok(false),
        }
    })
}
