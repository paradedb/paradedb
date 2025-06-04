use crate::api::{HashMap, HashSet};
use crate::postgres::storage::block::SegmentMetaEntry;
use pgrx::pg_sys;
use std::cmp::Reverse;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tantivy::index::{DeleteMeta, InnerSegmentMeta, SegmentId};
use tantivy::indexer::{MergeCandidate, MergePolicy};
use tantivy::{Directory, Inventory, SegmentMeta};

#[derive(Debug)]
pub struct LayeredMergePolicy {
    #[allow(dead_code)]
    n: usize,
    layer_sizes: Vec<u64>,
    min_merge_count: usize,
    enable_logging: bool,

    mergeable_segments: HashMap<SegmentId, SegmentMetaEntry>,
    already_processed: AtomicBool,
}

pub type NumCandidates = usize;
pub type NumMerged = usize;

impl MergePolicy for LayeredMergePolicy {
    fn compute_merge_candidates(
        &self,
        directory: Option<&dyn Directory>,
        original_segments: &[SegmentMeta],
    ) -> Vec<MergeCandidate> {
        let logger = |directory: Option<&dyn Directory>, message: &str| {
            if self.enable_logging {
                if let Some(directory) = directory {
                    directory.log(message);
                } else {
                    pgrx::debug1!("{message}");
                }
            }
        };

        if original_segments.is_empty() {
            logger(directory, "compute_merge_candidates: no segments to merge");
            return Vec::new();
        }
        if self.already_processed.load(Ordering::Relaxed) {
            logger(
                directory,
                "compute_merge_candidates: already processed segments, skipping merge",
            );
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
            / self
                .mergeable_segments
                .values()
                .map(|entry| (entry.num_docs() + entry.num_deleted_docs()) as u64)
                .sum::<u64>();

        let mut candidates = Vec::new();
        let mut merged_segments = HashSet::default();
        let mut layer_sizes = self.layer_sizes.clone();
        layer_sizes.sort_by_key(|size| Reverse(*size)); // largest to smallest

        for layer_size in layer_sizes {
            // individual segments that total a certain byte amount typically merge together into
            // a segment of a smaller size than the individual source segments.  So we fudge things
            // by a third more in the hopes the final segment will be >= to this layer size, ensuring
            // it doesn't merge again
            let extended_layer_size = layer_size + layer_size / 3;

            // collect the list of mergeable segments so that we can combine those that fit in the next layer
            let segments =
                self.collect_mergeable_segments(original_segments, &merged_segments, avg_doc_size);
            let mut candidate_byte_size = 0;
            candidates.push((layer_size, MergeCandidate(vec![])));

            for segment in segments {
                if merged_segments.contains(&segment.id()) {
                    // we've already merged it
                    continue;
                }

                if self.segment_size(segment, avg_doc_size) > layer_size {
                    // this segment is larger than this layer_size... skip it
                    continue;
                }

                // add this segment as a candidate
                candidate_byte_size +=
                    actual_byte_size(segment, &self.mergeable_segments, avg_doc_size);
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

        logger(
            directory,
            &format!(
                "compute_merge_candidates: candidates before min merge count are {:?}",
                candidates
            ),
        );

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

        logger(
            directory,
            &format!(
                "compute_merge_candidates: final candidates are {:?}",
                candidates
            ),
        );

        candidates
            .into_iter()
            .map(|(_, candidate)| candidate)
            .collect()
    }
}

impl LayeredMergePolicy {
    pub fn new(layer_sizes: Vec<u64>) -> LayeredMergePolicy {
        Self {
            n: std::thread::available_parallelism()
                .expect("your computer should have at least one CPU")
                .get(),
            layer_sizes,
            min_merge_count: 2,
            enable_logging: unsafe { pg_sys::message_level_is_interesting(pg_sys::DEBUG1 as _) },

            mergeable_segments: Default::default(),
            already_processed: Default::default(),
        }
    }

    pub fn set_mergeable_segment_entries(
        &mut self,
        mergeable_segments: impl Iterator<Item = (SegmentId, SegmentMetaEntry)>,
    ) {
        self.mergeable_segments = mergeable_segments.collect();
    }

    /// Run a simulation of what tantivy will do if it were to call our [`MergePolicy::compute_merge_candidates`]
    /// implementation
    pub fn simulate(&mut self) -> (Vec<MergeCandidate>, NumMerged) {
        // we don't want the whole world to know how to do this conversion
        #[allow(non_local_definitions)]
        impl From<SegmentMetaEntry> for SegmentMeta {
            fn from(value: SegmentMetaEntry) -> Self {
                Self {
                    tracked: Inventory::new().track(InnerSegmentMeta {
                        segment_id: value.segment_id,
                        max_doc: value.max_doc,
                        deletes: value.delete.map(|delete_entry| DeleteMeta {
                            num_deleted_docs: delete_entry.num_deleted_docs,
                            opstamp: 0,
                        }),
                        include_temp_doc_store: Arc::new(Default::default()),
                    }),
                }
            }
        }

        let segment_metas = self
            .mergeable_segments
            .values()
            .cloned()
            .map(From::from)
            .collect::<Vec<SegmentMeta>>();
        let candidates = self.compute_merge_candidates(None, &segment_metas);
        let nmerged = candidates.iter().flat_map(|candidate| &candidate.0).count();
        let segment_ids = candidates
            .iter()
            .flat_map(|candidate| &candidate.0)
            .collect();

        self.retain(segment_ids);

        (candidates, nmerged)
    }

    fn retain(&mut self, to_keep: HashSet<&SegmentId>) {
        self.mergeable_segments
            .retain(|segment_id, _| to_keep.contains(segment_id));
    }

    pub fn mergeable_segments(&self) -> impl Iterator<Item = &SegmentId> {
        self.mergeable_segments.keys()
    }

    fn collect_mergeable_segments<'a>(
        &self,
        segments: &'a [SegmentMeta],
        exclude: &HashSet<SegmentId>,
        avg_doc_size: u64,
    ) -> Vec<&'a SegmentMeta> {
        let mut segments = segments
            .iter()
            .filter(|meta| {
                self.mergeable_segments.contains_key(&meta.id()) && !exclude.contains(&meta.id())
            })
            .collect::<Vec<_>>();

        // sort largest to smallest
        segments.sort_by_key(|segment| Reverse(self.segment_size(segment, avg_doc_size)));
        segments
    }

    fn segment_size(&self, segment: &SegmentMeta, avg_doc_size: u64) -> u64 {
        adjusted_byte_size(segment, &self.mergeable_segments, avg_doc_size)
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

#[inline]
fn adjusted_byte_size(
    meta: &SegmentMeta,
    all_entries: &HashMap<SegmentId, SegmentMetaEntry>,
    avg_doc_size: u64,
) -> u64 {
    if meta.num_docs() == 0 {
        return 0;
    }

    let adjusted_byte_size = all_entries
        .get(&meta.id())
        .map(|entry| {
            entry
                .byte_size()
                .saturating_sub(entry.num_deleted_docs() as u64 * avg_doc_size)
        })
        .unwrap_or(meta.num_docs() as u64 * avg_doc_size)
        .max(avg_doc_size);
    pgrx::warning!("adjusted byte size = {}", adjusted_byte_size);
    adjusted_byte_size
}
