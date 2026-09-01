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

//! Vector metrics and their PostgreSQL operators and operator classes.

use crate::postgres::catalog::lookup_opfamily_name;
use pgrx::{IntoDatum, direct_function_call, pg_sys};
use serde::{Deserialize, Serialize};
use tantivy::vector::Metric as TantivyMetric;

/// A pgvector-compatible distance metric.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VectorMetric {
    /// Euclidean distance.
    #[default]
    L2,
    /// Cosine distance.
    Cosine,
    /// Inner-product distance.
    InnerProduct,
}

impl VectorMetric {
    /// Returns the corresponding Tantivy metric.
    pub fn runtime_metric(self) -> TantivyMetric {
        match self {
            VectorMetric::L2 => TantivyMetric::L2,
            VectorMetric::Cosine => TantivyMetric::Cosine,
            VectorMetric::InnerProduct => TantivyMetric::Dot,
        }
    }

    /// Returns the pgvector operator-class name.
    pub fn opclass_name(self) -> &'static str {
        match self {
            VectorMetric::L2 => "vector_l2_ops",
            VectorMetric::Cosine => "vector_cosine_ops",
            VectorMetric::InnerProduct => "vector_ip_ops",
        }
    }

    /// Returns the pgvector distance operator.
    pub fn operator(self) -> &'static str {
        match self {
            VectorMetric::L2 => "<->",
            VectorMetric::Cosine => "<=>",
            VectorMetric::InnerProduct => "<#>",
        }
    }

    /// Resolves a pgvector distance operator OID.
    pub(crate) fn from_opoid(opoid: pg_sys::Oid) -> Option<Self> {
        use std::sync::OnceLock;
        static OP_METRICS: OnceLock<[(pg_sys::Oid, VectorMetric); 3]> = OnceLock::new();
        let cached = OP_METRICS.get_or_init(|| unsafe {
            let vector_type_exists =
                direct_function_call::<pg_sys::Oid>(pg_sys::to_regtype, &["vector".into_datum()])
                    .is_some();
            let lookup = |sig: &std::ffi::CStr| -> pg_sys::Oid {
                if !vector_type_exists {
                    return pg_sys::Oid::INVALID;
                }
                direct_function_call::<pg_sys::Oid>(pg_sys::regoperatorin, &[sig.into_datum()])
                    .unwrap_or(pg_sys::Oid::INVALID)
            };
            [
                (lookup(c"<->(vector,vector)"), VectorMetric::L2),
                (lookup(c"<=>(vector,vector)"), VectorMetric::Cosine),
                (lookup(c"<#>(vector,vector)"), VectorMetric::InnerProduct),
            ]
        });
        cached
            .iter()
            .find(|(oid, _)| *oid != pg_sys::Oid::INVALID && *oid == opoid)
            .map(|(_, metric)| *metric)
    }

    fn from_opfamily_name(name: &str) -> Option<Self> {
        match name {
            "vector_l2_ops" => Some(Self::L2),
            "vector_cosine_ops" => Some(Self::Cosine),
            "vector_ip_ops" => Some(Self::InnerProduct),
            _ => None,
        }
    }

    /// Resolves the metric for an index attribute.
    ///
    /// # Arguments
    ///
    /// * `attno` - Zero-based index attribute position.
    ///
    /// # Safety
    ///
    /// `indexrel` must be null or point to a valid open PostgreSQL index relation.
    pub unsafe fn from_index_attr(indexrel: pg_sys::Relation, attno: usize) -> Option<Self> {
        if indexrel.is_null() {
            return None;
        }
        let n = (*indexrel)
            .rd_att
            .as_ref()
            .map(|t| t.natts as usize)
            .unwrap_or(0);
        if attno >= n {
            return None;
        }
        let rd_opfamily = (*indexrel).rd_opfamily;
        if rd_opfamily.is_null() {
            return None;
        }
        let opfamily_oid = rd_opfamily.add(attno).read();
        let name = lookup_opfamily_name(opfamily_oid)?;
        Self::from_opfamily_name(&name)
    }
}

impl From<TantivyMetric> for VectorMetric {
    fn from(m: TantivyMetric) -> Self {
        match m {
            TantivyMetric::L2 => Self::L2,
            TantivyMetric::Cosine => Self::Cosine,
            TantivyMetric::Dot => Self::InnerProduct,
        }
    }
}

impl From<VectorMetric> for TantivyMetric {
    fn from(m: VectorMetric) -> Self {
        match m {
            VectorMetric::L2 => TantivyMetric::L2,
            VectorMetric::Cosine => TantivyMetric::Cosine,
            VectorMetric::InnerProduct => TantivyMetric::Dot,
        }
    }
}
