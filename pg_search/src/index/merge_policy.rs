use crate::postgres::storage::block::SegmentMetaEntry;
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use tantivy::index::SegmentId;
use tantivy::indexer::{MergeCandidate, MergePolicy};
use tantivy::{Directory, SegmentMeta};

#[derive(Debug)]
pub struct LayeredMergePolicy {
    pub n: usize,
    pub min_merge_count: usize,
    pub layer_sizes: Vec<u64>,
    pub vacuum_list: HashSet<SegmentId>,
    pub segment_entries: HashMap<SegmentId, SegmentMetaEntry>,
    pub already_processed: AtomicBool,
}

impl MergePolicy for LayeredMergePolicy {
    fn compute_merge_candidates(
        &self,
        directory: Option<&dyn Directory>,
        original_segments: &[SegmentMeta],
    ) -> Vec<MergeCandidate> {
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
        let mut layer_sizes = layer_sizes.iter().cloned().peekable();

        while let Some(layer_size) = layer_sizes.next() {
            let next_layer_size = layer_sizes.peek().cloned().unwrap_or(0);

            // collect the list of mergeable segments so that we can combine those that fit in the next layer
            let segments = collect_mergable_segments(original_segments, self, &merged_segments);

            let mut candidate_byte_size = 0;
            candidates.push((layer_size, MergeCandidate(vec![])));

            directory
                .unwrap()
                .log(&format!("rolling up into {layer_size}"));
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
                    directory.unwrap().log(&format!(
                        "removing candidate with only {} segments",
                        candidates[i].1 .0.len()
                    ));
                    candidates.remove(i);
                    continue 'outer;
                }
            }
            break;
        }

        // pop off merge candidates until we'll have at least `self.n` segments remaining
        let mut ndropped = 0;
        while !candidates.is_empty()
            && original_segments.len()
                - candidates
                    .iter()
                    .map(|candidate| candidate.1 .0.len())
                    .sum::<usize>()
                + candidates.len()
                < self.n
        {
            ndropped += 1;
            candidates.pop();
        }
        if ndropped > 0 {
            directory
                .unwrap()
                .log(&format!("dropped {ndropped} candidates"));
        }

        if !candidates.is_empty() {
            self.already_processed.store(true, Ordering::Relaxed);

            directory.unwrap().log(&format!(
                "candidates: {:#?}",
                candidates
                    .iter()
                    .map(|candidate| {
                        format!(
                            "layer={}, merging {} segments, totalling {} bytes",
                            candidate.0,
                            candidate.1 .0.iter().count(),
                            candidate
                                .1
                                 .0
                                .iter()
                                .map(|segment_id| segment_size(segment_id, self).0)
                                .sum::<u64>()
                        )
                    })
                    .collect::<Vec<_>>()
            ));
        } else {
            directory.unwrap().log("empty merge");
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
        .into_iter()
        .filter(|meta| {
            !merge_policy.vacuum_list.contains(&meta.id()) && !exclude.contains(&meta.id())
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
