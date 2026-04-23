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
//! Provides a reference-counted wrapper around an open Postgres [`pg_sys::Relation`].
use crate::api::HashMap;
use crate::postgres::build::is_bm25_index;
use crate::postgres::catalog::is_pgvector_oid;
use crate::postgres::options::BM25IndexOptions;
use crate::schema::SearchIndexSchema;
use crate::vector::metric::VectorMetric;
use crate::vector::sampler::{NoopSamplerFactory, PgHeapVectorSamplerFactory, VectorFieldInfo};
use pgrx::{name_data_to_str, pg_sys, PgList, PgTupleDesc};
use std::cell::RefCell;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::ops::Deref;
use std::ptr::NonNull;
use std::rc::Rc;
use std::sync::Arc;
use tantivy::schema::{Field, FieldType};
use tantivy::vector::cluster::kmeans::KMeansConfig;
use tantivy::vector::cluster::plugin::{ClusterConfig, ClusterFieldConfig};
use tantivy::vector::cluster::sampler::VectorSamplerFactory;
use tantivy::vector::rotation::{DynamicRotator, RotatorType};
use tantivy::vector::turboquant::TurboQuantizer;
use tantivy::TantivyError;

type NeedClose = bool;

#[repr(transparent)]
struct IsCreateIndex(Rc<RefCell<bool>>);
impl Default for IsCreateIndex {
    fn default() -> Self {
        Self(Rc::new(RefCell::new(false)))
    }
}
impl IsCreateIndex {
    fn set(&self, value: bool) {
        self.0.replace(value);
    }

    fn get(&self) -> bool {
        *self.0.borrow()
    }
}

#[repr(transparent)]
struct ForkNumber(Rc<RefCell<pg_sys::ForkNumber::Type>>);
impl Default for ForkNumber {
    fn default() -> Self {
        Self(Rc::new(RefCell::new(pg_sys::ForkNumber::MAIN_FORKNUM)))
    }
}
impl ForkNumber {
    fn set(&self, value: pg_sys::ForkNumber::Type) {
        self.0.replace(value);
    }

    fn get(&self) -> pg_sys::ForkNumber::Type {
        *self.0.borrow()
    }
}

#[derive(Debug, Clone)]
pub enum SchemaError {
    RelationNotBM25Index,
    Other(TantivyError),
}

impl Display for SchemaError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RelationNotBM25Index => write!(f, "relation is not a BM25 index"),
            Self::Other(e) => write!(f, "{e}"),
        }
    }
}

impl Error for SchemaError {}

/// Represents an opened Postgres relation to be used by pg_search.
///
/// [`PgSearchRelation`] is reference counted and will close the underlying
/// [`pg_sys::Relation`] when the last reference is dropped, accounting for
/// the state of the current transaction.
///
/// Instances of [`PgSearchRelation`] can be closed as necessary.
#[allow(clippy::type_complexity)]
#[derive(Clone)]
#[repr(transparent)]
pub struct PgSearchRelation(
    Option<
        Rc<(
            NonNull<pg_sys::RelationData>,
            NeedClose,
            Option<pg_sys::LOCKMODE>,
            RefCell<Option<Result<SearchIndexSchema, SchemaError>>>,
            BM25IndexOptions,
            IsCreateIndex,
            ForkNumber,
        )>,
    >,
);

impl Debug for PgSearchRelation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PgSearchRelation")
            .field("relation", &self.oid())
            .field("lockmode", &self.lockmode())
            .finish()
    }
}

crate::impl_safe_drop!(PgSearchRelation, |self| {
    let Some(rc) = self.0.take() else {
        return;
    };
    let Some((relation, need_close, lockmode, ..)) = Rc::into_inner(rc) else {
        return;
    };
    unsafe {
        if need_close && pg_sys::IsTransactionState() {
            match lockmode {
                Some(lockmode) => pg_sys::relation_close(relation.as_ptr(), lockmode),
                None => pg_sys::RelationClose(relation.as_ptr()),
            }
        }
    }
});

impl Deref for PgSearchRelation {
    type Target = pg_sys::RelationData;

    fn deref(&self) -> &Self::Target {
        // SAFETY: the backing pointer is always correct for use by Rust as we couldn't have
        // gotten here otherwise
        unsafe { self.as_ptr().as_ref().unwrap_unchecked() }
    }
}

impl PgSearchRelation {
    /// Take ownership of a [`pg_sys::Relation`] pointer previously created by Postgres
    ///
    /// This relation will not be closed when we're dropped.
    pub unsafe fn from_pg(relation: pg_sys::Relation) -> Self {
        Self(Some(Rc::new((
            NonNull::new(relation)
                .expect("PgSearchRelation::from_pg: provided relation cannot be NULL"),
            false,
            None,
            Default::default(),
            BM25IndexOptions::from_relation(relation),
            IsCreateIndex::default(),
            ForkNumber::default(),
        ))))
    }

    /// Open a relation with the specified [`pg_sys::Oid`].
    ///
    /// This relation will be closed when we're the last of our reference-counted clones to be dropped.
    pub fn open(oid: pg_sys::Oid) -> Self {
        unsafe {
            // SAFETY: RelationIdGetRelation() should return a valid RelationData pointer
            // unless no pg_class row could be found (suggesting the relation was just dropped)
            let relation = pg_sys::RelationIdGetRelation(oid);
            if relation.is_null() {
                panic!("relation not found, suggesting it was just dropped");
            }

            Self(Some(Rc::new((
                NonNull::new_unchecked(relation),
                true,
                None,
                Default::default(),
                BM25IndexOptions::from_relation(relation),
                IsCreateIndex::default(),
                ForkNumber::default(),
            ))))
        }
    }

    /// Open a relation with the specified [`pg_sys::Oid`]
    ///
    /// Like [`Self::with_lock`], but fallible
    pub fn try_open(oid: pg_sys::Oid, lockmode: pg_sys::LOCKMODE) -> Option<Self> {
        // SAFETY: See `open`
        unsafe {
            let relation = pg_sys::try_relation_open(oid, lockmode);
            if relation.is_null() {
                None
            } else {
                Some(Self(Some(Rc::new((
                    NonNull::new_unchecked(relation),
                    true,
                    None,
                    Default::default(),
                    BM25IndexOptions::from_relation(relation),
                    IsCreateIndex::default(),
                    ForkNumber::default(),
                )))))
            }
        }
    }

    /// Open a relation with the specified [`pg_sys::Oid`] under the specified [`pg_sys::LOCKMODE`].
    ///
    /// This relation will be closed when we're the last of our reference-counted clones to be dropped.
    pub fn with_lock(oid: pg_sys::Oid, lockmode: pg_sys::LOCKMODE) -> Self {
        unsafe {
            // SAFETY: relation_open() always returns a valid RelationData pointer
            let relation = pg_sys::relation_open(oid, lockmode);
            Self(Some(Rc::new((
                NonNull::new_unchecked(relation),
                true,
                Some(lockmode),
                Default::default(),
                BM25IndexOptions::from_relation(relation),
                IsCreateIndex::default(),
                ForkNumber::default(),
            ))))
        }
    }

    pub fn set_is_create_index(&mut self) {
        self.0.as_ref().unwrap().5.set(true);
    }

    pub fn is_create_index(&self) -> bool {
        self.0.as_ref().unwrap().5.get()
    }

    pub fn set_fork_number(&mut self, fork_number: pg_sys::ForkNumber::Type) {
        self.0.as_ref().unwrap().6.set(fork_number);
    }

    pub fn fork_number(&self) -> pg_sys::ForkNumber::Type {
        self.0.as_ref().unwrap().6.get()
    }

    /// Returns false if in the middle of a `REINDEX CONCURRENTLY`
    /// and the index is not yet ready to serve queries
    pub fn is_valid(&self) -> bool {
        unsafe { (*(*self.as_ptr()).rd_index).indisvalid }
    }

    pub fn lockmode(&self) -> Option<pg_sys::LOCKMODE> {
        // SAFETY: self.0 is always Some
        unsafe { self.0.as_ref().unwrap_unchecked().2 }
    }

    pub fn oid(&self) -> pg_sys::Oid {
        // SAFETY: self.as_ptr() is always a valid pointer
        unsafe { (*self.as_ptr()).rd_id }
    }

    pub fn rel_oid(&self) -> Option<pg_sys::Oid> {
        if self.rd_index.is_null() {
            None
        } else {
            unsafe { Some((*self.rd_index).indrelid) }
        }
    }

    pub fn name(&self) -> &str {
        unsafe { name_data_to_str(&(*self.rd_rel).relname) }
    }

    pub fn namespace(&self) -> &str {
        unsafe {
            core::ffi::CStr::from_ptr(pg_sys::get_namespace_name((*self.rd_rel).relnamespace))
        }
        .to_str()
        .expect("unable to convert namespace name to UTF8")
    }

    pub fn tuple_desc(&self) -> PgTupleDesc<'_> {
        unsafe { PgTupleDesc::from_pg_unchecked(self.rd_att) }
    }

    pub fn reltuples(&self) -> Option<f32> {
        let reltuples = unsafe { (*self.rd_rel).reltuples };

        if reltuples == 0f32 {
            None
        } else {
            Some(reltuples)
        }
    }

    pub fn as_ptr(&self) -> pg_sys::Relation {
        // SAFETY: self.0 is always Some
        unsafe { self.0.as_ref().unwrap_unchecked().0.as_ptr() }
    }

    pub fn heap_relation(&self) -> Option<PgSearchRelation> {
        if self.rd_index.is_null() {
            None
        } else {
            unsafe { Some(PgSearchRelation::open((*self.rd_index).indrelid)) }
        }
    }

    pub fn indices(&self, lockmode: pg_sys::LOCKMODE) -> impl Iterator<Item = PgSearchRelation> {
        // SAFETY: we know self.as_ptr() is a valid pointer as we created it
        let list =
            unsafe { PgList::<pg_sys::Oid>::from_pg(pg_sys::RelationGetIndexList(self.as_ptr())) };

        list.iter_oid()
            .filter(|oid| *oid != pg_sys::InvalidOid)
            .map(|oid| PgSearchRelation::with_lock(oid, lockmode))
            .collect::<Vec<_>>()
            .into_iter()
    }

    pub fn options(&self) -> &BM25IndexOptions {
        unsafe {
            // SAFETY: self.0 is always Some
            &self.0.as_ref().unwrap_unchecked().4
        }
    }

    pub fn schema(&self) -> Result<SearchIndexSchema, SchemaError> {
        let rc = self.0.as_ref().unwrap();
        let mut borrow = rc.3.borrow_mut();
        let schema = borrow.get_or_insert_with(|| {
            if !is_bm25_index(self) {
                return Err(SchemaError::RelationNotBM25Index);
            }

            SearchIndexSchema::open(self).map_err(SchemaError::Other)
        });

        match schema {
            Ok(schema) => Ok(schema.clone()),
            Err(e) => Err(e.clone()),
        }
    }

    /// Get the index info for this relation.
    pub fn index_info(&self) -> *mut pg_sys::IndexInfo {
        unsafe { pg_sys::BuildIndexInfo(self.as_ptr()) }
    }

    /// Extract index expressions from the index info.
    pub fn index_expressions(&self) -> PgList<pg_sys::Expr> {
        unsafe { PgList::<pg_sys::Expr>::from_pg((*self.index_info()).ii_Expressions) }
    }

    /// Check if a field supports aggregate pushdown.
    ///
    /// Returns `Ok(false)` for NUMERIC fields, `Ok(true)` for other fields,
    /// or an error if the schema cannot be loaded.
    pub fn field_supports_aggregate(&self, field: &str) -> Result<bool, SchemaError> {
        self.schema().map(|s| s.field_supports_aggregate(field))
    }

    /// Whether this index has at least one vector field AND we're in a
    /// context where build-time clustering was deferred
    /// (`is_create_index()` is still set). Used by the parallel
    /// build worker to decide whether to do a per-worker cluster-rebuild
    /// pass after its final commit.
    pub fn needs_cluster_rebuild(&self) -> bool {
        if !self.is_create_index() {
            return false;
        }
        let Ok(schema) = self.schema() else {
            return false;
        };
        let tantivy_schema: tantivy::schema::Schema = schema.into();
        let has_vector = tantivy_schema
            .fields()
            .any(|(_, entry)| matches!(entry.field_type(), FieldType::Vector(_)));
        has_vector
    }

    /// Build a `ClusterConfig` for registering the cluster plugin on
    /// this index. Returns `None` if the index has no vector fields.
    ///
    /// `with_heap_sampler=true` wires up a `PgHeapVectorSampler` that
    /// re-reads full-precision vectors from the heap during k-means
    /// training (used on merge / build paths). `with_heap_sampler=false`
    /// uses a no-op sampler (read path; never re-trains).
    pub fn cluster_config(&self, with_heap_sampler: bool) -> Option<ClusterConfig> {
        let schema = self.schema().ok()?;
        let tantivy_schema: tantivy::schema::Schema = schema.into();

        let vector_fields: Vec<(Field, usize, VectorMetric)> = tantivy_schema
            .fields()
            .filter_map(|(field, entry)| match entry.field_type() {
                FieldType::Vector(opts) => Some((field, opts.dimensions, opts.metric.into())),
                _ => None,
            })
            .collect();
        if vector_fields.is_empty() {
            return None;
        }

        // Pick the sampler factory: heap-backed for merge / build (k-means
        // training re-reads full-precision vectors via ctid), no-op for
        // the read path. The heap-backed factory walks the heap relation
        // tuple desc once to map each schema vector field to its heap
        // attno; dims + metric already come from the schema.
        let sampler_factory: Arc<dyn VectorSamplerFactory> = with_heap_sampler
            .then(|| self.heap_relation())
            .flatten()
            .map(|heap_rel| {
                let tupdesc = heap_rel.tuple_desc();
                let attno_by_name: HashMap<&str, pg_sys::AttrNumber> = (0..tupdesc.len())
                    .filter_map(|i| {
                        let attr = tupdesc.get(i)?;
                        is_pgvector_oid(attr.type_oid().value())
                            .then(|| (attr.name(), (i + 1) as pg_sys::AttrNumber))
                    })
                    .collect();
                let field_info: HashMap<Field, VectorFieldInfo> = vector_fields
                    .iter()
                    .filter_map(|&(field, dims, metric)| {
                        let name = tantivy_schema.get_field_name(field);
                        let attno = *attno_by_name.get(name)?;
                        Some((
                            field,
                            VectorFieldInfo {
                                attno,
                                dims,
                                metric,
                            },
                        ))
                    })
                    .collect();
                if field_info.is_empty() {
                    Arc::new(NoopSamplerFactory) as Arc<dyn VectorSamplerFactory>
                } else {
                    Arc::new(PgHeapVectorSamplerFactory::new(heap_rel.oid(), field_info))
                }
            })
            .unwrap_or_else(|| Arc::new(NoopSamplerFactory));

        let field_configs: Vec<ClusterFieldConfig> = vector_fields
            .iter()
            .map(|&(field, dims, metric)| {
                let rotator = Arc::new(DynamicRotator::new(dims, RotatorType::FhtKacRotator, 42));
                let padded_dims = rotator.padded_dim();
                ClusterFieldConfig::new(
                    field,
                    dims,
                    padded_dims,
                    metric.runtime_metric(),
                    rotator,
                    42,
                    TurboQuantizer::new(dims, None, None),
                )
            })
            .collect();

        Some(ClusterConfig {
            fields: field_configs,
            clustering_threshold: 1000,
            num_clusters_fn: Arc::new(|n| (n as f64 / 250.0).ceil() as usize),
            // K-means iterations dominate the build. The cluster centroids
            // are only used for probe pruning at query time — recall comes
            // from the per-doc TurboQuant records, not from how perfectly
            // the centroids partition the space — so a small number of
            // iterations is fine. 5 iterations recovers most of the recall
            // benefit at ~1/5 the time. Sample is capped at 16k to bound
            // per-segment cost.
            kmeans: KMeansConfig {
                niter: 5,
                ..KMeansConfig::default()
            },
            sample_ratio: 0.1,
            sample_cap: 16_384,
            sampler_factory,
            defer_clustering: self.is_create_index(),
        })
    }
}
