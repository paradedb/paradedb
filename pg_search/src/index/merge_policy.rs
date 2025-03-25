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
    pub enable_logging: bool,
    pub layer_sizes: Vec<u64>,
    pub mergeable_segments: HashMap<SegmentId, SegmentMetaEntry>,
    pub already_processed: AtomicBool,
}

impl MergePolicy for LayeredMergePolicy {
    fn compute_merge_candidates(
        &self,
        directory: Option<&dyn Directory>,
        original_segments: &[SegmentMeta],
    ) -> Vec<MergeCandidate> {
        let directory = directory.expect("Directory should be provided to MergePolicy");

        if original_segments.is_empty() {
            directory.log("compute_merge_candidates: no segments to merge");
            return Vec::new();
        }
        if self.already_processed.load(Ordering::Relaxed) {
            directory.log("compute_merge_candidates: already processed segments, skipping merge");
            return Vec::new();
        }

        assert!(
            !self.mergeable_segments.is_empty(),
            "must have mergeable segments"
        );
        let avg_doc_size = self
            .mergeable_segments
            .values()
            .map(|entry| entry.byte_size())
            .sum::<u64>()
            / self.mergeable_segments.len() as u64;

        let mut candidates = Vec::new();
        let mut merged_segments = HashSet::new();
        let mut layer_sizes = self.layer_sizes.clone();
        layer_sizes.sort_by_key(|size| Reverse(*size)); // largest to smallest

        for layer_size in layer_sizes {
            // individual segments that total a certain byte amount typically merge together into
            // a segment of a smaller size than the individual source segments.  So we fudge things
            // by a third more in the hopes the final segment will be >= to this layer size, ensuring
            // it doesn't merge again
            let extended_layer_size = layer_size + layer_size / 3;

            // collect the list of mergeable segments so that we can combine those that fit in the next layer
            let segments = self.collect_mergeable_segments(original_segments, &merged_segments);
            let mut candidate_byte_size = 0;
            candidates.push((layer_size, MergeCandidate(vec![])));

            for segment in segments {
                if merged_segments.contains(&segment.id()) {
                    // we've already merged it
                    continue;
                }

                let byte_size = actual_byte_size(segment, &self.mergeable_segments, avg_doc_size);
                if byte_size > layer_size {
                    // this segment is larger than this layer_size... skip it
                    continue;
                }

                // add this segment as a candidate
                candidate_byte_size += byte_size;
                candidates.last_mut().unwrap().1 .0.push(segment.id());

                if candidate_byte_size >= extended_layer_size {
                    // the candidate now exceeds the layer size so we start a new candidate
                    candidate_byte_size = 0;
                    candidates.push((layer_size, MergeCandidate(vec![])));
                }
            }

            if candidate_byte_size < extended_layer_size {
                // the last candidate isn't full, so throw it away
                candidates.pop();
            }

            // remember the segments we have merged so we don't merge them again
            for candidate in &candidates {
                merged_segments.extend(candidate.1 .0.clone());
            }
        }

        if self.enable_logging {
            directory.log(&format!(
                "compute_merge_candidates: candidates before min merge count are {:?}",
                candidates
            ));
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

        if self.enable_logging {
            directory.log(&format!(
                "compute_merge_candidates: final candidates are {:?}",
                candidates
            ));
        }

        candidates
            .into_iter()
            .map(|(_, candidate)| candidate)
            .collect()
    }
}

impl LayeredMergePolicy {
    fn collect_mergeable_segments<'a>(
        &self,
        segments: &'a [SegmentMeta],
        exclude: &HashSet<SegmentId>,
    ) -> Vec<&'a SegmentMeta> {
        let mut segments = segments
            .iter()
            .filter(|meta| {
                self.mergeable_segments.contains_key(&meta.id()) && !exclude.contains(&meta.id())
            })
            .collect::<Vec<_>>();

        // sort largest to smallest
        segments.sort_by_key(|segment| Reverse(self.segment_size(&segment.id())));
        segments
    }

    // NB: just for logging purposes
    #[inline]
    fn segment_size(&self, segment_id: &SegmentId) -> (u64, usize) {
        self.mergeable_segments
            .get(segment_id)
            .map(|entry| (entry.byte_size(), entry.num_docs()))
            .unwrap_or((0, 0))
    }
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
