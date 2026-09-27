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

use std::sync::{Arc, OnceLock};

use crate::api::HashMap;
use crate::index::stats::{EmpiricalStats, PartitionSegments, SegmentInclusion, SegmentStats};
use crate::scan::range_partitioning::PartitionRange;
use crate::schema::SearchField;
use tantivy::index::SegmentId;
use tantivy::{Searcher, SegmentOrdinal};

/// One segment in the snapshot, at the same ordinal as its reader in the searcher.
///
/// Statistics are opened on first use. A cached `None` means the segment has no `.stats`
/// component, as with mutable segments; failures to open an existing component abort the query.
struct CapturedSegment {
    id: SegmentId,
    stats: OnceLock<Option<SegmentStats>>,
}

/// Lazily opened statistics tied to one frozen searcher.
///
/// The retained searcher fixes segment membership and order for this snapshot's lifetime.
/// Readers created from the same manifest share the snapshot, so a segment's `.stats` component
/// is opened at most once. Individual field entries are decoded when requested.
///
/// Missing statistics cannot exclude a segment. Errors opening or decoding existing statistics
/// abort the query rather than concealing an unreadable index component.
pub(crate) struct SegmentStatsSnapshot {
    searcher: Searcher,
    segments: Box<[CapturedSegment]>,
    ordinal_by_id: HashMap<SegmentId, usize>,
}

impl SegmentStatsSnapshot {
    /// Capture the segments `searcher` was built on: one `Arc` clone of the searcher, no I/O.
    pub(crate) fn capture(searcher: &Searcher) -> Arc<Self> {
        let segments = searcher
            .segment_readers()
            .iter()
            .map(|reader| CapturedSegment {
                id: reader.segment_id(),
                stats: OnceLock::new(),
            })
            .collect::<Box<[_]>>();
        let ordinal_by_id = segments
            .iter()
            .enumerate()
            .map(|(ord, segment)| (segment.id, ord))
            .collect();
        Arc::new(Self {
            searcher: searcher.clone(),
            segments,
            ordinal_by_id,
        })
    }

    #[cfg(any(test, feature = "pg_test"))]
    pub(crate) fn len(&self) -> usize {
        self.segments.len()
    }

    /// The searcher ordinal of `id`, or `None` when the segment is not in this snapshot.
    pub(crate) fn segment_index(&self, id: SegmentId) -> Option<usize> {
        self.ordinal_by_id.get(&id).copied()
    }

    /// Decode one field for one execution segment. Missing components or entries return None;
    /// read and conversion errors abort the query. Decoded entries are not cached.
    pub(crate) fn empirical(&self, ord: usize, field: &SearchField) -> Option<EmpiricalStats> {
        let stats = self.stats(ord)?;
        let segment_id = self.segments[ord].id;
        #[cfg(any(test, feature = "pg_test"))]
        test_support::EMPIRICAL_READS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let empirical = stats.empirical_for(field);
        #[cfg(any(test, feature = "pg_test"))]
        let empirical = test_support::maybe_fail(
            segment_id,
            test_support::InjectedStatsFailure::Empirical,
            empirical,
        );
        empirical.unwrap_or_else(|error| {
            pgrx::error!("could not read empirical statistics for field {:?} in segment {segment_id:?}: {error}", field.field())
        })
    }

    fn stats(&self, ord: usize) -> Option<&SegmentStats> {
        self.segments[ord]
            .stats
            .get_or_init(|| {
                let reader = self.searcher.segment_reader(ord as SegmentOrdinal);
                #[cfg(any(test, feature = "pg_test"))]
                test_support::STATS_OPENS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let opened = SegmentStats::of_reader(reader);
                #[cfg(any(test, feature = "pg_test"))]
                let opened = test_support::maybe_fail(
                    reader.segment_id(),
                    test_support::InjectedStatsFailure::Open,
                    opened,
                );
                opened.unwrap_or_else(|error| {
                    pgrx::error!(
                        "could not open segment statistics for {:?}: {error}",
                        reader.segment_id()
                    )
                })
            })
            .as_ref()
    }

    /// Classify a segment's relation to this partition: fully included, partially included,
    /// or excluded. Missing statistics fall back to partially included; an unreadable entry
    /// aborts the query.
    pub(crate) fn classify_partition_segment(
        &self,
        ord: usize,
        field: &SearchField,
        range: &PartitionRange,
    ) -> SegmentInclusion {
        if !range.includes_nulls() && range.values().is_none() {
            return SegmentInclusion::Excluded;
        }
        let segment_id = self.segments[ord].id;
        let Some(stats) = self.stats(ord) else {
            return SegmentInclusion::PartiallyIncluded;
        };
        let empirical = self.empirical(ord, field);
        let logical = stats.logical(field.field());
        #[cfg(any(test, feature = "pg_test"))]
        let logical = test_support::maybe_fail(
            segment_id,
            test_support::InjectedStatsFailure::Logical,
            logical,
        );
        let logical = logical.unwrap_or_else(|error| {
            pgrx::error!(
                "could not read logical statistics for field {:?} in segment {segment_id:?}: {error}",
                field.field()
            )
        });
        let Some((lower, upper)) = range.values() else {
            if logical.is_some_and(|b| !b.may_hold_nulls()) {
                return SegmentInclusion::Excluded;
            }
            if empirical.is_some_and(|e| !e.nullable) {
                return SegmentInclusion::Excluded;
            }
            return SegmentInclusion::PartiallyIncluded;
        };
        match (logical, empirical) {
            (None, None) => SegmentInclusion::PartiallyIncluded,
            (Some(bounds), None) => {
                let intersects = (range.includes_nulls() && bounds.may_hold_nulls())
                    || bounds.intersects(lower, upper);
                if !intersects {
                    SegmentInclusion::Excluded
                } else if (range.includes_nulls() || !bounds.may_hold_nulls())
                    && bounds.is_subset_of(lower, upper)
                {
                    SegmentInclusion::FullyIncluded
                } else {
                    SegmentInclusion::PartiallyIncluded
                }
            }
            (None, Some(empirical)) => {
                let intersects = (range.includes_nulls() && empirical.nullable)
                    || empirical.intersects(lower, upper);
                if !intersects {
                    SegmentInclusion::Excluded
                } else if (range.includes_nulls() || !empirical.nullable)
                    && empirical.is_subset_of(lower, upper)
                {
                    SegmentInclusion::FullyIncluded
                } else {
                    SegmentInclusion::PartiallyIncluded
                }
            }
            (Some(bounds), Some(empirical)) => {
                let intersects = (range.includes_nulls() && empirical.nullable)
                    || (bounds.intersects(lower, upper) && empirical.intersects(lower, upper));
                if !intersects {
                    SegmentInclusion::Excluded
                } else {
                    let logical_subset = (range.includes_nulls() || !bounds.may_hold_nulls())
                        && bounds.is_subset_of(lower, upper);
                    let empirical_subset = (range.includes_nulls() || !empirical.nullable)
                        && empirical.is_subset_of(lower, upper);
                    if logical_subset || empirical_subset {
                        SegmentInclusion::FullyIncluded
                    } else {
                        SegmentInclusion::PartiallyIncluded
                    }
                }
            }
        }
    }

    /// Classify all execution segments for this partition into included, partially included,
    /// and pruned lists.
    pub(crate) fn classify_partition_segments(
        &self,
        field: &SearchField,
        range: &PartitionRange,
    ) -> PartitionSegments {
        let mut included = Vec::new();
        let mut partially_included = Vec::new();
        let mut pruned = Vec::new();
        for (ord, segment) in self.segments.iter().enumerate() {
            match self.classify_partition_segment(ord, field, range) {
                SegmentInclusion::FullyIncluded => included.push(segment.id),
                SegmentInclusion::PartiallyIncluded => partially_included.push(segment.id),
                SegmentInclusion::Excluded => pruned.push(segment.id),
            }
        }
        PartitionSegments {
            included,
            partially_included,
            pruned,
        }
    }

    /// Yield execution segment IDs whose available bounds overlap this partition, in searcher
    /// order. Missing statistics retain the segment; a read or decode error aborts the query.
    #[cfg(any(test, feature = "pg_test"))]
    pub(crate) fn segments_intersecting_partition<'a>(
        &'a self,
        field: &'a SearchField,
        range: &'a PartitionRange,
    ) -> impl Iterator<Item = SegmentId> + 'a {
        self.segments
            .iter()
            .enumerate()
            .filter(move |(ord, _)| {
                self.classify_partition_segment(*ord, field, range) != SegmentInclusion::Excluded
            })
            .map(|(_, segment)| segment.id)
    }
}

#[cfg(any(test, feature = "pg_test"))]
pub(crate) mod test_support {
    use super::SegmentId;
    use crate::api::HashMap;
    use std::io;
    use std::sync::atomic::AtomicUsize;
    use std::sync::{LazyLock, Mutex};

    /// Number of `.stats` open attempts, including ones that fail.
    pub(crate) static STATS_OPENS: AtomicUsize = AtomicUsize::new(0);
    pub(crate) static EMPIRICAL_READS: AtomicUsize = AtomicUsize::new(0);

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum InjectedStatsFailure {
        Open,
        Empirical,
        Logical,
    }

    struct Failure {
        operation: InjectedStatsFailure,
        hits: usize,
    }

    static FAILURES: LazyLock<Mutex<HashMap<SegmentId, Failure>>> =
        LazyLock::new(|| Mutex::new(HashMap::default()));

    pub(crate) struct InjectedStatsFailureGuard {
        segment_id: SegmentId,
    }

    impl InjectedStatsFailureGuard {
        pub(crate) fn hits(&self) -> usize {
            FAILURES
                .lock()
                .expect("injected statistics failure lock poisoned")
                .get(&self.segment_id)
                .expect("failure guard must be registered")
                .hits
        }
    }

    impl Drop for InjectedStatsFailureGuard {
        fn drop(&mut self) {
            // Runs during a failing test's unwind; a panic here would abort the backend.
            FAILURES
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(&self.segment_id);
        }
    }

    pub(crate) fn inject_stats_failure(
        segment_id: SegmentId,
        operation: InjectedStatsFailure,
    ) -> InjectedStatsFailureGuard {
        let replaced = FAILURES
            .lock()
            .expect("injected statistics failure lock poisoned")
            .insert(segment_id, Failure { operation, hits: 0 });
        assert!(
            replaced.is_none(),
            "one failure may be injected per segment"
        );
        InjectedStatsFailureGuard { segment_id }
    }

    /// Replace only a successful real call, so tests exercise the production error handler and
    /// cannot mistake an unrelated storage failure for reaching the requested boundary.
    pub(super) fn maybe_fail<T>(
        segment_id: SegmentId,
        operation: InjectedStatsFailure,
        result: io::Result<T>,
    ) -> io::Result<T> {
        let mut failures = FAILURES
            .lock()
            .expect("injected statistics failure lock poisoned");
        if let Some(failure) = failures.get_mut(&segment_id)
            && failure.operation == operation
            && result.is_ok()
        {
            failure.hits += 1;
            return Err(io::Error::other(format!(
                "injected {operation:?} statistics failure"
            )));
        }
        result
    }
}
