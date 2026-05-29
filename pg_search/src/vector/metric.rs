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

//! Distance metric for indexed vector fields.
//!
//! The metric travels via the index attribute's operator class — pgvector
//! convention. CREATE INDEX uses one of:
//!
//!   embedding vector_l2_ops      -- <-> L2
//!   embedding vector_cosine_ops  -- <=> Cosine
//!   embedding vector_ip_ops      -- <#> InnerProduct
//!
//! At build time we read `pg_index.indclass[i]` for each vector attribute
//! and map the opclass name to a `VectorMetric` via
//! `metric_from_opclass_name`. The metric is then persisted in the
//! tantivy schema's `VectorOptions::metric`.

use crate::postgres::catalog::lookup_opfamily_name;
use pgrx::pg_sys;
use serde::{Deserialize, Serialize};
use tantivy::vector::Metric as TantivyMetric;

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VectorMetric {
    #[default]
    L2,
    Cosine,
    InnerProduct,
}

impl VectorMetric {
    /// Map the pgvector-facing metric to Tantivy's vector metric.
    pub fn runtime_metric(self) -> TantivyMetric {
        match self {
            VectorMetric::L2 => TantivyMetric::L2,
            VectorMetric::Cosine => TantivyMetric::Cosine,
            VectorMetric::InnerProduct => TantivyMetric::Dot,
        }
    }

    /// pgvector opclass name (`vector_l2_ops`, `vector_cosine_ops`,
    /// `vector_ip_ops`). Inverse of [`Self::from_opfamily_name`].
    pub fn opclass_name(self) -> &'static str {
        match self {
            VectorMetric::L2 => "vector_l2_ops",
            VectorMetric::Cosine => "vector_cosine_ops",
            VectorMetric::InnerProduct => "vector_ip_ops",
        }
    }

    /// pgvector distance operator symbol (`<->`, `<=>`, `<#>`).
    pub fn operator(self) -> &'static str {
        match self {
            VectorMetric::L2 => "<->",
            VectorMetric::Cosine => "<=>",
            VectorMetric::InnerProduct => "<#>",
        }
    }

    /// Map a pg_opfamily name (e.g. `vector_cosine_ops`) to its implied
    /// metric. Returns `None` for any name we don't recognise — callers
    /// then fall back to `VectorMetric::default()` (L2).
    ///
    /// Opfamily and opclass names match for the three vector opclasses
    /// we declare (CREATE OPERATOR CLASS without an explicit FAMILY
    /// clause makes Postgres invent a same-named family).
    fn from_opfamily_name(name: &str) -> Option<Self> {
        match name {
            "vector_l2_ops" => Some(Self::L2),
            "vector_cosine_ops" => Some(Self::Cosine),
            "vector_ip_ops" => Some(Self::InnerProduct),
            _ => None,
        }
    }

    /// Resolve the metric for a vector attribute by reading the
    /// opfamily OID cached in the index relation's `rd_opfamily[]`
    /// array and mapping the opfamily name.
    ///
    /// `attno` is the **0-based** index attribute position (matches
    /// `IndexInfo.ii_IndexAttrNumbers[attno]`). Returns `None` if the
    /// opfamily isn't one of our three vector opfamilies (e.g. the
    /// user fell back to the default `anyelement_bm25_ops`).
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
