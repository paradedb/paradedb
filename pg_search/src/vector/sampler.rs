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

//! `VectorSampler` implementations used by the cluster plugin at merge
//! time. The heap sampler re-reads full-precision vectors from Postgres
//! via `ctid`, so k-means training can operate on the original vectors
//! regardless of what the encoded segment stored. The no-op sampler is
//! used on the read path (and when the index has no heap vector
//! columns) to satisfy the trait without doing any work.

use pgrx::{pg_sys, FromDatum};
use tantivy::index::SegmentReader;
use tantivy::indexer::doc_id_mapping::SegmentDocIdMapping;
use tantivy::schema::Field;
use tantivy::vector::cluster::sampler::{VectorSampler, VectorSamplerFactory};
use tantivy::{DocAddress, DocId};

use crate::api::HashMap;
use crate::postgres::rel::PgSearchRelation;
use crate::vector::metric::{l2_normalize_in_place, VectorMetric};
use crate::vector::PgVector;

pub(crate) struct VectorFieldInfo {
    pub attno: pg_sys::AttrNumber,
    pub dims: usize,
    /// Distance metric the BM25 index was built for. The sampler uses
    /// this to L2-normalize sampled vectors when `Cosine` so k-means
    /// clusters in the unit sphere — matching the normalized doc
    /// vectors that get TurboQuant-encoded.
    pub metric: VectorMetric,
}

pub(crate) struct PgHeapVectorSamplerFactory {
    heap_oid: pg_sys::Oid,
    field_info: HashMap<Field, VectorFieldInfo>,
}

impl PgHeapVectorSamplerFactory {
    pub(crate) fn new(heap_oid: pg_sys::Oid, field_info: HashMap<Field, VectorFieldInfo>) -> Self {
        Self {
            heap_oid,
            field_info,
        }
    }
}

impl VectorSamplerFactory for PgHeapVectorSamplerFactory {
    fn create_sampler(
        &self,
        readers: &[SegmentReader],
        doc_id_mapping: &SegmentDocIdMapping,
    ) -> tantivy::Result<Box<dyn VectorSampler>> {
        let mapping: Vec<DocAddress> = doc_id_mapping.iter_old_doc_addrs().collect();

        let mut ctid_columns = Vec::with_capacity(readers.len());
        for reader in readers {
            let col = reader.fast_fields().u64("ctid").map_err(|e| {
                tantivy::TantivyError::InternalError(format!("ctid fast field: {e}"))
            })?;
            ctid_columns.push(col);
        }

        let field_attno: HashMap<Field, pg_sys::AttrNumber> = self
            .field_info
            .iter()
            .map(|(&f, info)| (f, info.attno))
            .collect();
        let field_dims: HashMap<Field, usize> = self
            .field_info
            .iter()
            .map(|(&f, info)| (f, info.dims))
            .collect();
        let field_metric: HashMap<Field, VectorMetric> = self
            .field_info
            .iter()
            .map(|(&f, info)| (f, info.metric))
            .collect();

        Ok(Box::new(PgHeapVectorSampler {
            mapping,
            ctid_columns,
            heap_oid: self.heap_oid,
            field_attno,
            field_dims,
            field_metric,
        }))
    }
}

struct PgHeapVectorSampler {
    mapping: Vec<DocAddress>,
    ctid_columns: Vec<tantivy::columnar::Column<u64>>,
    heap_oid: pg_sys::Oid,
    field_attno: HashMap<Field, pg_sys::AttrNumber>,
    field_dims: HashMap<Field, usize>,
    field_metric: HashMap<Field, VectorMetric>,
}

unsafe impl Send for PgHeapVectorSampler {}
unsafe impl Sync for PgHeapVectorSampler {}

impl VectorSampler for PgHeapVectorSampler {
    fn sample_vectors(
        &self,
        field: Field,
        doc_ids: &[DocId],
    ) -> tantivy::Result<Vec<Option<Vec<f32>>>> {
        let attno = *self.field_attno.get(&field).ok_or_else(|| {
            tantivy::TantivyError::InternalError(format!(
                "no heap attno mapping for vector field {field:?}"
            ))
        })?;

        let mut results = Vec::with_capacity(doc_ids.len());

        unsafe {
            let heaprel = PgSearchRelation::open(self.heap_oid);
            let snapshot = pg_sys::GetActiveSnapshot();
            let slot = pg_sys::MakeSingleTupleTableSlot(
                heaprel.tuple_desc().as_ptr(),
                &pg_sys::TTSOpsBufferHeapTuple,
            );

            for &new_doc_id in doc_ids {
                let addr = self.mapping[new_doc_id as usize];
                let ctid_col = &self.ctid_columns[addr.segment_ord as usize];
                let ctid_val = ctid_col.first(addr.doc_id);

                let vec = if let Some(ctid) = ctid_val {
                    let mut tid = pg_sys::ItemPointerData::default();
                    crate::postgres::utils::u64_to_item_pointer(ctid, &mut tid);

                    let found = pg_sys::table_tuple_fetch_row_version(
                        heaprel.as_ptr(),
                        &mut tid,
                        snapshot,
                        slot,
                    );

                    if found {
                        let mut is_null = false;
                        let datum = pg_sys::slot_getattr(slot, attno as i32, &mut is_null);
                        if let Some(PgVector(mut floats)) = PgVector::from_datum(datum, is_null) {
                            // Mirror the insert-path normalization
                            // (see postgres::utils::row_to_search_document):
                            // L2 + Cosine both use the unit-sphere
                            // codec, so heap samples used for k-means
                            // training must also be unit-norm.
                            if self
                                .field_metric
                                .get(&field)
                                .copied()
                                .is_some_and(|m| m.requires_unit_norm())
                            {
                                l2_normalize_in_place(&mut floats);
                            }
                            Some(floats)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };
                results.push(vec);
            }

            pg_sys::ExecDropSingleTupleTableSlot(slot);
        }

        Ok(results)
    }

    fn dims(&self, field: Field) -> usize {
        self.field_dims.get(&field).copied().unwrap_or(0)
    }
}

pub(crate) struct NoopSamplerFactory;

impl VectorSamplerFactory for NoopSamplerFactory {
    fn create_sampler(
        &self,
        _readers: &[SegmentReader],
        _doc_id_mapping: &SegmentDocIdMapping,
    ) -> tantivy::Result<Box<dyn VectorSampler>> {
        Ok(Box::new(NoopSampler))
    }
}

struct NoopSampler;

impl VectorSampler for NoopSampler {
    fn sample_vectors(
        &self,
        _field: Field,
        doc_ids: &[DocId],
    ) -> tantivy::Result<Vec<Option<Vec<f32>>>> {
        Ok(vec![None; doc_ids.len()])
    }

    fn dims(&self, _field: Field) -> usize {
        0
    }
}
