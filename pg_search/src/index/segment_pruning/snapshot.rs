// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

use std::ops::Bound;
use std::sync::{Arc, OnceLock};

use crate::api::HashMap;
use crate::index::mvcc::MVCCDirectory;
use crate::index::stats::{EmpiricalStats, SegmentStats};
use crate::postgres::pdb_owned_value::PdbOwnedValue;
use crate::postgres::types::is_datetime_type;
use crate::schema::{SearchField, SearchFieldType};
use tantivy::index::{Segment, SegmentId};

#[derive(Debug)]
struct CapturedSegment {
    segment: Segment,
    has_stats_component: bool,
    /// Opened on first use. Most readers never consult statistics, and opening `.stats` costs
    /// buffer reads per segment, so a reader open must not pay for it.
    stats: OnceLock<Option<SegmentStats>>,
}

impl CapturedSegment {
    fn id(&self) -> SegmentId {
        self.segment.id()
    }

    fn doc_count(&self) -> u32 {
        self.segment.meta().num_docs()
    }

    fn stats(&self) -> Option<&SegmentStats> {
        self.stats
            .get_or_init(|| capture_stats(&self.segment, self.has_stats_component))
            .as_ref()
    }
}

/// Statistics captured from exactly one Searcher/manifest view.
#[derive(Debug)]
pub(crate) struct SegmentStatsSnapshot {
    ordinal_by_id: HashMap<SegmentId, usize>,
    segments: Box<[CapturedSegment]>,
}

#[cfg(any(test, feature = "pg_test"))]
pub(crate) mod test_support {
    use super::SegmentId;
    use crate::api::HashMap;
    use std::sync::atomic::AtomicUsize;
    use std::sync::{LazyLock, Mutex};

    /// Number of `.stats` components actually opened. Tests use it to prove that readers
    /// whose proofs never consult statistics perform zero opens.
    pub(crate) static STATS_OPENS: AtomicUsize = AtomicUsize::new(0);

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub(crate) enum InjectedStatsFailure {
        Open,
        Read,
    }

    static FAILURES: LazyLock<Mutex<HashMap<SegmentId, InjectedStatsFailure>>> =
        LazyLock::new(|| Mutex::new(HashMap::default()));

    pub(crate) struct InjectedStatsFailureGuard {
        segment_id: SegmentId,
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
        failure: InjectedStatsFailure,
    ) -> InjectedStatsFailureGuard {
        let replaced = FAILURES
            .lock()
            .expect("injected statistics failure lock poisoned")
            .insert(segment_id, failure);
        assert!(
            replaced.is_none(),
            "one failure may be injected per segment"
        );
        InjectedStatsFailureGuard { segment_id }
    }

    pub(super) fn injected_stats_failure(segment_id: SegmentId) -> Option<InjectedStatsFailure> {
        FAILURES
            .lock()
            .expect("injected statistics failure lock poisoned")
            .get(&segment_id)
            .copied()
    }
}

fn capture_stats(segment: &Segment, has_stats_component: bool) -> Option<SegmentStats> {
    if !has_stats_component {
        return None;
    }
    #[cfg(any(test, feature = "pg_test"))]
    if matches!(
        test_support::injected_stats_failure(segment.id()),
        Some(test_support::InjectedStatsFailure::Open)
    ) {
        return None;
    }
    #[cfg(any(test, feature = "pg_test"))]
    test_support::STATS_OPENS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    match SegmentStats::of_segment(segment) {
        Ok(stats) => stats,
        Err(error) => {
            pgrx::debug1!(
                "segment pruning could not open .stats for {:?}: {error}",
                segment.id()
            );
            None
        }
    }
}

impl SegmentStatsSnapshot {
    /// Capture lightweight segment handles belonging to `directory`. Manifest metadata decides
    /// whether `.stats` may be opened later, because opening any component path on a mutable
    /// segment would first materialize that entire segment even though mutable segments cannot
    /// have a persisted statistics component.
    pub(crate) fn capture_segments(directory: &MVCCDirectory, segments: &[Segment]) -> Arc<Self> {
        Self::capture_with_presence(segments, |segment_id| {
            directory.has_stats_component(&segment_id)
        })
    }

    #[cfg(test)]
    pub(crate) fn capture_test_segments(segments: &[Segment]) -> Arc<Self> {
        // Ordinary Tantivy test indexes do not have ParadeDB manifest entries. Their segment
        // directories can be queried directly without the mutable-segment materialization path.
        Self::capture_with_presence(segments, |_| true)
    }

    fn capture_with_presence(
        segments: &[Segment],
        has_stats_component: impl Fn(SegmentId) -> bool,
    ) -> Arc<Self> {
        let captured = segments
            .iter()
            .map(|segment| CapturedSegment {
                segment: segment.clone(),
                has_stats_component: has_stats_component(segment.id()),
                stats: OnceLock::new(),
            })
            .collect::<Box<[_]>>();
        let ordinal_by_id = captured
            .iter()
            .enumerate()
            .map(|(idx, segment)| (segment.id(), idx))
            .collect::<HashMap<_, _>>();
        assert_eq!(
            ordinal_by_id.len(),
            captured.len(),
            "segment IDs in one snapshot must be unique"
        );

        Arc::new(Self {
            ordinal_by_id,
            segments: captured,
        })
    }

    pub(crate) fn len(&self) -> usize {
        self.segments.len()
    }

    pub(crate) fn segment_ids(&self) -> impl ExactSizeIterator<Item = SegmentId> + '_ {
        self.segments.iter().map(CapturedSegment::id)
    }

    pub(crate) fn doc_count(&self, segment_idx: usize) -> u32 {
        self.segments[segment_idx].doc_count()
    }

    pub(crate) fn segment_index(&self, segment_id: SegmentId) -> Option<usize> {
        self.ordinal_by_id.get(&segment_id).copied()
    }

    /// `Ok(None)` when the segment has no statistics for `field`; `Err` when statistics exist
    /// but could not be read, which callers must treat as unknown rather than absent.
    fn read_empirical(
        &self,
        segment_idx: usize,
        field: &SearchField,
    ) -> Result<Option<EmpiricalStats>, ()> {
        let segment = &self.segments[segment_idx];
        #[cfg(any(test, feature = "pg_test"))]
        if matches!(
            test_support::injected_stats_failure(segment.id()),
            Some(test_support::InjectedStatsFailure::Read)
        ) {
            return Err(());
        }
        let Some(stats) = segment.stats() else {
            return Ok(None);
        };
        let value = stats.empirical(field.field()).map_err(|error| {
            pgrx::debug1!(
                "segment pruning could not read field {:?} from .stats for {:?}: {error}",
                field.field(),
                segment.id()
            );
        })?;
        match (field.field_type(), value) {
            (SearchFieldType::I64(oid), Some(value)) if is_datetime_type(oid) => {
                value.into_dates().map(Some).ok_or(())
            }
            (_, value) => Ok(value),
        }
    }

    pub(crate) fn empirical(
        &self,
        segment_idx: usize,
        field: &SearchField,
    ) -> Option<EmpiricalStats> {
        self.read_empirical(segment_idx, field).ok().flatten()
    }

    /// Whether this execution segment may contain a row assigned to one range partition.
    ///
    /// Both partition routing and predicate proofs read through this snapshot so they share the
    /// exact execution manifest, the mutable-segment guard, date conversion, and fail-open error
    /// policy. Persisted logical bounds describe where the build routed rows; empirical bounds
    /// describe what the segment currently contains. When both exist, both must overlap.
    pub(crate) fn may_intersect_partition(
        &self,
        segment_idx: usize,
        field: &SearchField,
        lower: &Bound<PdbOwnedValue>,
        upper: &Bound<PdbOwnedValue>,
        includes_nulls: bool,
    ) -> bool {
        let Ok(empirical) = self.read_empirical(segment_idx, field) else {
            return true;
        };
        let segment = &self.segments[segment_idx];
        let Some(stats) = segment.stats() else {
            return true;
        };
        let logical = match stats.logical(field.field()) {
            Ok(value) => value,
            Err(error) => {
                pgrx::debug1!(
                    "segment partitioning could not read logical field {:?} from .stats for {:?}: {error}",
                    field.field(),
                    segment.id()
                );
                return true;
            }
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
}
