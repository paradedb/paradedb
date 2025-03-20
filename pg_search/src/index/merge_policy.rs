use crate::postgres::storage::block::SegmentMetaEntry;
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use tantivy::index::SegmentId;
use tantivy::indexer::{MergeCandidate, MergePolicy};
use tantivy::{Directory, SegmentMeta};

#[derive(Debug)]
pub struct LayeredMergePolicy {
    #[allow(dead_code)]
    pub n: usize,
    pub min_merge_count: usize,
    pub layer_sizes: Vec<u64>,
    pub possibly_mergeable_segments: HashSet<SegmentId>,
    pub segment_entries: HashMap<SegmentId, SegmentMetaEntry>,
    pub already_processed: AtomicBool,
}

impl MergePolicy for LayeredMergePolicy {
    fn compute_merge_candidates(
        &self,
        _directory: Option<&dyn Directory>,
        original_segments: &[SegmentMeta],
    ) -> Vec<MergeCandidate> {
        if original_segments.is_empty() {
            return Vec::new();
        }
        if self.already_processed.load(Ordering::Relaxed) {
            return Vec::new();
        }

        let avg_doc_size = self
            .segment_entries
            .values()
            .map(|entry| entry.byte_size())
            .sum::<u64>()
            / self.segment_entries.len() as u64;

        let mut candidates = Vec::new();
        let mut merged_segments = HashSet::new();
        let mut layer_sizes = self.layer_sizes.clone();
        layer_sizes.sort_by_key(|size| Reverse(*size)); // largest to smallest

        for layer_size in layer_sizes {
            // collect the list of mergeable segments so that we can combine those that fit in the next layer
            let segments = collect_mergable_segments(original_segments, self, &merged_segments);

            let mut candidate_byte_size = 0;
            candidates.push((layer_size, MergeCandidate(vec![])));

            for segment in segments {
                if merged_segments.contains(&segment.id()) {
                    // we've already merged it
                    continue;
                }

                let byte_size = actual_byte_size(segment, &self.segment_entries, avg_doc_size);
                if byte_size > layer_size {
                    // this segment is larger than this layer_size... skip it
                    continue;
                }

                // add this segment as a candidate
                candidate_byte_size += byte_size;
                candidates.last_mut().unwrap().1 .0.push(segment.id());

                if candidate_byte_size >= layer_size + layer_size / 3 {
                    // the candidate now exceeds the layer size so we start a new candidate
                    candidate_byte_size = 0;
                    candidates.push((layer_size, MergeCandidate(vec![])));
                }
            }

            if candidate_byte_size < layer_size + layer_size / 3 {
                // the last candidate isn't full, so throw it away
                candidates.pop();
            }

            // remember the segments we have merged so we don't merge them again
            for candidate in &candidates {
                merged_segments.extend(candidate.1 .0.clone());
            }
        }

        // remove short candidate lists
        'outer: while !candidates.is_empty() {
            for i in 0..candidates.len() {
                if candidates[i].1 .0.len() < self.min_merge_count {
                    candidates.remove(i);
                    continue 'outer;
                }
            }
            break;
        }

        // // pop off merge candidates until we have at least `self.n` segments remaining
        // // this ensures we generally keep as many segments as "N", which is typically the CPU count
        // let mut ndropped = 0;
        // let mut ndropped_segments = 0;
        // while !candidates.is_empty()
        //     && original_segments.len()
        //         - candidates
        //             .iter()
        //             .map(|candidate| candidate.1 .0.len())
        //             .sum::<usize>()
        //         + candidates.len()
        //         < self.n
        // {
        //     ndropped += 1;
        //     if let Some(dropped) = candidates.pop() {
        //         ndropped_segments += dropped.1 .0.len();
        //     }
        // }

        if !candidates.is_empty() {
            self.already_processed.store(true, Ordering::Relaxed);
        }
        candidates
            .into_iter()
            .map(|(_, candidate)| candidate)
            .collect()
    }
}

fn collect_mergable_segments<'a>(
    segments: &'a [SegmentMeta],
    merge_policy: &LayeredMergePolicy,
    exclude: &HashSet<SegmentId>,
) -> Vec<&'a SegmentMeta> {
    let mut segments = segments
        .iter()
        .filter(|meta| {
            merge_policy
                .possibly_mergeable_segments
                .contains(&meta.id())
                && !exclude.contains(&meta.id())
        })
        .collect::<Vec<_>>();

    // sort largest to smallest
    segments.sort_by_key(|segment| Reverse(segment_size(&segment.id(), merge_policy)));
    segments
}

// NB: just for logging purposes
#[inline]
fn segment_size(segment_id: &SegmentId, merge_policy: &LayeredMergePolicy) -> (u64, usize) {
    merge_policy
        .segment_entries
        .get(segment_id)
        .map(|entry| (entry.byte_size(), entry.num_docs()))
        .unwrap_or((0, 0))
}

#[inline]
fn actual_byte_size(
    meta: &SegmentMeta,
    all_entries: &HashMap<SegmentId, SegmentMetaEntry>,
    avg_doc_size: u64,
) -> u64 {
    all_entries
        .get(&meta.id())
        .map(|entry| entry.byte_size())
        .unwrap_or(meta.num_docs() as u64 * avg_doc_size)
}
