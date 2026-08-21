//! Work distribution for a parallel vector scan, in one place: the tier
//! partition, the DSM segment sources, the participant count worth
//! launching, and the claim protocol.
//!
//! The clustered segments form ONE unit of work — they share a routing
//! pass and a heap (tantivy's `CollectorMode::MultiSegment`) — and each
//! flat (mutable-tier) segment is its own unit: exact and independent. A
//! participant claims units until none remain, so a lost worker costs
//! parallelism, never results.

use crate::api::OrderByFeature;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::ParallelScanState;
use crate::postgres::customscan::builders::custom_path::ExecMethodType;
use crate::postgres::rel::PgSearchRelation;
use tantivy::SegmentReader;
use tantivy::index::SegmentId;

/// DSM source index of the clustered segments (the one coupled chunk).
const CLUSTERED_SOURCE: usize = 0;
/// DSM source index of the flat segments (one unit each).
const FLAT_SOURCE: usize = 1;

/// A vector scan's snapshot partitioned by tier.
pub struct VectorScanWork {
    clustered: Vec<SegmentReader>,
    flat: Vec<SegmentReader>,
}

/// One atomic claim — one UNIT of work: either the WHOLE clustered chunk
/// or one flat segment. A participant's total claim is a `Vec` of these,
/// each mapping 1:1 onto a `collect_ivf` / `collect_flat` call.
#[derive(Debug, Clone)]
pub enum VectorClaim {
    Clustered(Vec<SegmentId>),
    Flat(SegmentId),
}

impl VectorClaim {
    pub fn segment_ids(&self) -> Vec<SegmentId> {
        match self {
            VectorClaim::Clustered(ids) => ids.clone(),
            VectorClaim::Flat(id) => vec![*id],
        }
    }
}

impl VectorScanWork {
    /// Partition `reader`'s snapshot by tier for `method`'s vector ORDER
    /// BY; `None` when the scan is not one (or a vector reader fails to
    /// open, which falls back to the single-source layout).
    pub fn from_scan(method: &ExecMethodType, reader: &SearchIndexReader) -> Option<Self> {
        let ExecMethodType::TopK {
            orderby_info: Some(infos),
            ..
        } = method
        else {
            return None;
        };
        let OrderByFeature::VectorDistance { name, .. } = &infos.first()?.feature else {
            return None;
        };
        let field = reader.schema().search_field(name)?.field();
        let mut clustered = Vec::new();
        let mut flat = Vec::new();
        for segment in reader.segment_readers() {
            let vector = segment.vector_index(field).ok()?;
            if vector.clusters().is_some() {
                clustered.push(segment.clone());
            } else {
                flat.push(segment.clone());
            }
        }
        Some(Self { clustered, flat })
    }

    /// Plan-time construction over a fresh snapshot of `indexrel`.
    pub fn for_index(indexrel: &PgSearchRelation, method: &ExecMethodType) -> Option<Self> {
        let reader = SearchIndexReader::empty(indexrel, MvccSatisfies::Snapshot).ok()?;
        Self::from_scan(method, &reader)
    }

    pub fn has_flat(&self) -> bool {
        !self.flat.is_empty()
    }

    /// Participants worth running: one per flat segment plus one for the
    /// clustered chunk.
    pub fn desired_participants(&self) -> usize {
        self.flat.len() + usize::from(!self.clustered.is_empty())
    }

    /// The DSM segment sources, indexed by [`CLUSTERED_SOURCE`] and
    /// [`FLAT_SOURCE`].
    pub fn sources(&self) -> Vec<&[SegmentReader]> {
        vec![&self.clustered, &self.flat]
    }

    /// One atomic claim from the DSM: the whole clustered chunk if still
    /// unclaimed, else one flat segment, else `None`.
    ///
    /// # Safety
    /// `pscan_state` must point at an initialized [`ParallelScanState`]
    /// whose sources were registered from [`Self::sources`].
    pub unsafe fn claim(pscan_state: *mut ParallelScanState) -> Option<VectorClaim> {
        let clustered = (*pscan_state).checkout_all_for_source(CLUSTERED_SOURCE);
        if !clustered.is_empty() {
            return Some(VectorClaim::Clustered(clustered));
        }
        (*pscan_state)
            .checkout_segment_for_source(FLAT_SOURCE)
            .map(VectorClaim::Flat)
    }
}
