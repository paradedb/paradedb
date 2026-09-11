// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use crate::schema::SearchFieldType;
use pgrx::pg_sys::{FRAMEOPTION_NONDEFAULT, Query, WindowFunc};
use pgrx::{PgList, pg_sys};
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::nodecast;
use crate::postgres::customscan::aggregatescan::join_targetlist::{
    AggKind, classify_aggregate_oid, unwrap_to_var,
};
use crate::postgres::customscan::joinscan::planning::resolve_fast_field_from_join_sources;

use super::build::JoinSource;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
    pub field_type: Option<SearchFieldType>,
}
impl ColumnInfo {
    pub fn new(
        rti: pg_sys::Index,
        attno: pg_sys::AttrNumber,
        field_type: Option<SearchFieldType>,
    ) -> Self {
        Self {
            rti,
            attno,
            field_type,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ResultType(pub pg_sys::Oid);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowAgg {
    pub agg_type: SupportedWindowAggType,
    pub col_info: Option<ColumnInfo>,
    pub result_type: ResultType,
    pub resno: pg_sys::AttrNumber,
}
impl WindowAgg {
    pub fn arg_field_type(&self) -> Option<&SearchFieldType> {
        self.col_info.as_ref().and_then(|ci| ci.field_type.as_ref())
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WindowAggIndex(usize);
impl WindowAggIndex {
    pub fn as_col_name(&self) -> String {
        WindowAggColumn::new(*self).to_string()
    }
}

pub struct WindowAggColumn(WindowAggIndex);
impl WindowAggColumn {
    const PREFIX: &'static str = "window_agg_";

    pub fn new(index: WindowAggIndex) -> Self {
        WindowAggColumn(index)
    }

    #[allow(dead_code)]
    pub fn index(&self) -> WindowAggIndex {
        self.0
    }
}
impl fmt::Display for WindowAggColumn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", Self::PREFIX, self.0.0)
    }
}
impl TryFrom<&str> for WindowAggColumn {
    type Error = ();

    fn try_from(col_name: &str) -> Result<Self, Self::Error> {
        let index = col_name
            .strip_prefix(Self::PREFIX)
            .ok_or(())?
            .parse::<usize>()
            .map_err(|_| ())?;
        Ok(Self::new(WindowAggIndex(index)))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct WindowAggList(Vec<WindowAgg>);
impl WindowAggList {
    pub fn new(aggs: Vec<WindowAgg>) -> Self {
        Self(aggs)
    }

    pub fn get(&self, index: WindowAggIndex) -> Option<&WindowAgg> {
        self.0.get(index.0)
    }

    pub fn find_index_by_resno(&self, resno: pg_sys::AttrNumber) -> Option<WindowAggIndex> {
        self.0.iter().enumerate().find_map(|(i, wa)| {
            if wa.resno == resno {
                Some(WindowAggIndex(i))
            } else {
                None
            }
        })
    }
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

                let Some(ff) = resolve_fast_field_from_join_sources(sources, &var) else {
                    return Err("arguments to window aggregate must be fast fields".to_string());
                };

                Some(ColumnInfo::new(
                    var.varno as pg_sys::Index,
                    var.varattno,
                    ff.field_type().cloned(),
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
        resno,
    })
}

pub fn is_supported_window_agg_node(node: *mut pg_sys::Node) -> bool {
    if node.is_null() {
        return false;
    }
    if let Some(wf) = unsafe { nodecast!(WindowFunc, T_WindowFunc, node) } {
        let wf = unsafe { &*wf };
        return SupportedWindowAggType::from_funcoid(wf.winfnoid, wf.winstar).is_some();
    }
    false
}
