use pgrx::pg_sys::{FRAMEOPTION_DEFAULTS, Query, WindowFunc};
use pgrx::{PgList, pg_sys};

use crate::postgres::customscan::aggregatescan::join_targetlist::AggKind;

pub struct WindowAgg {
    agg_type: AggKind,
    rti: pg_sys::Index,
    attno: pg_sys::AttrNumber,
    output_type: pg_sys::Oid,
}

pub fn extract_window_agg(wf: *const WindowFunc, parse: *const Query) -> Result<WindowAgg, String> {
    assert!(!wf.is_null());

    if unsafe { !(*wf).aggfilter.is_null() } {
        return Err("window function filter clause is not supported".to_string());
    }

    if unsafe { !(*wf).winagg } {
        return Err(
            "only simple (sum/min/max/avg/count) window functions are supported".to_string(),
        );
    }

    let clause = unsafe {
        PgList::<pg_sys::WindowClause>::from_pg((*parse).windowClause)
            .iter_ptr()
            .find(|wc| (**wc).winref == (*wf).winref)
            .expect("WindowFunc.winref should always match a clause")
    };

    if unsafe {
        !(*clause).partitionClause.is_null()
            || !(*clause).orderClause.is_null()
            || !((*clause).frameOptions & FRAMEOPTION_DEFAULTS as i32 == 0)
    } {
        return Err(
            "only bare window functions of the style 'agg OVER ()' are supported".to_string(),
        );
    }

    Ok(WindowAgg {})
}
