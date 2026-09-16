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
use crate::index::stats::{EmpiricalStats, LogicalBounds, SegmentStats};
use crate::postgres::types::is_datetime_type;
use crate::scan::range_partitioning::PartitionRange;
use crate::schema::{SearchField, SearchFieldType};
use tantivy::Searcher;
use tantivy::index::{Segment, SegmentId, SegmentReader};

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
    /// Seeded `Absent` when the manifest declares no `.stats`, so a mutable segment is never
    /// asked to open a component. Otherwise opened on first use: most readers never consult
    /// statistics, and opening `.stats` costs buffer reads per segment, so a reader open must
    /// not pay for it. Failed opens are cached too.
    stats: OnceLock<StatsRead<SegmentStats>>,
}

impl CapturedSegment {
    fn id(&self) -> SegmentId {
        self.segment.id()
    }

    fn stats(&self) -> &StatsRead<SegmentStats> {
        self.stats.get_or_init(|| capture_stats(&self.segment))
    }

    /// Logical bounds describe where the build routed rows; empirical bounds describe what
    /// the segment contains. Both must overlap when available. An unreadable entry keeps the
    /// segment, whereas an absent entry still permits the other kind of bounds to prune it.
    fn may_intersect_partition(&self, field: &SearchField, range: &PartitionRange) -> bool {
        let stats = match self.stats() {
            StatsRead::Known(stats) => stats,
            StatsRead::Absent | StatsRead::Unknown => return true,
        };
        let empirical = match read_empirical(stats, self.id(), field) {
            StatsRead::Known(value) => Some(value),
            StatsRead::Absent => None,
            StatsRead::Unknown => return true,
        };
        let logical = match read_logical(stats, self.id(), field) {
            StatsRead::Known(value) => Some(value),
            StatsRead::Absent => None,
            StatsRead::Unknown => return true,
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
            // Never panic here: this runs during a failing test's unwind, and a poisoned lock
            // would otherwise turn an assertion failure into a backend abort.
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

fn capture_stats(segment: &Segment) -> StatsRead<SegmentStats> {
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

fn read_empirical(
    stats: &SegmentStats,
    segment_id: SegmentId,
    field: &SearchField,
) -> StatsRead<EmpiricalStats> {
    let value = stats.empirical(field.field());
    #[cfg(any(test, feature = "pg_test"))]
    let value = test_support::maybe_fail(
        segment_id,
        test_support::InjectedStatsFailure::Empirical,
        value,
    );
    let value = match value {
        Ok(Some(value)) => value,
        Ok(None) => return StatsRead::Absent,
        Err(error) => {
            pgrx::debug1!(
                "segment pruning could not read field {:?} from .stats for {segment_id:?}: {error}",
                field.field()
            );
            return StatsRead::Unknown;
        }
    };
    if matches!(field.field_type(), SearchFieldType::I64(oid) if is_datetime_type(oid)) {
        match value.into_dates() {
            Some(value) => StatsRead::Known(value),
            None => {
                pgrx::debug1!(
                    "segment pruning could not lift field {:?} statistics to dates for {segment_id:?}",
                    field.field()
                );
                StatsRead::Unknown
            }
        }
    } else {
        StatsRead::Known(value)
    }
}

fn read_logical(
    stats: &SegmentStats,
    segment_id: SegmentId,
    field: &SearchField,
) -> StatsRead<LogicalBounds> {
    let value = stats.logical(field.field());
    #[cfg(any(test, feature = "pg_test"))]
    let value = test_support::maybe_fail(
        segment_id,
        test_support::InjectedStatsFailure::Logical,
        value,
    );
    match value {
        Ok(Some(value)) => StatsRead::Known(value),
        Ok(None) => StatsRead::Absent,
        Err(error) => {
            pgrx::debug1!(
                "segment partitioning could not read logical field {:?} from .stats for {segment_id:?}: {error}",
                field.field()
            );
            StatsRead::Unknown
        }
    }
}

impl SegmentStatsSnapshot {
    /// Capture lightweight segment handles for exactly the manifest `searcher` was built on.
    /// Manifest metadata decides whether `.stats` may be opened later, because opening any
    /// component path on a mutable segment would first materialize that entire segment even
    /// though mutable segments cannot have a persisted statistics component.
    pub(crate) fn capture(
        directory: &MVCCDirectory,
        segments: Vec<Segment>,
        searcher: &Searcher,
    ) -> Arc<Self> {
        debug_assert_eq!(
            segments.iter().map(Segment::id).collect::<Vec<_>>(),
            searcher
                .segment_readers()
                .iter()
                .map(SegmentReader::segment_id)
                .collect::<Vec<_>>(),
            "snapshot and searcher must name the same frozen manifest",
        );
        let captured = segments
            .into_iter()
            .map(|segment| CapturedSegment {
                stats: if directory.has_stats_component(&segment.id()) {
                    OnceLock::new()
                } else {
                    OnceLock::from(StatsRead::Absent)
                },
                segment,
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
