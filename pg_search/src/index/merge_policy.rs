use std::collections::HashSet;
use tantivy::index::SegmentId;
use tantivy::indexer::{MergeCandidate, MergePolicy};
use tantivy::SegmentMeta;
macro_rules! my_eprintln {
    () => {
        // eprintln!()
    };
    ($($arg:tt)*) => {{
        // eprintln!($($arg)*);
    }};
}

#[derive(Debug, Clone)]
pub enum AllowedMergePolicy {
    None,
    NPlusOne,
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

    pub vacuum_list: HashSet<SegmentId>,
}

impl MergePolicy for NPlusOneMergePolicy {
    fn compute_merge_candidates(&self, segments: &[SegmentMeta]) -> Vec<MergeCandidate> {
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
            .filter(|s| !self.vacuum_list.contains(&s.id()))
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
        let (mean, stddev) = mean_stddev(segments.iter().map(|s| s.num_docs())).unwrap();
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

            small_segments = segments.iter().take(segments.len() - self.n + 1).collect();

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
                    "{} segments in candidate, size={current_candidate_byte_size}",
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
            "{} segments in candidate, size={current_candidate_byte_size}",
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
        let variance = m2 / (count as f64);
        Some((mean, variance.sqrt()))
    }
}
