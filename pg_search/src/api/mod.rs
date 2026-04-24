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

// TODO: See https://github.com/pgcentralfoundation/pgrx/pull/2089
#![allow(for_loops_over_fallibles)]

mod admin;
pub mod aggregate;
pub mod builder_fns;
pub mod config;
pub mod operator;
pub mod tokenize;
pub mod tokenizers;
pub mod window_aggregate;

use pgrx::{
    direct_function_call, extension_sql, pg_cast, pg_sys, FromDatum, InOutFuncs, IntoDatum,
    PostgresType, StringInfo,
};

pub use aggregate::{
    agg_fn_oid, agg_funcoid, agg_with_solve_mvcc_funcoid, extract_solve_mvcc_from_const,
    MvccVisibility,
};
pub use rustc_hash::FxHashMap as HashMap;
pub use rustc_hash::FxHashSet as HashSet;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::ffi::CStr;
use std::fmt::{Debug, Display, Formatter};
use std::ops::Deref;
use tantivy::json_utils::split_json_path;

use crate::vector::metric::{l2_normalize_in_place, VectorMetric};
use crate::vector::PgVector;

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct Regex(regex::Regex);
impl Deref for Regex {
    type Target = regex::Regex;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Eq for Regex {}
impl PartialEq for Regex {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_str() == other.0.as_str()
    }
}
impl Serialize for Regex {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.0.as_str())
    }
}
impl<'de> Deserialize<'de> for Regex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let pattern = String::deserialize(deserializer)?;
        regex::Regex::new(&pattern)
            .map(Regex)
            .map_err(serde::de::Error::custom)
    }
}
impl Regex {
    pub fn new(pattern: &str) -> Result<Self, regex::Error> {
        regex::Regex::new(pattern).map(Regex)
    }
}

#[macro_export]
macro_rules! nodecast {
    ($type_:ident, $kind:ident, $node:expr) => {{
        let node = $node;
        pgrx::is_a(node.cast(), pgrx::pg_sys::NodeTag::$kind)
            .then(|| node.cast::<pgrx::pg_sys::$type_>())
    }};

    ($type_:ident, $kind:ident, $node:expr, true) => {{
        let node = $node;
        (node.is_null() || pgrx::is_a(node.cast(), pgrx::pg_sys::NodeTag::$kind))
            .then(|| node.cast::<pgrx::pg_sys::$type_>())
    }};
}

// came to life in pg15
pub type Cardinality = f64;

pub type Varno = i32;

#[allow(dead_code)]
pub trait AsBool {
    unsafe fn as_bool(&self) -> Option<bool>;
}

pub trait AsCStr {
    unsafe fn as_c_str(&self) -> Option<&std::ffi::CStr>;
}

impl AsBool for *mut pgrx::pg_sys::Node {
    unsafe fn as_bool(&self) -> Option<bool> {
        let node = nodecast!(Boolean, T_Boolean, *self)?;
        Some((*node).boolval)
    }
}

impl AsCStr for *mut pgrx::pg_sys::Node {
    unsafe fn as_c_str(&self) -> Option<&std::ffi::CStr> {
        let node = nodecast!(String, T_String, *self)?;
        Some(std::ffi::CStr::from_ptr((*node).sval))
    }
}

/// A type used whenever our builder functions require a fieldname.
#[derive(
    Debug, Clone, Ord, Eq, PartialOrd, PartialEq, Hash, Serialize, Deserialize, PostgresType,
)]
#[inoutfuncs]
pub struct FieldName(String);

impl Display for FieldName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for FieldName {
    fn as_ref(&self) -> &str {
        self
    }
}

impl std::ops::Deref for FieldName {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> From<T> for FieldName
where
    T: Into<String>,
{
    fn from(value: T) -> Self {
        FieldName(value.into())
    }
}

impl InOutFuncs for FieldName {
    fn input(input: &CStr) -> Self
    where
        Self: Sized,
    {
        FieldName(input.to_str().unwrap().to_owned())
    }

    fn output(&self, buffer: &mut StringInfo) {
        buffer.push_str(&self.0);
    }
}

impl FieldName {
    pub fn into_const(self) -> *mut pg_sys::Const {
        unsafe {
            pg_sys::makeConst(
                fieldname_typoid(),
                -1,
                pg_sys::Oid::INVALID,
                -1,
                self.into_datum().unwrap_unchecked(),
                false,
                false,
            )
        }
    }

    #[inline(always)]
    pub fn into_inner(self) -> String {
        self.0
    }

    pub fn root(&self) -> String {
        let json_path = split_json_path(self.0.as_str());
        if json_path.len() == 1 {
            self.0.clone()
        } else {
            json_path[0].clone()
        }
    }

    pub fn path(&self) -> Option<String> {
        if !self.0.as_str().contains('.') {
            return None;
        }

        let json_path = split_json_path(self.0.as_str());
        if json_path.len() == 1 {
            None
        } else {
            Some(json_path[1..].join("."))
        }
    }

    pub fn is_ctid(&self) -> bool {
        self.root() == "ctid"
    }
}

#[pg_cast(implicit)]
fn text_to_fieldname(field: String) -> FieldName {
    FieldName(field)
}

extension_sql!(
    r#"
    CREATE CAST (varchar AS paradedb.fieldname) WITH INOUT AS IMPLICIT;
    "#,
    name = "varchar_to_fieldname_cast",
    requires = [text_to_fieldname]
);

#[allow(unused)]
pub fn fieldname_typoid() -> pg_sys::Oid {
    unsafe {
        let oid = direct_function_call::<pg_sys::Oid>(
            pg_sys::regtypein,
            &[c"paradedb.FieldName".into_datum()],
        )
        .expect("type `paradedb.FieldName` should exist");
        if oid == pg_sys::Oid::INVALID {
            panic!("type `paradedb.FieldName` should exist");
        }
        oid
    }
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum SortDirection {
    AscNullsFirst,
    #[default]
    AscNullsLast,
    DescNullsFirst,
    DescNullsLast,
}

impl AsRef<str> for SortDirection {
    fn as_ref(&self) -> &str {
        match self {
            SortDirection::AscNullsFirst => "asc nulls first",
            SortDirection::AscNullsLast => "asc",
            SortDirection::DescNullsFirst => "desc",
            SortDirection::DescNullsLast => "desc nulls last",
        }
    }
}

impl Display for SortDirection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

impl From<SortDirection> for tantivy::collector::sort_key::ComparatorEnum {
    fn from(value: SortDirection) -> Self {
        match value {
            SortDirection::AscNullsLast => {
                tantivy::collector::sort_key::ComparatorEnum::ReverseNoneLower
            }
            SortDirection::DescNullsFirst => {
                tantivy::collector::sort_key::ComparatorEnum::NaturalNoneHigher
            }
            SortDirection::DescNullsLast => tantivy::collector::sort_key::ComparatorEnum::Natural,
            SortDirection::AscNullsFirst => tantivy::collector::sort_key::ComparatorEnum::Reverse,
        }
    }
}

impl From<SortDirection> for tantivy::aggregation::bucket::Order {
    fn from(value: SortDirection) -> Self {
        match value {
            SortDirection::AscNullsFirst | SortDirection::AscNullsLast => {
                tantivy::aggregation::bucket::Order::Asc
            }
            SortDirection::DescNullsFirst | SortDirection::DescNullsLast => {
                tantivy::aggregation::bucket::Order::Desc
            }
        }
    }
}

impl SortDirection {
    /// Determines sort direction from a Postgres sort operator OID.
    ///
    /// Returns `None` if the operator properties cannot be resolved (should not
    /// happen for valid `SortGroupClause` operators). Callers should bail out
    /// of the TopK optimization rather than guessing a direction.
    #[cfg(any(feature = "pg15", feature = "pg16", feature = "pg17"))]
    pub unsafe fn from_sort_op(sortop: pg_sys::Oid, nulls_first: bool) -> Option<Self> {
        let mut opfamily = pg_sys::InvalidOid;
        let mut opcintype = pg_sys::InvalidOid;
        let mut strategy: i16 = 0;
        if pg_sys::get_ordering_op_properties(sortop, &mut opfamily, &mut opcintype, &mut strategy)
        {
            let reverse = strategy as u32 == pg_sys::BTGreaterStrategyNumber;
            Some(match (reverse, nulls_first) {
                (true, true) => SortDirection::DescNullsFirst,
                (true, false) => SortDirection::DescNullsLast,
                (false, true) => SortDirection::AscNullsFirst,
                (false, false) => SortDirection::AscNullsLast,
            })
        } else {
            None
        }
    }

    /// Determines sort direction from a Postgres sort operator OID.
    #[cfg(feature = "pg18")]
    pub unsafe fn from_sort_op(sortop: pg_sys::Oid, nulls_first: bool) -> Option<Self> {
        let mut opfamily = pg_sys::InvalidOid;
        let mut opcintype = pg_sys::InvalidOid;
        let mut cmptype = pg_sys::CompareType::COMPARE_LT;
        if pg_sys::get_ordering_op_properties(sortop, &mut opfamily, &mut opcintype, &mut cmptype) {
            let reverse = cmptype == pg_sys::CompareType::COMPARE_GT;
            Some(match (reverse, nulls_first) {
                (true, true) => SortDirection::DescNullsFirst,
                (true, false) => SortDirection::DescNullsLast,
                (false, true) => SortDirection::AscNullsFirst,
                (false, false) => SortDirection::AscNullsLast,
            })
        } else {
            None
        }
    }

    pub fn is_asc(self) -> bool {
        matches!(self, Self::AscNullsFirst | Self::AscNullsLast)
    }

    pub fn is_nulls_first(self) -> bool {
        matches!(self, Self::AscNullsFirst | Self::DescNullsFirst)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum OrderByFeature {
    Score {
        rti: u32,
    },
    Field {
        name: FieldName,
        rti: u32,
    },
    /// A reference to a PostgreSQL variable (column) by its Range Table Index (RTI) and Attribute Number.
    ///
    /// This variant is primarily used by `JoinScan` to unambiguously identify columns across multiple
    /// relations in a join. Unlike `Field(FieldName)`, which relies on string matching and can be ambiguous
    /// (e.g., distinguishing `table.column` from `column.json_key`), `Var` provides a precise handle
    /// that maps directly to the plan's `RangeTblEntry`.
    ///
    /// It also allows for "deferred resolution" of column names, which is crucial for integration with
    /// execution engines like DataFusion where the final schema and aliases might not be fully resolved
    /// until execution time.
    Var {
        rti: pg_sys::Index,
        attno: pg_sys::AttrNumber,
        name: Option<String>,
    },
    NullTest {
        inner: Box<OrderByFeature>,
        nulltesttype: NullTestKind,
    },
    VectorDistance {
        name: FieldName,
        rti: u32,
        /// The query vector. Empty when `query_vector_param_id` is `Some` and
        /// has not yet been resolved at execution time.
        query_vector: Vec<f32>,
        /// `Some(paramid)` when the query vector is supplied as a Postgres
        /// `Param` (e.g. a prepared statement / generic-plan parameter binding).
        /// At execution time the basescan resolves it from
        /// `EState.es_param_list_info` and writes the floats into `query_vector`.
        #[serde(default)]
        query_vector_param_id: Option<i32>,
        /// Metric implied by the operator that drove this ORDER BY
        /// (`<->` → L2, `<=>` → Cosine, `<#>` → InnerProduct). Drives
        /// EXPLAIN output and whether the resolved query vector is
        /// L2-normalized for cosine/L2 ordering.
        metric: VectorMetric,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NullTestKind {
    IsNull,
    IsNotNull,
}

impl std::fmt::Display for OrderByFeature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Score { .. } => write!(f, "pdb.score()"),
            Self::Field { name, .. } => write!(f, "{name}"),
            Self::Var { name, .. } => write!(f, "{}", name.as_deref().unwrap_or("?")),
            Self::NullTest {
                inner,
                nulltesttype,
            } => {
                let test = match nulltesttype {
                    NullTestKind::IsNull => "IS NULL",
                    NullTestKind::IsNotNull => "IS NOT NULL",
                };
                write!(f, "{inner} {test}")
            }
            Self::VectorDistance { name, metric, .. } => {
                write!(f, "{name} {} vector", metric.operator())
            }
        }
    }
}

/// Simple ORDER BY information for serialization in PrivateData
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrderByInfo {
    pub feature: OrderByFeature,
    pub direction: SortDirection,
}

impl OrderByInfo {
    pub fn is_score(&self) -> bool {
        matches!(self.feature, OrderByFeature::Score { .. })
    }

    pub fn is_vector_distance(&self) -> bool {
        matches!(self.feature, OrderByFeature::VectorDistance { .. })
    }

    /// If the ORDER BY is a vector distance, return
    /// `(field_name, query_vector, metric)`. The query vector must
    /// already have been resolved (see `resolve_param`). Returns `None`
    /// for any other feature variant.
    pub fn as_vector_distance(&self) -> Option<(&FieldName, &[f32], VectorMetric)> {
        match &self.feature {
            OrderByFeature::VectorDistance {
                name,
                query_vector,
                metric,
                ..
            } => Some((name, query_vector.as_slice(), *metric)),
            _ => None,
        }
    }

    /// If this `OrderByInfo` carries a parameterized vector ORDER BY
    /// (`<-> $1` style, generic-plan prepared statement), look up the
    /// bound `Param` value in the executor's `es_param_list_info`,
    /// convert it to `Vec<f32>`, optionally L2-normalize for cosine,
    /// and overwrite `query_vector` so downstream search code sees a
    /// concrete vector.
    ///
    /// No-op for `OrderByFeature::VectorDistance` whose param ID is
    /// already `None` (i.e. the vector was a literal `Const`), and
    /// for every other `OrderByFeature` variant.
    pub unsafe fn resolve_param(&mut self, estate: *mut pgrx::pg_sys::EState) {
        let OrderByFeature::VectorDistance {
            query_vector,
            query_vector_param_id,
            metric,
            ..
        } = &mut self.feature
        else {
            return;
        };
        let Some(paramid) = *query_vector_param_id else {
            return;
        };
        let param_list = (*estate).es_param_list_info;
        assert!(
            !param_list.is_null(),
            "es_param_list_info is NULL but vector ORDER BY references Param ${paramid}"
        );
        let idx = (paramid - 1) as usize;
        assert!(
            idx < (*param_list).numParams as usize,
            "vector ORDER BY param_id {paramid} out of range (numParams={})",
            (*param_list).numParams
        );

        // Materialize the param value. If the slot is already evaluated
        // (PREPARE/EXECUTE-style binding sets PARAM_FLAG_CONST up front),
        // read it directly. Otherwise (plpgsql SPI's lazy path) invoke
        // `paramFetch` with a stack-local workspace, matching what the
        // normal executor does in `ExecEvalParamExtern`. Passing
        // `prm=NULL` to plpgsql's fetch callback crashes — it writes into
        // the caller-provided buffer when non-null and expects one for
        // out-of-band reads.
        let slot = &(*param_list)
            .params
            .as_slice((*param_list).numParams as usize)[idx];
        let (value, isnull) = if (slot.pflags & pg_sys::PARAM_FLAG_CONST as u16) != 0 {
            (slot.value, slot.isnull)
        } else if let Some(fetch) = (*param_list).paramFetch {
            let mut prmdata = pg_sys::ParamExternData {
                value: pg_sys::Datum::null(),
                isnull: true,
                pflags: 0,
                ptype: pg_sys::InvalidOid,
            };
            let prm = fetch(param_list, paramid, false, &mut prmdata);
            assert!(!prm.is_null(), "paramFetch returned NULL for ${paramid}");
            ((*prm).value, (*prm).isnull)
        } else {
            (slot.value, slot.isnull)
        };
        assert!(!isnull, "vector ORDER BY parameter ${paramid} is NULL");

        let mut floats = PgVector::from_datum(value, false)
            .expect("vector ORDER BY parameter should not be NULL")
            .0;
        if metric.requires_unit_norm() {
            l2_normalize_in_place(&mut floats);
        }
        *query_vector = floats;
        *query_vector_param_id = None;
    }
}
