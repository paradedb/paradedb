use pgrx::pg_sys::{FRAMEOPTION_NONDEFAULT, Query, WindowFunc};
use pgrx::{PgList, pg_sys};
use serde::{Deserialize, Serialize};

use crate::postgres::customscan::aggregatescan::join_targetlist::{
    AggKind, classify_aggregate_oid, unwrap_to_var,
};
use crate::postgres::customscan::joinscan::planning::is_fast_field;

use super::build::JoinSource;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SupportedWindowAggType {
    Count,
    CountStar,
    Sum,
    Avg,
    Min,
    Max,
}
impl SupportedWindowAggType {
    pub fn from_funcoid(oid: pg_sys::Oid, aggstar: bool) -> Option<Self> {
        match classify_aggregate_oid(oid.to_u32(), aggstar, false) {
            Some(AggKind::Count) => Some(SupportedWindowAggType::Count),
            Some(AggKind::CountStar) => Some(SupportedWindowAggType::CountStar),
            Some(AggKind::Sum) => Some(SupportedWindowAggType::Sum),
            Some(AggKind::Avg) => Some(SupportedWindowAggType::Avg),
            Some(AggKind::Min) => Some(SupportedWindowAggType::Min),
            Some(AggKind::Max) => Some(SupportedWindowAggType::Max),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub rti: pg_sys::Index,
    pub attno: pg_sys::AttrNumber,
    pub resno: pg_sys::AttrNumber,
}
impl ColumnInfo {
    pub fn new(rti: pg_sys::Index, attno: pg_sys::AttrNumber, resno: pg_sys::AttrNumber) -> Self {
        Self { rti, attno, resno }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultType(pub pg_sys::Oid);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowAgg {
    pub agg_type: SupportedWindowAggType,
    pub col_info: Option<ColumnInfo>,
    pub result_type: ResultType,
}

pub fn extract_window_agg(
    wf: *const WindowFunc,
    sources: &[&JoinSource],
    parse: &Query,
    resno: pg_sys::AttrNumber,
) -> Result<WindowAgg, String> {
    assert!(!wf.is_null());
    let wf = unsafe { &*wf };

    if !wf.aggfilter.is_null() {
        return Err("window function filter clause is not supported".to_string());
    }

    if !wf.winagg {
        return Err(
            "only simple (sum/min/max/avg/count) window functions are supported".to_string(),
        );
    }

    let clause = unsafe {
        PgList::<pg_sys::WindowClause>::from_pg(parse.windowClause)
            .iter_ptr()
            .find(|wc| (**wc).winref == wf.winref)
            .expect("WindowFunc.winref should always match a clause")
    };
    assert!(!clause.is_null());
    let clause = unsafe { *clause };

    if !clause.partitionClause.is_null()
        || !clause.orderClause.is_null()
        || clause.frameOptions & FRAMEOPTION_NONDEFAULT as i32 != 0
    {
        return Err(
            "only bare window functions of the style 'agg OVER ()' are supported".to_string(),
        );
    }

    let Some(agg_type) = SupportedWindowAggType::from_funcoid(wf.winfnoid, wf.winstar) else {
        return Err("unsupported window function was provided".to_string());
    };

    let col_info = {
        let args = unsafe { PgList::<pg_sys::Node>::from_pg(wf.args) };
        match args.len() {
            0 => {
                assert!(wf.winstar); // count(*)
                None
            }
            1 => {
                let arg = args.get_ptr(0).unwrap();

                let var = unsafe {
                    unwrap_to_var(arg).ok_or_else(|| {
                        "window aggregate argument must be a direct column reference".to_string()
                    })?
                };
                assert!(!var.is_null());
                let var = unsafe { *var };

                if !is_fast_field(sources, &var) {
                    return Err("arguments to window aggregate must be fast fields".to_string());
                }

                Some(ColumnInfo::new(
                    var.varno as pg_sys::Index,
                    var.varattno,
                    resno,
                ))
            }
            _ => {
                return Err("multi-argument window aggregates are not supported".to_string());
            }
        }
    };

    Ok(WindowAgg {
        agg_type,
        col_info,
        result_type: ResultType(wf.wintype),
    })
}
