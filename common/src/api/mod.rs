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

#![allow(for_loops_over_fallibles)]
#![allow(clippy::missing_safety_doc, unsafe_op_in_unsafe_fn)]

pub mod aggregate;
pub mod operator;
pub mod tokenizers;
pub mod version;
pub mod window_aggregate;

use pgrx::{
    FromDatum, InOutFuncs, IntoDatum, PostgresType, StringInfo, direct_function_call, pg_sys,
};

pub use aggregate::{
    MvccVisibility, agg_fn_oid, agg_funcoid, agg_funcoids, combine_agg_funcoid, is_agg_funcoid,
    is_combine_agg_funcoid, pdb_agg_spec, visibility_from_agg_arg,
};
pub use rustc_hash::FxHashMap as HashMap;
pub use rustc_hash::FxHashSet as HashSet;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::ffi::CStr;
use std::fmt::{Debug, Display, Formatter};
use std::ops::Deref;
use tantivy::json_utils::split_json_path;

use crate::vector::PgVector;
use crate::vector::metric::VectorMetric;

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

/// Casts `$node` to a `*mut pg_sys::$type_` when it is non-NULL and a node of that type.
///
/// `OpExpr`, `DistinctExpr` and `NullIfExpr` are one type in Rust with a node tag each. The name
/// given selects the tag, so `nodecast!(DistinctExpr, node)` matches only a `T_DistinctExpr`.
#[macro_export]
macro_rules! nodecast {
    ($type_:ident, $node:expr) => {{
        let node = $node;
        (!node.is_null() && pgrx::IsA!(node, pgrx::pg_sys::$type_))
            .then(|| node.cast::<pgrx::pg_sys::$type_>())
    }};
}

// came to life in pg15
pub type Cardinality = f64;

pub type Varno = i32;

pub const CTID_FIELD_NAME: &str = "ctid";

pub trait AsCStr {
    unsafe fn as_c_str(&self) -> Option<&std::ffi::CStr>;
}

impl AsCStr for *mut pgrx::pg_sys::Node {
    unsafe fn as_c_str(&self) -> Option<&std::ffi::CStr> {
        let node = nodecast!(String, *self)?;
        Some(std::ffi::CStr::from_ptr((*node).sval))
    }
}

/// A type used whenever our builder functions require a fieldname.
#[derive(
    Debug, Clone, Ord, Eq, PartialOrd, PartialEq, Hash, Serialize, Deserialize, PostgresType,
)]
#[inoutfuncs]
pub struct FieldName(pub String);

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
        self.root() == CTID_FIELD_NAME
    }
}

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

impl From<arrow_schema::SortOptions> for SortDirection {
    fn from(options: arrow_schema::SortOptions) -> Self {
        match (options.descending, options.nulls_first) {
            (true, true) => SortDirection::DescNullsFirst,
            (true, false) => SortDirection::DescNullsLast,
            (false, true) => SortDirection::AscNullsFirst,
            (false, false) => SortDirection::AscNullsLast,
        }
    }
}

impl SortDirection {
    #[cfg(any(feature = "pg15", feature = "pg16", feature = "pg17"))]
    pub unsafe fn from_sort_op(sortop: pg_sys::Oid, nulls_first: bool) -> Option<Self> {
        let mut opfamily = pg_sys::InvalidOid;
        let mut opcintype = pg_sys::InvalidOid;
        let mut strategy: i16 = 0;
        if pg_sys::get_ordering_op_properties(sortop, &mut opfamily, &mut opcintype, &mut strategy)
        {
            let descending = strategy as u32 == pg_sys::BTGreaterStrategyNumber;
            Some(Self::from(arrow_schema::SortOptions {
                descending,
                nulls_first,
            }))
        } else {
            None
        }
    }

    #[cfg(feature = "pg18")]
    pub unsafe fn from_sort_op(sortop: pg_sys::Oid, nulls_first: bool) -> Option<Self> {
        let mut opfamily = pg_sys::InvalidOid;
        let mut opcintype = pg_sys::InvalidOid;
        let mut cmptype = pg_sys::CompareType::COMPARE_LT;
        if pg_sys::get_ordering_op_properties(sortop, &mut opfamily, &mut opcintype, &mut cmptype) {
            let descending = cmptype == pg_sys::CompareType::COMPARE_GT;
            Some(Self::from(arrow_schema::SortOptions {
                descending,
                nulls_first,
            }))
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

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum QueryVector {
    Resolved(#[serde(with = "query_vector_bytes")] Vec<f32>),
    Param(i32),
    Expr(String),
}

/// A resolved query vector in serialized private data: one hex string of its little-endian
/// `f32` bytes, written and parsed as a single token rather than one JSON number per element.
mod query_vector_bytes {
    use serde::de::Error;
    use serde::{Deserialize, Deserializer, Serializer};

    const HEX: &[u8; 16] = b"0123456789abcdef";

    pub fn serialize<S: Serializer>(floats: &[f32], serializer: S) -> Result<S::Ok, S::Error> {
        let mut text = String::with_capacity(floats.len() * 8);
        for byte in floats.iter().flat_map(|value| value.to_le_bytes()) {
            text.push(char::from(HEX[usize::from(byte >> 4)]));
            text.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
        serializer.serialize_str(&text)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<f32>, D::Error> {
        let text = String::deserialize(deserializer)?;
        if text.len() % 8 != 0 {
            return Err(D::Error::custom(
                "query vector bytes are not whole f32 values",
            ));
        }
        let nibble = |digit: u8| match digit {
            b'0'..=b'9' => Ok(digit - b'0'),
            b'a'..=b'f' => Ok(digit - b'a' + 10),
            _ => Err(D::Error::custom("query vector bytes are not lowercase hex")),
        };
        text.as_bytes()
            .chunks_exact(8)
            .map(|digits| {
                let mut bytes = [0u8; 4];
                for (byte, pair) in bytes.iter_mut().zip(digits.chunks_exact(2)) {
                    *byte = (nibble(pair[0])? << 4) | nibble(pair[1])?;
                }
                Ok(f32::from_le_bytes(bytes))
            })
            .collect()
    }
}

impl QueryVector {
    pub fn resolved(&self) -> Option<&[f32]> {
        match self {
            QueryVector::Resolved(floats) => Some(floats.as_slice()),
            QueryVector::Param(_) | QueryVector::Expr(_) => None,
        }
    }

    pub unsafe fn resolve(
        &mut self,
        estate: *mut pgrx::pg_sys::EState,
        planstate: *mut pgrx::pg_sys::PlanState,
    ) {
        let paramid = match self {
            QueryVector::Resolved(_) => return,
            QueryVector::Expr(expr_str) => {
                use crate::postgres::customscan::expr_eval::PreparedPgExpr;
                let prepared = PreparedPgExpr::from_serialized(expr_str, &[]);
                let expr_state = pg_sys::ExecInitExpr(prepared.as_ptr(), planstate);
                let econtext = (*planstate).ps_ExprContext;
                assert!(
                    !econtext.is_null(),
                    "planstate has no ExprContext to evaluate vector ORDER BY operand"
                );
                let mut isnull = false;
                let datum = pg_sys::ExecEvalExpr(expr_state, econtext, &mut isnull);
                assert!(!isnull, "vector ORDER BY operand evaluated to NULL");
                let floats = PgVector::from_datum(datum, false)
                    .expect("vector ORDER BY operand should not be NULL")
                    .0;
                *self = QueryVector::Resolved(floats);
                return;
            }
            QueryVector::Param(paramid) => *paramid,
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

        let floats = PgVector::from_datum(value, false)
            .expect("vector ORDER BY parameter should not be NULL")
            .0;
        *self = QueryVector::Resolved(floats);
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ScoreOrderExpr {
    Score { rti: pg_sys::Index },
    Constant(f64),
    Add(Box<ScoreOrderExpr>, Box<ScoreOrderExpr>),
    Mul(Box<ScoreOrderExpr>, Box<ScoreOrderExpr>),
}

impl ScoreOrderExpr {
    pub fn format_with<F: Fn(&pg_sys::Index) -> String>(&self, score_name: &F) -> String {
        match self {
            Self::Score { rti } => score_name(rti),
            Self::Constant(c) => format!("{c}"),
            Self::Add(l, r) => format!(
                "{} + {}",
                l.format_with(score_name),
                r.format_with(score_name)
            ),
            Self::Mul(l, r) => {
                let fmt_child = |child: &ScoreOrderExpr| match child {
                    Self::Add(_, _) => format!("({})", child.format_with(score_name)),
                    _ => child.format_with(score_name),
                };
                format!("{} * {}", fmt_child(l), fmt_child(r))
            }
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum OrderByFeature {
    Score {
        rti: u32,
    },
    ScoreSum {
        expr: ScoreOrderExpr,
        rtis: Vec<pg_sys::Index>,
    },
    Field {
        name: FieldName,
        rti: u32,
    },
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
        query_vector: QueryVector,
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
            Self::ScoreSum { .. } => write!(f, "sum(pdb.score())"),
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

    pub fn as_vector_distance(&self) -> Option<(&FieldName, &[f32], VectorMetric)> {
        match &self.feature {
            OrderByFeature::VectorDistance {
                name,
                query_vector,
                metric,
                ..
            } => {
                let floats = query_vector
                    .resolved()
                    .expect("vector ORDER BY query vector was never resolved");
                Some((name, floats, *metric))
            }
            _ => None,
        }
    }

    pub unsafe fn resolve_query_vector(
        &mut self,
        estate: *mut pgrx::pg_sys::EState,
        planstate: *mut pgrx::pg_sys::PlanState,
    ) {
        if let OrderByFeature::VectorDistance { query_vector, .. } = &mut self.feature {
            query_vector.resolve(estate, planstate);
        }
    }
}

#[cfg(test)]
mod query_vector_tests {
    use super::QueryVector;

    /// A resolved vector serializes as one hex string and parses back bit for bit, signed zeros,
    /// infinities, subnormals and NaN payloads included; malformed text is rejected.
    #[test]
    fn resolved_vector_round_trips_as_one_hex_string() {
        let floats = vec![
            0.0,
            -0.0,
            1.5,
            -2.25e-3,
            f32::MIN_POSITIVE / 4.0,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::from_bits(0x7fc0_1234),
        ];
        assert_eq!(
            serde_json::to_string(&QueryVector::Resolved(vec![1.0, -2.0])).unwrap(),
            r#"{"Resolved":"0000803f000000c0"}"#
        );
        let json = serde_json::to_string(&QueryVector::Resolved(floats.clone())).unwrap();
        assert_eq!(
            json.len(),
            r#"{"Resolved":""}"#.len() + floats.len() * 8,
            "{json}"
        );
        let QueryVector::Resolved(parsed) = serde_json::from_str(&json).unwrap() else {
            panic!("{json}");
        };
        assert_eq!(
            parsed
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            floats
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>()
        );
        for bad in [r#"{"Resolved":"0000803"}"#, r#"{"Resolved":"0000803G"}"#] {
            assert!(serde_json::from_str::<QueryVector>(bad).is_err(), "{bad}");
        }
    }
}
