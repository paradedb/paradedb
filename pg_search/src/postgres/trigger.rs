use anyhow::{anyhow, Result};
use pgrx::*;

#[pg_extern(sql = "
    CREATE FUNCTION trigger_example() 
    RETURNS trigger 
    LANGUAGE c 
    AS 'MODULE_PATHNAME', '@FUNCTION_NAME@';
")]
unsafe fn trigger_example(fcinfo: pg_sys::FunctionCallInfo) {
    trigger_impl(fcinfo).unwrap_or_else(|err| {
        panic!("{err}")
    });
}

#[inline]
unsafe fn trigger_impl(fcinfo) -> Result<()> {
    let trigger = PgTrigger::from_fcinfo(fcinfo.as_ref().ok_or_else(|| anyhow!("fcinfo is null")))?;
    let deleted_tuple = trigger.old().ok_or_else(|| anyhow!("old tuple is null"))?;
    let item_pointer = unsafe { (*deleted_tuple.into_pg()).t_self };

    let relation = trigger.relation()?;

    Ok(())
}
