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
use tantivy::schema::VectorMetric as TantivyMetric;
use tantivy::vector::Metric as RuntimeMetric;

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VectorMetric {
    #[default]
    L2,
    Cosine,
    InnerProduct,
}

impl VectorMetric {
    /// Whether the underlying TurboQuant codec (InnerProduct on the unit
    /// sphere) requires query/insert vectors to be L2-normalized first.
    /// L2 and Cosine both reduce to unit-sphere IP; InnerProduct keeps
    /// magnitude — caller asked for magnitude-aware IP.
    pub fn requires_unit_norm(&self) -> bool {
        matches!(self, VectorMetric::L2 | VectorMetric::Cosine)
    }

    /// Map the schema-level metric to the runtime metric the TurboQuant
    /// codec computes against. L2 and Cosine both collapse to
    /// `Metric::L2` because the codec is unit-sphere only and we
    /// pre-normalize at the boundary on the insert + query paths
    /// (see `postgres::utils::row_to_search_document` and
    /// `extract_vector_distance`). `InnerProduct` is preserved for
    /// callers who want magnitude-aware IP.
    pub fn runtime_metric(self) -> RuntimeMetric {
        match self {
            VectorMetric::L2 | VectorMetric::Cosine => RuntimeMetric::L2,
            VectorMetric::InnerProduct => RuntimeMetric::InnerProduct,
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
            TantivyMetric::InnerProduct => Self::InnerProduct,
        }
    }
}

impl From<VectorMetric> for TantivyMetric {
    fn from(m: VectorMetric) -> Self {
        match m {
            VectorMetric::L2 => TantivyMetric::L2,
            VectorMetric::Cosine => TantivyMetric::Cosine,
            VectorMetric::InnerProduct => TantivyMetric::InnerProduct,
        }
    }
}

/// L2-normalize a vector in-place (divide by its L2 norm). For zero
/// vectors the input is left unchanged. Callers use this to pre-normalize
/// inputs to a cosine-metric vector field so the underlying TurboQuant
/// index (which ranks by InnerProduct on the unit sphere) produces
/// cosine-equivalent ordering.
pub fn l2_normalize_in_place(v: &mut [f32]) {
    let mut norm_sq = 0.0f32;
    for &x in v.iter() {
        norm_sq += x * x;
    }
    if norm_sq <= 0.0 {
        return;
    }
    let inv = 1.0 / norm_sq.sqrt();
    for x in v.iter_mut() {
        *x *= inv;
    }
}
