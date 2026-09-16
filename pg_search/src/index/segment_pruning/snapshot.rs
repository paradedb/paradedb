// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

use std::fmt::{self, Display};
use std::io;
use std::ops::Bound;
use std::sync::{Arc, Mutex, OnceLock};

use crate::api::HashMap;
use crate::index::stats::{EmpiricalStats, LogicalBounds, SegmentStats};
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::scan::range_partitioning::PartitionRange;
use crate::schema::SearchField;
use tantivy::index::{SegmentId, SegmentReader};
use tantivy::schema::Field;
use tantivy::{Searcher, SegmentOrdinal};

/// Absence describes missing statistics, never missing field values. Unknown means a read or
/// conversion failed; it must not be used as evidence for pruning.
pub(super) enum StatsRead<T> {
    Absent,
    Known(T),
    Unknown,
}

impl<T> StatsRead<T> {
    pub(super) fn known(&self) -> Option<&T> {
        match self {
            Self::Known(value) => Some(value),
            Self::Absent | Self::Unknown => None,
        }
    }
}

struct CapturedSegment {
    id: SegmentId,
    /// Opened on first use, since most readers never consult statistics. A failed open is
    /// cached as `Unknown`.
    stats: OnceLock<StatsRead<SegmentStats>>,
}

/// Lazily opened statistics tied to one frozen searcher.
pub(crate) struct SegmentStatsSnapshot {
    /// The segment readers stay with the searcher; each captured segment holds only its id and
    /// its statistics cache, so capture never clones a reader.
    searcher: Searcher,
    ordinal_by_id: HashMap<SegmentId, usize>,
    segments: Box<[CapturedSegment]>,
    /// Only requested fields are decoded. Entries, including failed reads, live as long as
    /// this frozen view and are shared by readers and their range-partition derivatives.
    empirical_by_field: Mutex<HashMap<Field, Arc<[StatsRead<EmpiricalStats>]>>>,
}

impl fmt::Debug for SegmentStatsSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.segments.iter().map(|segment| segment.id))
            .finish()
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
    pub(crate) static FIELD_PROOF_PASSES: AtomicUsize = AtomicUsize::new(0);

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

fn classify<T>(value: io::Result<Option<T>>, what: impl Display) -> StatsRead<T> {
    match value {
        Ok(Some(value)) => StatsRead::Known(value),
        Ok(None) => StatsRead::Absent,
        Err(error) => {
            pgrx::debug1!("segment pruning could not read {what}: {error}");
            StatsRead::Unknown
        }
    }
}

fn capture_stats(reader: &SegmentReader) -> StatsRead<SegmentStats> {
    #[cfg(any(test, feature = "pg_test"))]
    test_support::STATS_OPENS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let opened = SegmentStats::of_reader(reader);
    #[cfg(any(test, feature = "pg_test"))]
    let opened = test_support::maybe_fail(
        reader.segment_id(),
        test_support::InjectedStatsFailure::Open,
        opened,
    );
    classify(opened, format_args!(".stats for {:?}", reader.segment_id()))
}

fn read_empirical(
    stats: &SegmentStats,
    segment_id: SegmentId,
    field: &SearchField,
) -> StatsRead<EmpiricalStats> {
    #[cfg(any(test, feature = "pg_test"))]
    test_support::EMPIRICAL_READS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let value = stats.empirical_for(field);
    #[cfg(any(test, feature = "pg_test"))]
    let value = test_support::maybe_fail(
        segment_id,
        test_support::InjectedStatsFailure::Empirical,
        value,
    );
    classify(
        value,
        format_args!("empirical field {:?} for {segment_id:?}", field.field()),
    )
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
    classify(
        value,
        format_args!("logical field {:?} for {segment_id:?}", field.field()),
    )
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
            .map(|(idx, segment)| (segment.id, idx))
            .collect::<HashMap<_, _>>();
        assert_eq!(
            ordinal_by_id.len(),
            segments.len(),
            "segment IDs in one snapshot must be unique"
        );
        Arc::new(Self {
            searcher: searcher.clone(),
            ordinal_by_id,
            segments,
            empirical_by_field: Mutex::new(HashMap::default()),
        })
    }

    pub(crate) fn len(&self) -> usize {
        self.segments.len()
    }

    pub(crate) fn segment_index(&self, segment_id: SegmentId) -> Option<usize> {
        self.ordinal_by_id.get(&segment_id).copied()
    }

    pub(super) fn empirical_for_field(
        &self,
        field: &SearchField,
    ) -> Arc<[StatsRead<EmpiricalStats>]> {
        // A panic while decoding poisons the lock; the cached entries stay valid regardless.
        let mut fields = self
            .empirical_by_field
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Arc::clone(fields.entry(field.field()).or_insert_with(|| {
            (0..self.len())
                .map(|idx| match self.stats(idx) {
                    StatsRead::Known(stats) => read_empirical(stats, self.segments[idx].id, field),
                    StatsRead::Absent => StatsRead::Absent,
                    StatsRead::Unknown => StatsRead::Unknown,
                })
                .collect()
        }))
    }

    #[cfg(any(test, feature = "pg_test"))]
    pub(crate) fn empirical(
        &self,
        segment_idx: usize,
        field: &SearchField,
    ) -> Option<EmpiricalStats> {
        self.empirical_for_field(field)[segment_idx]
            .known()
            .cloned()
    }

    fn stats(&self, ord: usize) -> &StatsRead<SegmentStats> {
        self.segments[ord]
            .stats
            .get_or_init(|| capture_stats(self.searcher.segment_reader(ord as SegmentOrdinal)))
    }

    /// Logical bounds describe where the build routed rows; empirical bounds describe what
    /// the segment contains. Both must overlap when available. An unreadable entry keeps the
    /// segment, whereas an absent entry still permits the other kind of bounds to prune it.
    fn may_intersect_partition(
        &self,
        ord: usize,
        field: &SearchField,
        includes_nulls: bool,
        (lower, upper): (&Bound<PdbOwnedValue>, &Bound<PdbOwnedValue>),
        empirical: &StatsRead<EmpiricalStats>,
    ) -> bool {
        let segment_id = self.segments[ord].id;
        let stats = match self.stats(ord) {
            StatsRead::Known(stats) => stats,
            StatsRead::Absent | StatsRead::Unknown => return true,
        };
        let empirical = match empirical {
            StatsRead::Known(value) => Some(value),
            StatsRead::Absent => None,
            StatsRead::Unknown => return true,
        };
        let logical = match read_logical(stats, segment_id, field) {
            StatsRead::Known(value) => Some(value),
            StatsRead::Absent => None,
            StatsRead::Unknown => return true,
        };
        match (logical, empirical) {
            (None, None) => true,
            (Some(bounds), None) => {
                (includes_nulls && bounds.may_hold_nulls()) || bounds.intersects(lower, upper)
            }
            (None, Some(empirical)) => {
                (includes_nulls && empirical.nullable) || empirical.intersects(lower, upper)
            }
            (Some(bounds), Some(empirical)) => {
                (includes_nulls && empirical.nullable)
                    || (bounds.intersects(lower, upper) && empirical.intersects(lower, upper))
            }
        }
    }

    pub(crate) fn segments_intersecting_partition<'a>(
        &'a self,
        field: &'a SearchField,
        range: &'a PartitionRange,
    ) -> impl Iterator<Item = SegmentId> + 'a {
        // Empty and NULL-only partitions need no statistics.
        let proof = range
            .values()
            .map(|bounds| (bounds, self.empirical_for_field(field)));
        self.segments
            .iter()
            .enumerate()
            .filter(move |(ord, _)| match &proof {
                Some((bounds, stats)) => self.may_intersect_partition(
                    *ord,
                    field,
                    range.includes_nulls(),
                    *bounds,
                    &stats[*ord],
                ),
                None => range.includes_nulls(),
            })
            .map(|(_, segment)| segment.id)
    }
}
