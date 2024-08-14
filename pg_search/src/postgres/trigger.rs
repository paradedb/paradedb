use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use pgrx::*;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::{
    env::register_commit_callback, globals::WriterGlobal, index::SearchIndex,
    writer::WriterDirectory,
};

static DELETED_CTIDS_MEMORY: Lazy<Mutex<HashMap<u32, Vec<u64>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[pg_extern(sql = "
    CREATE FUNCTION delete_trigger_row() 
    RETURNS trigger 
    LANGUAGE c 
    AS 'MODULE_PATHNAME', '@FUNCTION_NAME@';
")]
unsafe fn delete_trigger_row(fcinfo: pg_sys::FunctionCallInfo) {
    delete_trigger_row_impl(fcinfo).unwrap_or_else(|err| panic!("{err}"));
}

#[pg_extern(sql = "
    CREATE FUNCTION delete_trigger_stmt() 
    RETURNS trigger 
    LANGUAGE c 
    AS 'MODULE_PATHNAME', '@FUNCTION_NAME@';
")]
unsafe fn delete_trigger_stmt(fcinfo: pg_sys::FunctionCallInfo) {
    delete_trigger_stmt_impl(fcinfo).unwrap_or_else(|err| panic!("{err}"));
}

#[inline]
fn delete_trigger_row_impl(fcinfo: pg_sys::FunctionCallInfo) -> Result<()> {
    let trigger = unsafe {
        PgTrigger::from_fcinfo(fcinfo.as_ref().ok_or_else(|| anyhow!("fcinfo is null"))?)?
    };

    let extra_args = trigger.extra_args()?;
    let index_oid = extra_args[0].parse::<u32>()?;

    let deleted_tuple = trigger.old().ok_or_else(|| anyhow!("old tuple is null"))?;
    let item_pointer = unsafe { (*deleted_tuple.into_pg()).t_self };
    let ctid = item_pointer_to_u64(item_pointer);

    append_ctids(index_oid, vec![ctid]);
    Ok(())
}

#[inline]
fn delete_trigger_stmt_impl(fcinfo: pg_sys::FunctionCallInfo) -> Result<()> {
    let trigger = unsafe {
        PgTrigger::from_fcinfo(fcinfo.as_ref().ok_or_else(|| anyhow!("fcinfo is null"))?)?
    };

    let extra_args = trigger.extra_args()?;
    let index_oid = extra_args[0].parse::<u32>()?;

    let directory = WriterDirectory::from_index_oid(index_oid);
    let search_index = SearchIndex::from_disk(&directory)
        .unwrap_or_else(|err| panic!("error loading index from directory: {err}"));

    let writer_client = WriterGlobal::client();
    register_commit_callback(&writer_client, search_index.directory.clone())
        .expect("could not register commit callbacks for delete operation");

    let ctids = DELETED_CTIDS_MEMORY
        .lock()
        .unwrap()
        .remove(&index_oid)
        .unwrap_or_default();
    search_index.delete(&writer_client, ctids)?;

    Ok(())
}

fn append_ctids(index_oid: u32, ctids: Vec<u64>) {
    let mut deleted_ctids_memory = DELETED_CTIDS_MEMORY.lock().unwrap();

    if let Some(pending) = deleted_ctids_memory.get_mut(&index_oid) {
        pending.extend(ctids);
    } else {
        deleted_ctids_memory.insert(index_oid, ctids);
    }
}
