// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

use std::sync::{Arc, OnceLock};

use crate::index::mvcc::MVCCDirectory;
use crate::index::stats::{EmpiricalStats, SegmentStats};
use crate::postgres::types::is_datetime_type;
use crate::scan::range_partitioning::PartitionRange;
use crate::schema::{SearchField, SearchFieldType};
use tantivy::index::{Segment, SegmentId};

/// Absence describes missing statistics, never missing field values. Unknown means a read or
/// conversion failed; it must not be used as evidence for pruning.
#[derive(Debug)]
enum StatsRead<T> {
    Absent,
    Known(T),
    Unknown,
}

#[derive(Debug)]
struct CapturedSegment {
    segment: Segment,
    has_stats_component: bool,
    /// Opened on first use. Most readers never consult statistics, and opening `.stats` costs
    /// buffer reads per segment, so a reader open must not pay for it. Failed opens are cached too.
    stats: OnceLock<StatsRead<SegmentStats>>,
}

impl CapturedSegment {
    fn id(&self) -> SegmentId {
        self.segment.id()
    }

    fn stats(&self) -> &StatsRead<SegmentStats> {
        self.stats
            .get_or_init(|| capture_stats(&self.segment, self.has_stats_component))
    }

    fn read_empirical(&self, field: &SearchField) -> StatsRead<EmpiricalStats> {
        let stats = match self.stats() {
            StatsRead::Known(stats) => stats,
            StatsRead::Absent => return StatsRead::Absent,
            StatsRead::Unknown => return StatsRead::Unknown,
        };
        let value = stats.empirical(field.field());
        #[cfg(any(test, feature = "pg_test"))]
        let value = test_support::maybe_fail(
            self.id(),
            test_support::InjectedStatsFailure::Empirical,
            value,
        );
        let value = match value {
            Ok(Some(value)) => value,
            Ok(None) => return StatsRead::Absent,
            Err(error) => {
                pgrx::debug1!(
                    "segment pruning could not read field {:?} from .stats for {:?}: {error}",
                    field.field(),
                    self.id()
                );
                return StatsRead::Unknown;
            }
        };
        if matches!(field.field_type(), SearchFieldType::I64(oid) if is_datetime_type(oid)) {
            match value.into_dates() {
                Some(value) => StatsRead::Known(value),
                None => StatsRead::Unknown,
            }
        } else {
            StatsRead::Known(value)
        }
    }

    /// Logical bounds describe where the build routed rows; empirical bounds describe what
    /// the segment contains. Both must overlap when available. An unreadable entry keeps the
    /// segment, whereas an absent entry still permits the other kind of bounds to prune it.
    fn may_intersect_partition(&self, field: &SearchField, range: &PartitionRange) -> bool {
        let empirical = match self.read_empirical(field) {
            StatsRead::Known(value) => Some(value),
            StatsRead::Absent => None,
            StatsRead::Unknown => return true,
        };
        let StatsRead::Known(stats) = self.stats() else {
            return true;
        };
        let logical = stats.logical(field.field());
        #[cfg(any(test, feature = "pg_test"))]
        let logical = test_support::maybe_fail(
            self.id(),
            test_support::InjectedStatsFailure::Logical,
            logical,
        );
        let logical = match logical {
            Ok(value) => value,
            Err(error) => {
                pgrx::debug1!(
                    "segment partitioning could not read logical field {:?} from .stats for {:?}: {error}",
                    field.field(),
                    self.id()
                );
                return true;
            }
        };
        let (lower, upper) = (&range.lower, &range.upper);
        match (logical, empirical) {
            (None, None) => true,
            (Some(bounds), None) => {
                (range.includes_nulls && bounds.may_hold_nulls()) || bounds.intersects(lower, upper)
            }
            (None, Some(empirical)) => {
                (range.includes_nulls && empirical.nullable) || empirical.intersects(lower, upper)
            }
            (Some(bounds), Some(empirical)) => {
                (range.includes_nulls && empirical.nullable)
                    || (bounds.intersects(lower, upper) && empirical.intersects(lower, upper))
            }
        }
    }
}

/// Statistics captured from exactly one Searcher/manifest view.
#[derive(Debug)]
pub(crate) struct SegmentStatsSnapshot {
    segments: Box<[CapturedSegment]>,
}

#[cfg(any(test, feature = "pg_test"))]
pub(crate) mod test_support {
    use super::SegmentId;
    use crate::api::HashMap;
    use std::io;
    use std::sync::atomic::AtomicUsize;
    use std::sync::{LazyLock, Mutex};

    /// Number of `.stats` open attempts. Tests prove that ordinary searches perform zero opens
    /// and that the snapshot caches both successful and failed opens.
    pub(crate) static STATS_OPENS: AtomicUsize = AtomicUsize::new(0);

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
            FAILURES
                .lock()
                .expect("injected statistics failure lock poisoned")
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

fn capture_stats(segment: &Segment, has_stats_component: bool) -> StatsRead<SegmentStats> {
    if !has_stats_component {
        return StatsRead::Absent;
    }
    #[cfg(any(test, feature = "pg_test"))]
    test_support::STATS_OPENS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let opened = SegmentStats::of_segment(segment);
    #[cfg(any(test, feature = "pg_test"))]
    let opened = test_support::maybe_fail(
        segment.id(),
        test_support::InjectedStatsFailure::Open,
        opened,
    );
    match opened {
        Ok(Some(stats)) => StatsRead::Known(stats),
        Ok(None) => StatsRead::Absent,
        Err(error) => {
            pgrx::debug1!(
                "segment pruning could not open .stats for {:?}: {error}",
                segment.id()
            );
            StatsRead::Unknown
        }
    }
}

impl SegmentStatsSnapshot {
    /// Capture lightweight segment handles belonging to `directory`. Manifest metadata decides
    /// whether `.stats` may be opened later, because opening any component path on a mutable
    /// segment would first materialize that entire segment even though mutable segments cannot
    /// have a persisted statistics component.
    pub(crate) fn capture_segments(directory: &MVCCDirectory, segments: &[Segment]) -> Arc<Self> {
        let captured = segments
            .iter()
            .map(|segment| CapturedSegment {
                segment: segment.clone(),
                has_stats_component: directory.has_stats_component(&segment.id()),
                stats: OnceLock::new(),
            })
            .collect::<Box<[_]>>();
        Arc::new(Self { segments: captured })
    }

    #[cfg(any(test, feature = "pg_test"))]
    pub(crate) fn segment_ids(&self) -> impl ExactSizeIterator<Item = SegmentId> + '_ {
        self.segments.iter().map(CapturedSegment::id)
    }

    /// Resolve a partition against this execution's captured segments. Callers never have to
    /// pair an external enumeration with this snapshot's internal ordinals.
    pub(crate) fn segments_intersecting_partition<'a>(
        &'a self,
        field: &'a SearchField,
        range: &'a PartitionRange,
    ) -> impl Iterator<Item = SegmentId> + 'a {
        self.segments
            .iter()
            .filter(move |segment| segment.may_intersect_partition(field, range))
            .map(CapturedSegment::id)
    }
}
