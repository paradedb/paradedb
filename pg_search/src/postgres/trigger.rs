use anyhow::{anyhow, Result};
use pgrx::*;

use crate::{
    env::register_commit_callback, globals::WriterGlobal, index::SearchIndex,
    writer::WriterDirectory,
};

#[pg_extern(sql = "
    CREATE FUNCTION trigger_example() 
    RETURNS trigger 
    LANGUAGE c 
    AS 'MODULE_PATHNAME', '@FUNCTION_NAME@';
")]
unsafe fn trigger_example(oid: u32, fcinfo: pg_sys::FunctionCallInfo) {
    trigger_impl(oid, fcinfo).unwrap_or_else(|err| panic!("{err}"));
}

#[inline]
unsafe fn trigger_impl(index_oid: u32, fcinfo: pg_sys::FunctionCallInfo) -> Result<()> {
    let trigger =
        PgTrigger::from_fcinfo(fcinfo.as_ref().ok_or_else(|| anyhow!("fcinfo is null"))?)?;
    let deleted_tuple = trigger.old().ok_or_else(|| anyhow!("old tuple is null"))?;
    let item_pointer = unsafe { (*deleted_tuple.into_pg()).t_self };
    let ctid = item_pointer_to_u64(item_pointer);

    let directory = WriterDirectory::from_index_oid(index_oid);
    let search_index = SearchIndex::from_disk(&directory)
        .unwrap_or_else(|err| panic!("error loading index from directory: {err}"));

    let writer_client = WriterGlobal::client();
    register_commit_callback(&writer_client, search_index.directory.clone())
        .expect("could not register commit callbacks for delete operation");

    search_index.delete(&writer_client, ctid)?;

    Ok(())
}
