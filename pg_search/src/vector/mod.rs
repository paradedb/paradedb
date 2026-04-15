use std::sync::Arc;

use pgrx::pg_sys;
use tantivy::index::SegmentReader;
use tantivy::indexer::doc_id_mapping::SegmentDocIdMapping;
use tantivy::schema::{Field, FieldType};
use tantivy::vector::bqvec::BqVecPlugin;
use tantivy::vector::cluster::kmeans::KMeansConfig;
use tantivy::vector::cluster::plugin::{ClusterConfig, ClusterFieldConfig, ClusterPlugin};
use tantivy::vector::cluster::sampler::{VectorSampler, VectorSamplerFactory};
use tantivy::vector::rabitq::rotation::{DynamicRotator, RotatorType};
use tantivy::vector::rabitq::Metric;
use tantivy::{DocAddress, DocId};

use crate::postgres::rel::PgSearchRelation;

use std::collections::HashMap;

pub struct VectorFieldInfo {
    pub attno: pg_sys::AttrNumber,
    pub dims: usize,
}

pub struct PgHeapVectorSamplerFactory {
    heap_oid: pg_sys::Oid,
    field_info: HashMap<Field, VectorFieldInfo>,
}

impl PgHeapVectorSamplerFactory {
    pub fn new(heap_oid: pg_sys::Oid, field_info: HashMap<Field, VectorFieldInfo>) -> Self {
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

        Ok(Box::new(PgHeapVectorSampler {
            mapping,
            ctid_columns,
            heap_oid: self.heap_oid,
            field_attno,
            field_dims,
        }))
    }
}

struct PgHeapVectorSampler {
    mapping: Vec<DocAddress>,
    ctid_columns: Vec<tantivy::columnar::Column<u64>>,
    heap_oid: pg_sys::Oid,
    field_attno: HashMap<Field, pg_sys::AttrNumber>,
    field_dims: HashMap<Field, usize>,
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
                        if !is_null {
                            let floats =
                                crate::postgres::types::extract_pgvector_floats_from_datum(datum);
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

/// Detect all vector columns' heap attribute info from an index relation.
/// Returns (heap_oid, map of field_name → VectorFieldInfo) if vector columns are found.
pub unsafe fn find_vector_heap_info(
    indexrel: &PgSearchRelation,
) -> Option<(pg_sys::Oid, Vec<(String, VectorFieldInfo)>)> {
    let heap_rel = indexrel.heap_relation()?;
    let heap_oid = heap_rel.oid();
    let tupdesc = heap_rel.tuple_desc();

    let mut fields = Vec::new();
    for i in 0..tupdesc.len() {
        let attr = tupdesc.get(i)?;
        let type_oid = attr.type_oid().value();
        if crate::postgres::catalog::is_pgvector_oid(type_oid) {
            let attno = (i + 1) as pg_sys::AttrNumber;
            let typmod = attr.atttypmod;
            let dims = if typmod > 0 { typmod as usize } else { 0 };
            let name = attr.name().to_string();
            fields.push((name, VectorFieldInfo { attno, dims }));
        }
    }

    if fields.is_empty() {
        None
    } else {
        Some((heap_oid, fields))
    }
}

pub fn register_vector_plugins_for_merge(index: &mut tantivy::Index, indexrel: &PgSearchRelation) {
    let schema = index.schema();
    let has_vector = schema
        .fields()
        .any(|(_, entry)| matches!(entry.field_type(), FieldType::Vector(_)));

    if !has_vector {
        return;
    }

    if let Some((heap_oid, heap_fields)) = unsafe { find_vector_heap_info(indexrel) } {
        let mut field_info = HashMap::new();
        for (name, info) in heap_fields {
            if let Ok(field) = schema.get_field(&name) {
                field_info.insert(field, info);
            }
        }
        if !field_info.is_empty() {
            register_vector_plugins_with_sampler(index, heap_oid, field_info);
            return;
        }
    }

    register_vector_plugins(index);
}

pub fn register_vector_plugins(index: &mut tantivy::Index) {
    let schema = index.schema();
    let vector_fields: Vec<(Field, usize)> = schema
        .fields()
        .filter_map(|(field, entry)| {
            if let FieldType::Vector(opts) = entry.field_type() {
                Some((field, opts.dimensions))
            } else {
                None
            }
        })
        .collect();

    if vector_fields.is_empty() {
        return;
    }

    let field_configs: Vec<ClusterFieldConfig> = vector_fields
        .iter()
        .map(|&(field, dims)| {
            let rotator = Arc::new(DynamicRotator::new(dims, RotatorType::FhtKacRotator, 42));
            let padded_dims = rotator.padded_dim();
            ClusterFieldConfig {
                field,
                dims,
                padded_dims,
                ex_bits: 2,
                metric: Metric::L2,
                rotator,
                rotator_seed: 42,
            }
        })
        .collect();

    let cluster_config = ClusterConfig {
        fields: field_configs,
        clustering_threshold: 1000,
        num_clusters_fn: Arc::new(|n| (n as f64 / 250.0).ceil() as usize),
        kmeans: KMeansConfig::default(),
        sample_ratio: 0.1,
        sample_cap: 65536,
        sampler_factory: Arc::new(NoopSamplerFactory),
    };

    let bqvec_plugin = BqVecPlugin::builder().build();

    index.register_plugin(Arc::new(ClusterPlugin::new(cluster_config)));
    index.register_plugin(Arc::new(bqvec_plugin));
}

fn register_vector_plugins_with_sampler(
    index: &mut tantivy::Index,
    heap_oid: pg_sys::Oid,
    field_info: HashMap<Field, VectorFieldInfo>,
) {
    let schema = index.schema();
    let vector_fields: Vec<(Field, usize)> = schema
        .fields()
        .filter_map(|(field, entry)| {
            if let FieldType::Vector(opts) = entry.field_type() {
                Some((field, opts.dimensions))
            } else {
                None
            }
        })
        .collect();

    if vector_fields.is_empty() {
        return;
    }

    let field_configs: Vec<ClusterFieldConfig> = vector_fields
        .iter()
        .map(|&(field, dims)| {
            let rotator = Arc::new(DynamicRotator::new(dims, RotatorType::FhtKacRotator, 42));
            let padded_dims = rotator.padded_dim();
            ClusterFieldConfig {
                field,
                dims,
                padded_dims,
                ex_bits: 2,
                metric: Metric::L2,
                rotator,
                rotator_seed: 42,
            }
        })
        .collect();

    let sampler_factory = Arc::new(PgHeapVectorSamplerFactory::new(heap_oid, field_info));

    let cluster_config = ClusterConfig {
        fields: field_configs,
        clustering_threshold: 1000,
        num_clusters_fn: Arc::new(|n| (n as f64 / 250.0).ceil() as usize),
        kmeans: KMeansConfig::default(),
        sample_ratio: 0.1,
        sample_cap: 65536,
        sampler_factory,
    };

    let bqvec_plugin = BqVecPlugin::builder().build();

    index.register_plugin(Arc::new(ClusterPlugin::new(cluster_config)));
    index.register_plugin(Arc::new(bqvec_plugin));
}

struct NoopSamplerFactory;

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
