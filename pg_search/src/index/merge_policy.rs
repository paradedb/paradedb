use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use tantivy::index::SegmentId;
use tantivy::indexer::{MergeCandidate, MergePolicy};
use tantivy::{Directory, SegmentMeta};

/// A tantivy [`MergePolicy`] that endeavours to keep a maximum number of segments "N", plus
/// one extra for leftovers.
///
/// It merges the smallest segments, accounting for deleted docs.
#[derive(Debug)]
pub struct NPlusOneMergePolicy {
    // the number of segments we want to maintain on disk
    pub n: usize,

    // the minimum number of segments to merge together
    // if we don't have this many, no merge is performed
    pub min_merge_count: usize,

    pub avg_byte_size_per_doc: f64,

    // the size, in bytes, of a segment whereby we will not
    // to merge it at all
    pub segment_freeze_size: usize,

    // segments which are currently being vacuumed -- we cannot merge these
    pub vacuum_list: HashSet<SegmentId>,

    // we only want to run once per merge.  this lets us make that decision
    pub already_processed: AtomicBool,
}

impl MergePolicy for NPlusOneMergePolicy {
    fn compute_merge_candidates(
        &self,
        _directory: Option<&dyn Directory>,
        segments: &[SegmentMeta],
    ) -> Vec<MergeCandidate> {
        if self.already_processed.load(Ordering::Relaxed) {
            return vec![];
        }
        if segments.len() <= self.n {
            // too few segments of interest to merge
            return vec![];
        }

        // filter out any segments that are likely larger, on-disk, than the memory_budget configuration
        // these segments will live on disk, as-is, until they become smaller through deletes
        let mut segments = segments
            .iter()
            // filter out segments that are currently being vacuumed
            .filter(|s| !self.vacuum_list.contains(&s.id()))
            // filter out segments that are too big
            .filter(|s| {
                // estimate the byte size of this segment, accounting for only the *live* docs
                let byte_size = s.num_docs() as f64 * self.avg_byte_size_per_doc;

                // and we only accept, for merging, those whose estimated byte size is below our
                // `segment_freeze_size`
                byte_size < self.segment_freeze_size as f64
            })
            .collect::<Vec<_>>();

        // sort them smallest-to-largest, by # of alive docs
        segments.sort_unstable_by_key(|a| a.num_docs());

        let mut candidates = vec![MergeCandidate(vec![])];
        let mut adjusted_segment_count = segments.len() + 1;
        let mut current_candidate_size = 0;

        while adjusted_segment_count > self.n + 1 {
            if let Some(meta) = segments.pop() {
                let byte_size =
                    meta.num_docs() as usize * self.avg_byte_size_per_doc.ceil() as usize;

                if current_candidate_size >= self.segment_freeze_size {
                    candidates.push(MergeCandidate(vec![]));
                    adjusted_segment_count += 1;
                    current_candidate_size = 0;
                }

                candidates.last_mut().unwrap().0.push(meta.id());
                adjusted_segment_count -= 1;
                current_candidate_size += byte_size;
            } else {
                break;
            }
        }

        // remove short candidate lists
        'outer: while !candidates.is_empty() {
            for i in 0..candidates.len() {
                if candidates[i].0.len() < self.min_merge_count {
                    candidates.remove(i);
                    continue 'outer;
                }
            }
            break;
        }

        self.already_processed.store(true, Ordering::Relaxed);
        candidates
    }
}

#[cfg(test)]
mod tests {
    use crate::index::merge_policy::NPlusOneMergePolicy;
    use serde::Deserialize;
    use serde_json::json;
    use std::collections::{HashMap, HashSet};
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    use tantivy::index::{DeleteMeta, InnerSegmentMeta, SegmentId};
    use tantivy::merge_policy::MergePolicy;
    use tantivy::{Inventory, SegmentMeta};

    #[test]
    fn test_all_same_size() {
        let segments = vec![new_segment_meta(); 1000];
        let policy = NPlusOneMergePolicy {
            n: 8,
            min_merge_count: 2,
            avg_byte_size_per_doc: 116.0,
            segment_freeze_size: 10 * 1024 * 1024,
            vacuum_list: HashSet::new(),
            already_processed: Default::default(),
        };

        let candidates = policy.compute_merge_candidates(None, &segments);

        assert_eq!(candidates.len(), 11);
    }

    fn new_segment_meta() -> SegmentMeta {
        SegmentMeta {
            tracked: Inventory::new().track(InnerSegmentMeta {
                segment_id: SegmentId::generate_random(),
                max_doc: 1000,
                deletes: None,
                include_temp_doc_store: Arc::new(Default::default()),
            }),
        }
    }

    #[test]
    fn test_merge_candidates1() {
        let mut segments = to_segment_meta(example_segments());
        let policy = NPlusOneMergePolicy {
            n: 8,
            min_merge_count: 2,
            avg_byte_size_per_doc: 40.814_610_635_234_4,
            segment_freeze_size: 200 * 1024 * 1024,
            vacuum_list: HashSet::new(),
            already_processed: Default::default(),
        };

        let lookup = segments
            .iter()
            .map(|s| (s.id(), s))
            .collect::<HashMap<_, _>>();

        let candidates = policy.compute_merge_candidates(None, &segments);
        let mut total_merged = 0;
        for (i, candidate) in candidates.iter().enumerate() {
            eprintln!("CANDIDATE #{i}, count={}", candidate.0.len());
            for segment_id in &candidate.0 {
                let meta = lookup.get(segment_id).unwrap();
                eprintln!("{meta:?}");

                total_merged += 1;
            }
        }

        eprintln!(
            "remaining={}",
            segments.len() - total_merged + candidates.len()
        );

        assert_eq!(segments.len() - total_merged + candidates.len(), 9);

        let mut merged = Vec::new();
        let mut merged_ids = HashSet::new();
        for candidate in candidates {
            let inner_segment_meta = InnerSegmentMeta {
                segment_id: candidate.0[0],
                max_doc: candidate
                    .0
                    .iter()
                    .map(|s| lookup.get(s).unwrap().num_docs())
                    .sum::<u32>(),
                deletes: None,
                include_temp_doc_store: Arc::new(Default::default()),
            };
            merged.push(SegmentMeta {
                tracked: Inventory::new().track(inner_segment_meta),
            });
            merged_ids.extend(candidate.0.into_iter());
        }
        drop(lookup);
        segments.retain(|s| !merged_ids.contains(&s.id()));
        segments.append(&mut merged);

        eprintln!("FINAL SEGMENT SET: {segments:#?}");
    }

    #[test]
    fn test_merge_candidates2() {
        let mut segments = to_segment_meta(example_segments());
        // find a specific segment and change its max_doc value
        for meta in &mut segments {
            let uuid = meta.id().short_uuid_string();
            if uuid == "ceb78257" {
                let tmp = InnerSegmentMeta {
                    segment_id: meta.id(),
                    max_doc: 1,
                    deletes: meta.tracked.deletes.clone(),
                    include_temp_doc_store: meta.tracked.include_temp_doc_store.clone(),
                };
                let tracked = Inventory::new().track(tmp);
                *meta = SegmentMeta { tracked }
            }
        }
        let policy = NPlusOneMergePolicy {
            n: 32,
            min_merge_count: 2,
            avg_byte_size_per_doc: 40.814_610_635_234_4,
            segment_freeze_size: 200 * 1024 * 1024,
            vacuum_list: HashSet::new(),
            already_processed: Default::default(),
        };

        let candidates = policy.compute_merge_candidates(None, &segments);
        let mut total_merged = 0;
        for candidate in &candidates {
            total_merged += candidate.0.len();
        }

        eprintln!(
            "remaining={}",
            segments.len() - total_merged + candidates.len()
        );

        // we should have 33 segments remaining.  that's our N (32) + 1
        assert_eq!(segments.len() - total_merged + candidates.len(), 33)
    }

    fn to_segment_meta(json_data: serde_json::Value) -> Vec<SegmentMeta> {
        #[derive(Deserialize)]
        struct MySegmentMeta {
            segment_id: String,
            max_doc: u32,
            deletes: Option<DeleteMeta>,
        }

        let inventory = Inventory::new();
        let my_segment_metas = serde_json::from_value::<Vec<MySegmentMeta>>(json_data).unwrap();
        let mut result = Vec::with_capacity(my_segment_metas.len());
        for meta in my_segment_metas {
            let inner_segment_meta = InnerSegmentMeta {
                segment_id: SegmentId::from_uuid_string(&format!(
                    "{}000000000000000000000000",
                    meta.segment_id
                ))
                .unwrap(),
                max_doc: meta.max_doc,
                deletes: meta.deletes,
                include_temp_doc_store: Arc::new(AtomicBool::new(false)),
            };

            result.push(SegmentMeta {
                tracked: inventory.track(inner_segment_meta),
            });
        }
        result
    }

    fn example_segments() -> serde_json::Value {
        json!(
            [
              {
                "max_doc": 9,
                "segment_id": "fb72978d",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "c352fd24",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 6,
                "segment_id": "647b4479",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 37513,
                "segment_id": "356c65ff",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "1998ea6e",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "3f06d6ff",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "30cc4d92",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 111,
                "segment_id": "847621c7",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 37329,
                "segment_id": "4e8d58fb",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 9,
                "segment_id": "9c93ec2a",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 4,
                "segment_id": "be9c2770",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "6e6eaeed",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 111,
                "segment_id": "ddbc1de9",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 6,
                "segment_id": "ab61c641",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 2,
                "segment_id": "bf0cb6f3",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 2,
                "segment_id": "a6bbbfcc",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "55dcbe92",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "75b5efb0",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "51592029",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 2,
                "segment_id": "e69c45b6",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 36822,
                "segment_id": "4d1f558a",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 9,
                "segment_id": "4caaebe5",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 9,
                "segment_id": "d90a8ab7",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 36982,
                "segment_id": "62df9318",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 37083,
                "segment_id": "130b45e3",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 37263,
                "segment_id": "f74d6778",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "f88971d4",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "8dce5ab0",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "4dc6c5ba",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 9,
                "segment_id": "0a404aab",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 4,
                "segment_id": "cd5e79d0",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 4,
                "segment_id": "f6ca3425",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 111,
                "segment_id": "bbfe2eb7",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "abbf1301",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 2,
                "segment_id": "a9c92e7c",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 6,
                "segment_id": "e07b097f",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 111,
                "segment_id": "850f6d9d",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 2,
                "segment_id": "66f47ac4",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "bfceb31e",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 4,
                "segment_id": "cf148ad0",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 111,
                "segment_id": "e11fc732",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 111,
                "segment_id": "50b7b1c4",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 6,
                "segment_id": "f351ae6b",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 4,
                "segment_id": "5964d21d",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "71c66d02",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 4,
                "segment_id": "22d32531",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "f2c76113",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 6,
                "segment_id": "84414a07",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 6,
                "segment_id": "69d62bc1",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 9,
                "segment_id": "d01d8a0d",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 37279,
                "segment_id": "10d10824",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 9,
                "segment_id": "be2e8ead",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 9,
                "segment_id": "923f9de8",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 2,
                "segment_id": "22f130f2",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "ee8bbb7d",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 37065,
                "segment_id": "d572814a",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 6,
                "segment_id": "5d09ee86",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 37358,
                "segment_id": "28a75e84",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 37209,
                "segment_id": "c05e736d",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 111,
                "segment_id": "bf594b5a",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 36906,
                "segment_id": "05b8c067",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 9,
                "segment_id": "087378ca",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 9,
                "segment_id": "ff10e3b8",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "88694361",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 37312,
                "segment_id": "348cb8a4",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 9,
                "segment_id": "a0304d6c",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 2,
                "segment_id": "d2ef63fe",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 9,
                "segment_id": "3b68bbda",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "728e756b",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 9,
                "segment_id": "9d3204ab",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "0bc4f56f",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 37163,
                "segment_id": "8dab63b6",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 9,
                "segment_id": "051b29b3",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "e9d7e04c",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 6,
                "segment_id": "4a085e92",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 9,
                "segment_id": "d8471d17",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "ce53d4fb",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "2f95e5b5",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 4,
                "segment_id": "cca71f94",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "dc8abc20",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 37371,
                "segment_id": "06739c20",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 4,
                "segment_id": "c5e83836",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 6,
                "segment_id": "1d4d7f48",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 111,
                "segment_id": "effc73a7",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 37263,
                "segment_id": "d259a242",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 2,
                "segment_id": "2e7d1d47",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 9,
                "segment_id": "9f50dc7b",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 36885,
                "segment_id": "e7b95cea",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 9,
                "segment_id": "fd8cb30f",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "f0f09b0a",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 9,
                "segment_id": "ecb9e81f",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "97d5dd26",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 2,
                "segment_id": "2789a649",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "1f16cde9",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 2,
                "segment_id": "3a177bc9",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 6,
                "segment_id": "eeda1833",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 6,
                "segment_id": "9e54aaa7",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 9,
                "segment_id": "79e24adc",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 9,
                "segment_id": "671dbc34",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 37170,
                "segment_id": "08d434b2",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 6,
                "segment_id": "95fbda2a",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 6,
                "segment_id": "46d94f10",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 9,
                "segment_id": "08717f15",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "ecfef749",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 37248,
                "segment_id": "d2ba2262",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "7b94bcb8",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 9,
                "segment_id": "ab9258b2",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "bf19d85c",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 4,
                "segment_id": "f7739efd",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 111,
                "segment_id": "11a9d6cc",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 111,
                "segment_id": "3dfd5317",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 4,
                "segment_id": "2a7d6505",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 37383,
                "segment_id": "98f1f8ec",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 37140,
                "segment_id": "66214302",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "3c31ca0c",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 2,
                "segment_id": "467385f4",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 37343,
                "segment_id": "54991207",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "662fac0f",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "23c2fa7c",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "bac75eba",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "7412b612",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "7f581aaa",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 9,
                "segment_id": "76aa31b5",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "51d4b3cb",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 4,
                "segment_id": "0b580590",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 37136,
                "segment_id": "780eba84",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 2,
                "segment_id": "0aef04b0",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 37455,
                "segment_id": "b42de903",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 37106,
                "segment_id": "e4fe97e2",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 2,
                "segment_id": "003a12db",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 37048,
                "segment_id": "3afdd87a",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 37145,
                "segment_id": "a5b463d4",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 111,
                "segment_id": "7584e3ef",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "bfd90bea",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 37359,
                "segment_id": "525c1ade",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 37327,
                "segment_id": "c80aeb35",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 37186,
                "segment_id": "58faa753",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 6,
                "segment_id": "19b33d10",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 9,
                "segment_id": "94ee84d6",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 4,
                "segment_id": "801a1a71",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 37223,
                "segment_id": "7951ad47",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 4,
                "segment_id": "a74c9c81",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 2,
                "segment_id": "746be3ca",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "5bd1b492",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 2,
                "segment_id": "29cd08e0",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 9,
                "segment_id": "02ed71f5",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 4,
                "segment_id": "8551882f",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 4,
                "segment_id": "005de272",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 9,
                "segment_id": "cff668ae",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 4,
                "segment_id": "87ab5308",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 36765, /* 222 */
                "segment_id": "ceb78257",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 9,
                "segment_id": "0c230f93",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 37406,
                "segment_id": "8aa81a51",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 6,
                "segment_id": "380bb87c",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 37380,
                "segment_id": "8fa77f33",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              },
              {
                "max_doc": 1,
                "segment_id": "0a04aa95",
                "jsonb_build_object": {
                  "opstamp": 0,
                  "num_deleted_docs": 0
                }
              }
            ]
        )
    }
}
