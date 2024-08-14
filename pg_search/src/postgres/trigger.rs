use anyhow::{anyhow, Result};
use pgrx::*;

use crate::{
    env::register_commit_callback, globals::WriterGlobal, index::SearchIndex,
    writer::WriterDirectory,
};

#[pg_extern(sql = "
    CREATE FUNCTION delete_trigger() 
    RETURNS trigger 
    LANGUAGE c 
    AS 'MODULE_PATHNAME', '@FUNCTION_NAME@';
")]
unsafe fn delete_trigger(fcinfo: pg_sys::FunctionCallInfo) {
    delete_trigger_impl(fcinfo).unwrap_or_else(|err| panic!("{err}"));
}

#[inline]
fn delete_trigger_impl(fcinfo: pg_sys::FunctionCallInfo) -> Result<()> {
    let trigger = unsafe {
        PgTrigger::from_fcinfo(fcinfo.as_ref().ok_or_else(|| anyhow!("fcinfo is null"))?)?
    };

    let extra_args = trigger.extra_args()?;
    let index_oid = extra_args[0].parse::<u32>()?;

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
