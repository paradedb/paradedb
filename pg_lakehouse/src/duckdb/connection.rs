use crate::once_cell::sync::Lazy;
use anyhow::{anyhow, Result};
use async_std::stream::StreamExt;
use duckdb::arrow::array::RecordBatch;
use duckdb::{Arrow, Connection, Params, Statement};
use signal_hook::consts::signal::*;
use signal_hook_async_std::Signals;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::Notify;
use tokio::task::{block_in_place, spawn_local, LocalSet};

thread_local! {
    static THREAD_LOCAL_CONNECTION: Rc<RefCell<Connection>> = Rc::new(RefCell::new(
        Connection::open_in_memory().expect("failed to open duckdb connection")
    ));
    static THREAD_LOCAL_STATEMENT: Rc<RefCell<Option<Statement<'static>>>> = Rc::new(RefCell::new(None));
    static THREAD_LOCAL_ARROW: Rc<RefCell<Option<duckdb::Arrow<'static>>>> = Rc::new(RefCell::new(None));
}

async fn await_cancel() -> Result<()> {
    let mut signals = Signals::new([SIGTERM, SIGINT, SIGQUIT])?;
    signals.next().await;
    pgrx::info!("await_cancel done");
    Ok(())
}

async fn create_arrow_impl(sql: &str) -> Result<bool> {
    let local = LocalSet::new();
    let sql = sql.to_string();

    local
        .run_until(async move {
            spawn_local(async move {
                THREAD_LOCAL_CONNECTION.with(|connection| -> Result<bool> {
                    let conn = connection.borrow_mut();
                    let statement = conn.prepare(&sql)?;
                    let static_statement: Statement<'static> =
                        unsafe { std::mem::transmute(statement) };

                    THREAD_LOCAL_STATEMENT.with(|stmt| {
                        *stmt.borrow_mut() = Some(static_statement);
                    });

                    THREAD_LOCAL_STATEMENT.with(|stmt| -> Result<()> {
                        let mut borrowed_statement = stmt.borrow_mut();
                        if let Some(static_statement) = borrowed_statement.as_mut() {
                            pgrx::info!("querying arrow");
                            let arrow = static_statement.query_arrow([])?;
                            pgrx::info!("got arrow");
                            THREAD_LOCAL_ARROW.with(|arr| {
                                *arr.borrow_mut() = Some(unsafe {
                                    std::mem::transmute::<duckdb::Arrow<'_>, duckdb::Arrow<'_>>(
                                        arrow,
                                    )
                                });
                            });
                        }
                        Ok(())
                    })?;

                    Ok(true)
                })
            })
            .await?
        })
        .await
}

pub async fn create_arrow(sql: &str) -> Result<bool> {
    let cancel_task = tokio::spawn(async move { await_cancel().await });

    tokio::select! {
        _ = cancel_task => {
            println!("Cancellation task completed");
            Ok(false)
        }
        query_result = create_arrow_impl(sql) => {
            query_result
        }
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
