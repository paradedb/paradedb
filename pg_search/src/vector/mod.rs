use tantivy::index::SegmentReader;
use tantivy::indexer::doc_id_mapping::SegmentDocIdMapping;
use tantivy::schema::Field;
use tantivy::vector::cluster::sampler::{VectorSampler, VectorSamplerFactory};
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
