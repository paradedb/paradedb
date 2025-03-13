use std::collections::HashSet;
use tantivy::index::SegmentId;
use tantivy::indexer::{MergeCandidate, MergePolicy};
use tantivy::{Directory, SegmentMeta};
macro_rules! my_eprintln {
    () => {
        // eprintln!()
    };
    ($($arg:tt)*) => {{
        // eprintln!($($arg)*);
    }};
}

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
}

impl MergePolicy for NPlusOneMergePolicy {
    fn compute_merge_candidates(
        &self,
        _directory: Option<&dyn Directory>,
        segments: &[SegmentMeta],
    ) -> Vec<MergeCandidate> {
        #[derive(Debug)]
        enum MergeBy {
            ByteSize,
            DocCount,
        }

        if segments.len() <= self.n {
            // too few segments of interest to merge
            return vec![];
        }

        my_eprintln!("---- compute_merge_candidates ---- ");
        my_eprintln!("#segments={}", segments.len());
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
                let keep = byte_size < self.segment_freeze_size as f64;
                if !keep {
                    my_eprintln!(
                        "rejecting segment: {:?}, size={}, docs={}",
                        s.id(),
                        byte_size,
                        s.num_docs(),
                    );
                }
                keep
            })
            .collect::<Vec<_>>();

        if segments.len() < self.min_merge_count {
            // not enough segments to even consider merging
            return vec![];
        }

        if segments.len() <= self.n + 1 {
            // we already have the right amount of segments
            my_eprintln!("already have the right amount of segments");
            return vec![];
        }

        // find all the segments, by live doc count, that are 1 (or more) standard deviation below the mean
        // these are the segments we'll merge together
        let Some((mut mean, stddev)) = mean_stddev(segments.iter().map(|s| s.num_docs())) else {
            return vec![];
        };
        my_eprintln!("mean={mean}, stddev={stddev}, nsegments={}", segments.len());
        let mut small_segments = segments
            .iter()
            .filter(|s| (s.num_docs() as f64) <= mean - stddev)
            .collect::<Vec<_>>();

        // sort smallest-to-larget
        small_segments.sort_unstable_by_key(|segment| segment.num_docs());
        my_eprintln!(
            "small_segments={:?}",
            small_segments
                .iter()
                .map(|s| (s.id(), s.num_docs()))
                .collect::<Vec<_>>()
        );

        if small_segments.len() <= self.min_merge_count && segments.len() <= self.n + 1 {
            // there's only 1 segment that falls below our cutoff threshold, so we'll just leave it
            my_eprintln!(
                "leaving small segment alone, id={}, size={}",
                small_segments[0].id(),
                small_segments[0].max_doc()
            );
            return vec![];
        }

        let mut merge_by = MergeBy::ByteSize;

        if small_segments.len() < self.min_merge_count && segments.len() > self.n {
            // we didn't come up with enough small segments to merge as they're all roughly the same
            // size, but we still have more than N segments.
            //
            // These segments are smaller than our "segment_freeze_size", so we'll merge the smallest
            // ones that would bring us back down to our "N"

            // sort smallest-to-larget
            segments.sort_unstable_by_key(|segment| segment.num_docs());

            small_segments = segments.iter().take(segments.len() - self.n).collect();

            // calculate a new mean from the remaining segments
            let Some((new_mean, _)) = mean_stddev(
                segments
                    .iter()
                    .skip(segments.len() - self.n)
                    .map(|s| s.num_docs()),
            ) else {
                return vec![];
            };
            mean = new_mean;
            my_eprintln!("new mean={mean}");

            // change our merging strategy to be by DocCount which keeps segments balanced by their
            // total "live" document count
            merge_by = MergeBy::DocCount;

            my_eprintln!(
                "more than N segments ({}), taking the first {} smallest which are {:?}",
                segments.len(),
                small_segments.len(),
                small_segments
                    .iter()
                    .map(|s| (s.id(), s.num_docs()))
                    .collect::<Vec<_>>()
            );
        }

        if small_segments.len() < self.min_merge_count {
            // not enough small segments to merge
            my_eprintln!("not enough small segments to merge");
            return vec![];
        }

        // group the small_segments together into sets of MergeCandidates, smallest to largest
        //
        // When the estimated byte size of a MergeCandidate crosses our `segment_freeze_size` we
        // start collecting another MergeCandidate
        my_eprintln!("---- merging, by {merge_by:?} ---- ");
        let mut candidates = vec![MergeCandidate(vec![])];
        let mut current_candidate_byte_size = 0;
        let mut current_candidate_docs = 0;

        for segment in small_segments {
            let byte_size = segment.max_doc() as usize * self.avg_byte_size_per_doc.ceil() as usize;

            my_eprintln!(
                "segment: {:?}, size={}, docs={}",
                segment.id(),
                byte_size,
                segment.num_docs(),
            );
            candidates.last_mut().unwrap().0.push(segment.id());
            current_candidate_byte_size += byte_size;
            current_candidate_docs += segment.num_docs();

            if (matches!(merge_by, MergeBy::DocCount)
                && current_candidate_docs >= mean.ceil() as u32)
                || (matches!(merge_by, MergeBy::ByteSize)
                    && current_candidate_byte_size >= self.segment_freeze_size)
            {
                my_eprintln!(
                    "{} segments in current candidate, size={current_candidate_byte_size}, docs={current_candidate_docs}",
                    candidates.last().unwrap().0.len()
                );
                // current `MergeCandidate` group is now as large as a segment is allowed to be,
                // so start another MergeCandidate to collect up the remaining segments
                candidates.push(MergeCandidate(vec![]));
                current_candidate_byte_size = 0;
                current_candidate_docs = 0;
            }
        }
        my_eprintln!(
            "{} segments in last candidate, size={current_candidate_byte_size}, docs={current_candidate_docs}",
            candidates.last().unwrap().0.len()
        );
        my_eprintln!("---- merging done, {} candidates ---- ", candidates.len());

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
        my_eprintln!("---- /compute_merge_candidates ---- ");

        candidates
    }
}

type Mean = f64;
type StdDev = f64;

fn mean_stddev<I: Iterator<Item = u32>>(iter: I) -> Option<(Mean, StdDev)> {
    let mut count = 0;
    let mut mean = 0.0;
    let mut m2 = 0.0;

    for x in iter {
        count += 1;
        let x = x as f64;
        let delta = x - mean;
        mean += delta / (count as f64);
        let delta2 = x - mean;
        m2 += delta * delta2;
    }

    if count == 0 {
        None
    } else {
        let variance = m2 / ((count - 1) as f64);
        Some((mean, variance.sqrt()))
    }
}

#[cfg(test)]
mod tests {
    use crate::index::merge_policy::{mean_stddev, NPlusOneMergePolicy};
    use serde::Deserialize;
    use serde_json::json;
    use std::collections::{HashMap, HashSet};
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    use tantivy::index::{DeleteMeta, InnerSegmentMeta, SegmentId};
    use tantivy::merge_policy::MergePolicy;
    use tantivy::{Inventory, SegmentMeta};

    #[test]
    fn test_mean_stddev() {
        let values = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let Some((mean, stddev)) = mean_stddev(values.into_iter()) else {
            panic!("failed to calculate mean and stddev")
        };
        assert_eq!(mean, 5.5);
        assert_eq!(stddev, 3.0276503540974917);
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
        };

        // we iteratively merge the segment list down until we get N+1 (9 in this case)
        // this is expected to merge iteratively as NPlusOneMergePolicy also tries to keep
        // segments balanced by size, so it takes a few iterations before it's able to do so
        // down to the count we expect
        for expectation in [21, 13, 11, 10, 9] {
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

            assert_eq!(
                segments.len() - total_merged + candidates.len(),
                expectation
            );

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
        }

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
