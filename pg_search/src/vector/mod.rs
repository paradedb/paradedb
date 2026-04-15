use std::sync::Arc;

use tantivy::index::SegmentReader;
use tantivy::indexer::doc_id_mapping::SegmentDocIdMapping;
use tantivy::schema::{Field, FieldType};
use tantivy::vector::bqvec::BqVecPlugin;
use tantivy::vector::cluster::kmeans::KMeansConfig;
use tantivy::vector::cluster::plugin::{ClusterConfig, ClusterFieldConfig, ClusterPlugin};
use tantivy::vector::cluster::sampler::{VectorSampler, VectorSamplerFactory};
use tantivy::vector::rabitq::rotation::{DynamicRotator, RotatorType};
use tantivy::vector::rabitq::Metric;
use tantivy::DocId;

pub struct NoopSamplerFactory;

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
