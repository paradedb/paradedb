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

use crate::api::{HashMap, HashSet};
use crate::index::writer::index::SearchIndexMerger;
use crate::postgres::storage::block::SegmentMetaEntry;
use crate::postgres::storage::merge::MergeLock;
use crate::postgres::storage::metadata::MetaPage;
use pgrx::pg_sys;
use std::cmp::Reverse;
use std::sync::atomic::{AtomicBool, Ordering};
use tantivy::index::SegmentId;
use tantivy::indexer::{MergeCandidate, MergePolicy};
use tantivy::{Directory, Inventory, SegmentMeta};

#[derive(Debug)]
pub struct LayeredMergePolicy {
    layer_sizes: Vec<u64>,
    min_merge_count: usize,
    enable_logging: bool,

    mergeable_segments: HashMap<SegmentId, SegmentMetaEntry>,
    already_processed: AtomicBool,
}

impl MergePolicy for LayeredMergePolicy {
    fn compute_merge_candidates(
        &self,
        directory: Option<&dyn Directory>,
        original_segments: &[SegmentMeta],
    ) -> Vec<MergeCandidate> {
        let candidates = self.compute_merge_candidates_inner(directory, original_segments);
        candidates
            .into_iter()
            .map(|(_, candidate)| candidate)
            .collect()
    }
}

impl LayeredMergePolicy {
    pub fn new(layer_sizes: Vec<u64>) -> LayeredMergePolicy {
        Self {
            layer_sizes,
            min_merge_count: 2,
            enable_logging: unsafe { pg_sys::message_level_is_interesting(pg_sys::DEBUG1 as _) },

            mergeable_segments: Default::default(),
            already_processed: Default::default(),
        }
    }

    pub fn set_mergeable_segment_entries(
        &mut self,
        metadata: &MetaPage,
        merge_lock: &MergeLock,
        merger: &SearchIndexMerger,
    ) {
        let mut non_mergeable_segments = metadata.vacuum_list().read_list();
        // `list_segment_ids` borrows the `MergeList`, so it needs a binding that outlives the
        // `extend` call rather than a temporary.
        let merge_list = merge_lock.merge_list();
        non_mergeable_segments.extend(unsafe { merge_list.list_segment_ids() });

        if unsafe { pg_sys::message_level_is_interesting(pg_sys::DEBUG1 as _) } {
            pgrx::debug1!("do_merge: non_mergeable_segments={non_mergeable_segments:?}");
        }

        // tell the MergePolicy which segments it's initially allowed to consider for merging
        self.mergeable_segments = merger
            .all_entries()
            .into_iter()
            .filter(|(segment_id, _)| {
                // skip segments that are already being vacuumed or merged
                !non_mergeable_segments.contains(segment_id)
            })
            .collect();
    }

    #[cfg(any(test, feature = "pg_test"))]
    pub fn set_mergeable_segments_for_test(&mut self, segments: Vec<SegmentMetaEntry>) {
        self.mergeable_segments = segments
            .into_iter()
            .map(|entry| (entry.segment_id(), entry))
            .collect();
    }

    /// Run a simulation of what tantivy will do if it were to call our [`MergePolicy::compute_merge_candidates`]
    /// implementation
    pub fn simulate(&mut self) -> (Vec<MergeCandidate>, u64) {
        #[allow(non_local_definitions)]
        impl From<SegmentMetaEntry> for SegmentMeta {
            fn from(value: SegmentMetaEntry) -> Self {
                Self {
                    tracked: Inventory::new().track(value.as_tantivy()),
                }
            }
        }

        // Convert mergeable segment entries to SegmentMeta
        let segment_metas = self
            .mergeable_segments
            .values()
            .cloned()
            .map(From::from)
            .collect::<Vec<SegmentMeta>>();

        let candidates_with_sizes = self.compute_merge_candidates_inner(None, &segment_metas);

        let mut largest_layer_size = 0u64;
        let mut segment_ids = HashSet::default();
        let mut candidates = Vec::with_capacity(candidates_with_sizes.len());

        for (layer_size, candidate) in candidates_with_sizes {
            if layer_size > largest_layer_size {
                largest_layer_size = layer_size;
            }

            for &seg_id in &candidate.0 {
                segment_ids.insert(seg_id);
            }

            candidates.push(candidate);
        }

        // prune mergeable_segments to only those used in candidates
        self.retain(segment_ids);

        (candidates, largest_layer_size)
    }

    /// Checks vector headers only for selected candidates and repeats selection after exclusions.
    /// Each selection considers the full mergeable set minus unsupported segments, including
    /// supported segments that were not selected in an earlier iteration.
    /// Writes without merge candidates do not read vector components.
    pub fn simulate_supported(
        &mut self,
        merger: &SearchIndexMerger,
    ) -> tantivy::Result<(Vec<MergeCandidate>, u64, usize)> {
        self.simulate_with_validation(|candidates| merger.unsupported_vector_segments(candidates))
    }

    fn simulate_with_validation(
        &mut self,
        mut unsupported: impl FnMut(
            &HashMap<SegmentId, SegmentMetaEntry>,
        ) -> tantivy::Result<HashSet<SegmentId>>,
    ) -> tantivy::Result<(Vec<MergeCandidate>, u64, usize)> {
        let full_mergeable_set = self.mergeable_segments.clone();
        let mut excluded = HashSet::default();
        loop {
            self.mergeable_segments = full_mergeable_set
                .iter()
                .filter(|(id, _)| !excluded.contains(*id))
                .map(|(id, entry)| (*id, *entry))
                .collect();
            let (candidates, largest_layer_size) = self.simulate();
            if candidates.is_empty() {
                return Ok((candidates, largest_layer_size, excluded.len()));
            }
            let unsupported = unsupported(&self.mergeable_segments)?;
            if unsupported.is_empty() {
                return Ok((candidates, largest_layer_size, excluded.len()));
            }
            excluded.extend(unsupported);
            self.already_processed.store(false, Ordering::Relaxed);
        }
    }

    fn retain(&mut self, to_keep: HashSet<SegmentId>) {
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

    fn compute_merge_candidates_inner(
        &self,
        directory: Option<&dyn Directory>,
        original_segments: &[SegmentMeta],
    ) -> Vec<(u64, MergeCandidate)> {
        let logger = |directory: Option<&dyn Directory>, message: &str| {
            if self.enable_logging {
                if let Some(directory) = directory {
                    directory.log(message);
                } else if unsafe { pg_sys::message_level_is_interesting(pg_sys::DEBUG1 as _) } {
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

        let mut candidates: Vec<(u64, MergeCandidate)> = Vec::new();
        let mut merged_segments = HashSet::default();

        // aggressively merge away any mutable or completely empty segments
        for (segment_id, segment_meta_entry) in &self.mergeable_segments {
            if segment_meta_entry.is_mutable() {
                // If a segment is mutable, then it makes sense to merge it away, even if it is the only item in the segment.
                if let Some(segment_meta) = original_segments.iter().find(|s| s.id() == *segment_id)
                {
                    if let Some((_, mc)) = candidates.iter_mut().find(|(lvl, _)| *lvl == 0) {
                        mc.0.push(segment_meta.id());
                    } else {
                        candidates.push((0, MergeCandidate(vec![segment_meta.id()])));
                    }

                    merged_segments.insert(segment_meta.id());
                }
            } else if segment_meta_entry.num_docs() == 0 {
                // If it is not mutable, but is still empty for some reason, then we should include it in any other candidate level
                // that is planned (in order to get rid of it), but there is no point in doing a single-entry merge.
                if let Some(segment_meta) = original_segments.iter().find(|s| s.id() == *segment_id)
                    && let Some((_, mc)) = candidates.iter_mut().find(|(lvl, _)| *lvl == 0)
                {
                    mc.0.push(segment_meta.id());
                    merged_segments.insert(segment_meta.id());
                }
            }
        }

        let mut layer_sizes = self.layer_sizes.clone();
        layer_sizes.sort_by_key(|size| Reverse(*size)); // largest to smallest

        logger(directory, &format!("merged segments: {merged_segments:?}"));

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
                let segment_byte_size =
                    actual_byte_size(segment, &self.mergeable_segments, avg_doc_size);
                candidate_byte_size += segment_byte_size;
                candidates.last_mut().unwrap().1.0.push(segment.id());

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
                merged_segments.extend(candidate.1.0.clone());
            }
        }

        logger(
            directory,
            &format!(
                "compute_merge_candidates: candidates before min merge count are {candidates:?}"
            ),
        );

        // remove short candidate lists
        'outer: while !candidates.is_empty() {
            for i in 0..candidates.len() {
                let candidate_segments = &candidates[i].1.0;
                if candidate_segments.len() == 1 {
                    // this is a single-segment candidate, which we allow for mutable segments
                    let segment_id = &candidate_segments[0];
                    if let Some(entry) = self.mergeable_segments.get(segment_id)
                        && entry.is_mutable()
                    {
                        // it's a mutable segment conversion, keep it
                        continue;
                    }
                }

                if candidate_segments.len() < self.min_merge_count {
                    candidates.remove(i);
                    continue 'outer;
                }
            }
            break;
        }

        if !candidates.is_empty() {
            self.already_processed.store(true, Ordering::Relaxed);
        }

        logger(
            directory,
            &format!("compute_merge_candidates: final candidates are {candidates:?}"),
        );

        candidates
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

    all_entries
        .get(&meta.id())
        .map(|entry| {
            let fraction_alive =
                entry.num_docs() as f64 / (entry.num_docs() + entry.num_deleted_docs()) as f64;
            (entry.byte_size() as f64 * fraction_alive) as u64
        })
        .unwrap_or(meta.num_docs() as u64 * avg_doc_size)
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use crate::postgres::storage::block::{
        DeleteEntry, FileEntry, SegmentMetaEntry, SegmentMetaEntryImmutable,
        SegmentMetaEntryMutable,
    };
    use pgrx::pg_sys;
    use pgrx::prelude::*;
    use tantivy::index::SegmentId;

    fn create_segment_meta_entry(
        byte_size: u64,
        num_docs: u32,
        num_deleted_docs: u32,
    ) -> SegmentMetaEntry {
        let segment_id = SegmentId::generate_random();
        let max_doc = num_docs + num_deleted_docs;

        let delete_entry = if num_deleted_docs > 0 {
            Some(DeleteEntry {
                file_entry: FileEntry {
                    starting_block: pg_sys::InvalidBlockNumber,
                    total_bytes: 0, // assume delete file size is not important for merge policy
                },
                num_deleted_docs,
            })
        } else {
            None
        };

        let immutable = SegmentMetaEntryImmutable {
            postings: Some(FileEntry {
                starting_block: pg_sys::InvalidBlockNumber,
                total_bytes: byte_size as usize, // Corrected to usize
            }),
            delete: delete_entry,
            ..Default::default()
        };
        SegmentMetaEntry::new_immutable(
            segment_id,
            max_doc,
            pg_sys::InvalidTransactionId,
            immutable,
        )
    }

    fn create_mutable_segment_meta_entry(
        num_docs: u32,
        num_deleted_docs: u32,
        frozen: bool,
    ) -> SegmentMetaEntry {
        let segment_id = SegmentId::generate_random();
        let max_doc = num_docs + num_deleted_docs;

        let content = SegmentMetaEntryMutable {
            header_block: pg_sys::InvalidBlockNumber,
            num_deleted_docs,
            frozen,
            estimated_bytes: 0,
            persist_estimated_bytes: true,
        };

        SegmentMetaEntry::new_mutable(segment_id, max_doc, pg_sys::InvalidTransactionId, content)
    }

    #[pg_test]
    fn test_layered_merge_policy_eagerly_merges_mutable_segment() {
        let mut policy = LayeredMergePolicy::new(vec![1000]);
        // min_merge_count is 2 by default, but the single mutable segment should still be merged
        let segments = vec![create_mutable_segment_meta_entry(100, 0, false)];
        let segment_ids: Vec<_> = segments.iter().map(|s| s.segment_id()).collect();

        policy.set_mergeable_segments_for_test(segments);
        let (candidates, largest_layer_size) = policy.simulate();

        // We expect one candidate for converting the mutable segment
        assert_eq!(candidates.len(), 1);
        let candidate_ids: Vec<_> = candidates[0].0.to_vec();

        // The candidate should contain only our single mutable segment
        assert_eq!(candidate_ids.len(), 1);
        assert_eq!(candidate_ids[0], segment_ids[0]);
        assert_eq!(largest_layer_size, 0);
    }

    #[pg_test]
    fn test_layered_merge_policy_simple() {
        let mut policy = LayeredMergePolicy::new(vec![1000]);
        let segments = vec![
            create_segment_meta_entry(700, 70, 0),
            create_segment_meta_entry(700, 70, 0),
        ];
        let segment_ids: Vec<_> = segments.iter().map(|s| s.segment_id()).collect();

        policy.set_mergeable_segments_for_test(segments);
        let (candidates, largest_layer_size) = policy.simulate();

        assert_eq!(candidates.len(), 1);
        let candidate_ids: Vec<_> = candidates[0].0.to_vec();
        assert_eq!(candidate_ids.len(), 2);
        assert!(candidate_ids.contains(&segment_ids[0]));
        assert!(candidate_ids.contains(&segment_ids[1]));
        assert_eq!(largest_layer_size, 1000);
    }

    #[pg_test]
    fn test_layered_merge_policy_not_full_enough() {
        let mut policy = LayeredMergePolicy::new(vec![1000]);
        let segments = vec![
            create_segment_meta_entry(400, 40, 0),
            create_segment_meta_entry(400, 40, 0),
            create_segment_meta_entry(400, 40, 0),
        ];

        policy.set_mergeable_segments_for_test(segments);
        let (candidates, largest_layer_size) = policy.simulate();

        assert_eq!(candidates.len(), 0);
        assert_eq!(largest_layer_size, 0);
    }

    #[pg_test]
    fn merge_validation_does_not_open_unselected_segments() {
        let mut policy = LayeredMergePolicy::new(vec![1000]);
        let large = create_segment_meta_entry(2000, 200, 0);
        let large_id = large.segment_id();
        policy.set_mergeable_segments_for_test(vec![
            create_segment_meta_entry(700, 70, 0),
            create_segment_meta_entry(700, 70, 0),
            large,
        ]);
        let (candidates, _, skipped) = policy
            .simulate_with_validation(|selected| {
                assert_eq!(selected.len(), 2);
                assert!(!selected.contains_key(&large_id));
                Ok(HashSet::default())
            })
            .unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(skipped, 0);

        let mut policy = LayeredMergePolicy::new(vec![1000]);
        policy.set_mergeable_segments_for_test(vec![
            create_segment_meta_entry(400, 40, 0),
            create_segment_meta_entry(400, 40, 0),
        ]);
        let (candidates, _, skipped) = policy
            .simulate_with_validation(|_| panic!("no candidate may open a vector component"))
            .unwrap();
        assert!(candidates.is_empty());
        assert_eq!(skipped, 0);
    }

    #[pg_test]
    fn merge_validation_repeats_selection_after_exclusions() {
        let segments = (0..3)
            .map(|_| create_mutable_segment_meta_entry(100, 0, false))
            .collect::<Vec<_>>();
        let mut excluded = segments[..2]
            .iter()
            .map(SegmentMetaEntry::segment_id)
            .collect::<Vec<_>>();
        let retained = segments[2].segment_id();
        let mut policy = LayeredMergePolicy::new(vec![1000]);
        policy.set_mergeable_segments_for_test(segments);
        let mut checks = 0;
        let (candidates, _, skipped) = policy
            .simulate_with_validation(|selected| {
                checks += 1;
                assert!(selected.contains_key(&retained));
                Ok(excluded.pop().into_iter().collect())
            })
            .unwrap();
        assert_eq!(checks, 3);
        assert_eq!(skipped, 2);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].0, vec![retained]);
    }

    #[pg_test]
    fn merge_validation_reconsiders_unselected_segments() {
        let segments = vec![
            create_segment_meta_entry(900, 90, 0),
            create_segment_meta_entry(500, 50, 0),
            create_segment_meta_entry(450, 45, 0),
            create_segment_meta_entry(450, 45, 0),
        ];
        let unsupported_id = segments[0].segment_id();
        let first_selection = segments[..2]
            .iter()
            .map(SegmentMetaEntry::segment_id)
            .collect::<HashSet<_>>();
        let supported = segments[1..]
            .iter()
            .map(SegmentMetaEntry::segment_id)
            .collect::<HashSet<_>>();
        let mut policy = LayeredMergePolicy::new(vec![1000]);
        policy.set_mergeable_segments_for_test(segments);
        let mut checks = 0;
        let (candidates, largest_layer_size, skipped) = policy
            .simulate_with_validation(|selected| {
                checks += 1;
                let selected = selected.keys().copied().collect::<HashSet<_>>();
                if checks == 1 {
                    assert_eq!(selected, first_selection);
                    Ok([unsupported_id].into_iter().collect())
                } else {
                    assert_eq!(selected, supported);
                    Ok(HashSet::default())
                }
            })
            .unwrap();
        assert_eq!(checks, 2);
        assert_eq!(skipped, 1);
        assert_eq!(largest_layer_size, 1000);
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].0.iter().copied().collect::<HashSet<_>>(),
            supported
        );
        assert_eq!(
            policy.mergeable_segments().copied().collect::<HashSet<_>>(),
            supported
        );
    }

    #[pg_test]
    fn test_layered_merge_policy_min_merge_count() {
        let mut policy = LayeredMergePolicy::new(vec![1000]);
        policy.min_merge_count = 3;
        let segments = vec![
            create_segment_meta_entry(700, 70, 0),
            create_segment_meta_entry(700, 70, 0),
        ];

        policy.set_mergeable_segments_for_test(segments);
        let (candidates, largest_layer_size) = policy.simulate();

        assert_eq!(candidates.len(), 0);
        assert_eq!(largest_layer_size, 0);
    }

    #[pg_test]
    fn test_layered_merge_policy_multiple_layers() {
        let mut policy = LayeredMergePolicy::new(vec![1000, 10000]);
        let segments = vec![
            create_segment_meta_entry(700, 70, 0),
            create_segment_meta_entry(700, 70, 0),
            create_segment_meta_entry(7000, 700, 0),
            create_segment_meta_entry(7000, 700, 0),
        ];
        let segment_ids: Vec<_> = segments.iter().map(|s| s.segment_id()).collect();

        policy.set_mergeable_segments_for_test(segments);
        let (candidates, largest_layer_size) = policy.simulate();

        assert_eq!(candidates.len(), 2);

        let candidate1_ids: HashSet<_> = candidates[0].0.iter().cloned().collect();
        let candidate2_ids: HashSet<_> = candidates[1].0.iter().cloned().collect();

        let small_segment_ids: HashSet<_> = segment_ids[0..2].iter().cloned().collect();
        let large_segment_ids: HashSet<_> = segment_ids[2..4].iter().cloned().collect();

        if candidate1_ids == small_segment_ids {
            assert_eq!(candidate2_ids, large_segment_ids);
        } else {
            assert_eq!(candidate1_ids, large_segment_ids);
            assert_eq!(candidate2_ids, small_segment_ids);
        }
        assert_eq!(largest_layer_size, 10000);
    }
}
