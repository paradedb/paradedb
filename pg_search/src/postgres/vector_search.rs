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

use std::cell::UnsafeCell;
use std::mem::{align_of, size_of};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, Ordering};
use std::time::Instant;

use pgrx::check_for_interrupts;
use tantivy::vector::{
    ClusterWork, PROBE_WAVE_SIZE, PreparedVectorSearch, ProbeBudget, ProbeStats, ProbeWave,
    RankedCluster, RoutingPhases, VectorSearchControl,
};
use tantivy::{DocAddress, Score};

use super::ParallelScanState;
use super::condition_variable::ConditionVariable;

const MAX_SHARED_TOPK: usize = 1024;

#[derive(Clone, Copy)]
#[repr(C)]
struct Candidate {
    score: f32,
    segment: u32,
    doc: u32,
}

#[derive(Clone, Copy, Default)]
#[repr(C)]
struct RouteData {
    ranked_len: usize,
    incremental: bool,
    num_centroids: usize,
    precomputed_centroids: usize,
    shareable: bool,
    routing: Option<tantivy::vector::RouterMetrics>,
    routing_time_ns: u64,
    routing_phases: RoutingPhases,
    budget: ProbeBudget,
    initial_wave: Option<ProbeWave>,
}

#[repr(C)]
pub struct ParallelVectorState {
    locked: AtomicBool,
    threshold: AtomicU64,
    heap_len: UnsafeCell<usize>,
    heap_capacity: usize,
    cluster_capacity: usize,
    route: UnsafeCell<RouteData>,
    route_status: AtomicU32,
    route_owner: AtomicU32,
    rank_owner: AtomicU32,
    ranked_len: AtomicU64,
    rank_exhausted: AtomicBool,
    rank_complete: AtomicBool,
    rank_metrics: UnsafeCell<Option<tantivy::vector::RouterMetrics>>,
    rank_phases: UnsafeCell<RoutingPhases>,
    rank_time_ns: AtomicU64,
    rank_precomputed_centroids: AtomicU64,
    capacity_opens: AtomicU64,
    capacity_rows: AtomicU64,
    leader_routes: bool,
    native_participants: AtomicU32,
    route_wait_ns: AtomicU64,
    attach_wait_ns: AtomicU64,
    initialized_at: i64,
    timeline_us: [AtomicU64; 7],
    segment_count: usize,
    counters: [AtomicU64; 19],
    round_cv: UnsafeCell<ConditionVariable>,
    arrived: AtomicU64,
    generation: AtomicU64,
    wave_end: AtomicU64,
    spent: AtomicU64,
    wave_threshold: AtomicU64,
    opens: [AtomicU64; PROBE_WAVE_SIZE],
    rows: [AtomicU64; PROBE_WAVE_SIZE],
}

impl From<&PreparedVectorSearch> for RouteData {
    fn from(plan: &PreparedVectorSearch) -> Self {
        Self {
            ranked_len: plan.clusters.len(),
            incremental: plan.incremental,
            num_centroids: plan.num_centroids,
            precomputed_centroids: plan.precomputed_centroids,
            shareable: plan.shareable,
            routing: plan.routing,
            routing_time_ns: plan.routing_time_ns,
            routing_phases: plan.routing_phases,
            budget: plan.budget,
            initial_wave: plan.initial_wave,
        }
    }
}

impl ParallelVectorState {
    pub fn size(probe_clusters: usize, _limit: usize, segment_count: usize) -> usize {
        size_of::<Self>()
            + probe_clusters * size_of::<RankedCluster>()
            + MAX_SHARED_TOPK * size_of::<Candidate>()
            + (2 * segment_count + 1) * size_of::<AtomicU32>()
            + probe_clusters
    }

    pub fn offset(base_size: usize) -> usize {
        base_size.next_multiple_of(align_of::<Self>())
    }

    /// # Safety
    /// `scan` must have space for the aligned base scan and vector payload, with no attached workers.
    pub unsafe fn initialize(
        scan: *mut ParallelScanState,
        base_size: usize,
        leader_routes: bool,
        probe_clusters: usize,
        limit: usize,
        segment_count: usize,
    ) {
        unsafe {
            let offset = Self::offset(base_size);
            (*scan).vector_offset = offset;
            let state = scan.cast::<u8>().add(offset).cast::<Self>();
            Self::initialize_at(state, leader_routes, probe_clusters, limit, segment_count);
        }
    }

    /// # Safety
    /// `state` must be aligned and have `size` bytes available with no attached readers.
    unsafe fn initialize_at(
        state: *mut Self,
        leader_routes: bool,
        probe_clusters: usize,
        limit: usize,
        segment_count: usize,
    ) {
        unsafe {
            state.write(Self {
                locked: AtomicBool::new(false),
                threshold: AtomicU64::new(0),
                heap_len: UnsafeCell::new(0),
                heap_capacity: if limit <= MAX_SHARED_TOPK { limit } else { 0 },
                cluster_capacity: probe_clusters,
                route: UnsafeCell::new(RouteData::default()),
                route_status: AtomicU32::new(0),
                route_owner: AtomicU32::new(if leader_routes {
                    pgrx::pg_sys::MyProcPid as u32
                } else {
                    0
                }),
                rank_owner: AtomicU32::new(0),
                ranked_len: AtomicU64::new(0),
                rank_exhausted: AtomicBool::new(false),
                rank_complete: AtomicBool::new(false),
                rank_metrics: UnsafeCell::new(None),
                rank_phases: UnsafeCell::new(RoutingPhases::default()),
                rank_time_ns: AtomicU64::new(0),
                rank_precomputed_centroids: AtomicU64::new(0),
                capacity_opens: AtomicU64::new(0),
                capacity_rows: AtomicU64::new(0),
                leader_routes,
                native_participants: AtomicU32::new(0),
                route_wait_ns: AtomicU64::new(0),
                attach_wait_ns: AtomicU64::new(0),
                initialized_at: pgrx::pg_sys::GetCurrentTimestamp(),
                timeline_us: std::array::from_fn(|i| {
                    AtomicU64::new(if i == 2 || i == 4 || i == 5 {
                        u64::MAX
                    } else {
                        0
                    })
                }),
                segment_count,
                counters: std::array::from_fn(|_| AtomicU64::new(0)),
                round_cv: UnsafeCell::new(ConditionVariable::new()),
                arrived: AtomicU64::new(0),
                generation: AtomicU64::new(0),
                wave_end: AtomicU64::new(0),
                spent: AtomicU64::new(0f64.to_bits()),
                wave_threshold: AtomicU64::new(0),
                opens: std::array::from_fn(|_| AtomicU64::new(0)),
                rows: std::array::from_fn(|_| AtomicU64::new(0)),
            });
            for i in 0..MAX_SHARED_TOPK {
                (*state).heap_ptr().add(i).write(Candidate {
                    score: 0.0,
                    segment: 0,
                    doc: 0,
                });
            }
            for i in 0..probe_clusters {
                (*state).flags_ptr().add(i).write(AtomicU8::new(0));
            }
            for i in 0..segment_count {
                (*state).work_ptr().add(i).write(AtomicU32::new(0));
            }
            for i in 0..=segment_count {
                (*state).pids_ptr().add(i).write(AtomicU32::new(0));
            }
        }
    }

    /// # Safety
    /// `scan` must point to initialized, live scan DSM.
    pub unsafe fn from_scan(scan: *mut ParallelScanState) -> Option<NonNull<Self>> {
        unsafe {
            let offset = (*scan).vector_offset;
            (offset != 0).then(|| NonNull::new_unchecked(scan.cast::<u8>().add(offset).cast()))
        }
    }

    fn elapsed_us(&self) -> u64 {
        (unsafe { pgrx::pg_sys::GetCurrentTimestamp() } - self.initialized_at).max(0) as u64
    }

    fn route(&self) -> &RouteData {
        assert_eq!(self.route_status.load(Ordering::Acquire), 2);
        unsafe { &*self.route.get() }
    }

    pub fn worker_attached(&self) {
        let elapsed = self.elapsed_us();
        self.timeline_us[5].fetch_min(elapsed, Ordering::Relaxed);
        self.timeline_us[6].fetch_max(elapsed, Ordering::Relaxed);
    }

    pub fn native_participants(&self) -> usize {
        self.native_participants.load(Ordering::Acquire) as usize
    }

    /// # Safety
    /// A non-null context must be this scan's live leader-local parallel context after launch.
    pub unsafe fn prepare_route(
        &self,
        context: *mut pgrx::pg_sys::ParallelContext,
        prepare: impl FnOnce() -> PreparedVectorSearch,
    ) -> PreparedVectorSearch {
        if unsafe { pgrx::pg_sys::ParallelWorkerNumber } >= 0 {
            let elapsed = self.elapsed_us();
            self.timeline_us[2].fetch_min(elapsed, Ordering::Relaxed);
            self.timeline_us[3].fetch_max(elapsed, Ordering::Relaxed);
        }
        if self.route_status.load(Ordering::Acquire) == 2 {
            return self.plan();
        }
        struct PublishGuard<'a>(&'a ParallelVectorState, bool);
        impl Drop for PublishGuard<'_> {
            fn drop(&mut self) {
                if !self.1 {
                    self.0.route_status.store(3, Ordering::Release);
                    unsafe { &mut *self.0.round_cv.get() }.broadcast();
                }
            }
        }
        if (!self.leader_routes || !context.is_null())
            && self
                .route_status
                .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
        {
            let mut guard = PublishGuard(self, false);
            self.route_owner
                .store(unsafe { pgrx::pg_sys::MyProcPid as u32 }, Ordering::Release);
            self.timeline_us[0].store(self.elapsed_us(), Ordering::Relaxed);
            if !context.is_null() {
                self.native_participants.store(
                    unsafe { (*context).nworkers_launched as u32 + 1 },
                    Ordering::Release,
                );
            }
            let plan = prepare();
            self.timeline_us[1].store(self.elapsed_us(), Ordering::Relaxed);
            assert!(plan.clusters.len() <= self.cluster_capacity);
            unsafe {
                self.ranked_ptr()
                    .copy_from_nonoverlapping(plan.clusters.as_ptr(), plan.clusters.len());
                self.route.get().write(RouteData::from(&plan));
            }
            self.route_status.store(2, Ordering::Release);
            guard.1 = true;
            unsafe { &mut *self.round_cv.get() }.broadcast();
            return self.plan();
        }
        struct CancelSleep;
        impl Drop for CancelSleep {
            fn drop(&mut self) {
                ConditionVariable::cancel_sleep();
            }
        }
        let _cancel = CancelSleep;
        let started = Instant::now();
        let cv = unsafe { &mut *self.round_cv.get() };
        loop {
            check_for_interrupts!();
            cv.prepare_to_sleep();
            match self.route_status.load(Ordering::Acquire) {
                2 => break,
                3 => pgrx::error!("vector routing failed"),
                _ => {}
            }
            if cv.sleep_for(10) {
                let owner = self.route_owner.load(Ordering::Acquire);
                if owner != 0 && unsafe { pgrx::pg_sys::BackendPidGetProc(owner as i32).is_null() }
                {
                    pgrx::error!("vector routing participant exited");
                }
            }
        }
        self.route_wait_ns
            .fetch_add(started.elapsed().as_nanos() as u64, Ordering::Relaxed);
        self.plan()
    }

    fn ranked_ptr(&self) -> *mut RankedCluster {
        unsafe {
            (self as *const Self)
                .cast::<u8>()
                .add(size_of::<Self>())
                .cast_mut()
                .cast()
        }
    }

    fn heap_ptr(&self) -> *mut Candidate {
        unsafe { self.ranked_ptr().add(self.cluster_capacity).cast() }
    }

    fn flags_ptr(&self) -> *mut AtomicU8 {
        unsafe { self.work_ptr().add(self.segment_count).cast() }
    }

    fn work_ptr(&self) -> *mut AtomicU32 {
        unsafe { self.pids_ptr().add(self.segment_count + 1) }
    }

    fn pids_ptr(&self) -> *mut AtomicU32 {
        unsafe { self.heap_ptr().add(MAX_SHARED_TOPK).cast() }
    }

    fn register_pid(&self, pid: u32) {
        for i in 0..=self.segment_count {
            if unsafe { &*self.pids_ptr().add(i) }.load(Ordering::Acquire) == pid {
                return;
            }
        }
        for i in 0..=self.segment_count {
            if unsafe { &*self.pids_ptr().add(i) }
                .compare_exchange(0, pid, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                return;
            }
        }
        pgrx::error!("too many vector probe participants");
    }

    fn unregister(&self) {
        let pid = unsafe { pgrx::pg_sys::MyProcPid as u32 };
        for i in 0..=self.segment_count {
            let _ = unsafe { &*self.pids_ptr().add(i) }.compare_exchange(
                pid,
                0,
                Ordering::AcqRel,
                Ordering::Relaxed,
            );
        }
    }

    fn arrive(&self, segments: usize, selection: Option<(usize, usize)>) -> u64 {
        let generation = self.generation.load(Ordering::Acquire);
        let arrived = self.arrived.fetch_add(segments as u64, Ordering::AcqRel) + segments as u64;
        assert!(arrived <= self.segment_count as u64);
        if arrived == self.segment_count as u64 {
            if let Some((start, len)) = selection {
                for i in 0..self.segment_count {
                    unsafe { &*self.work_ptr().add(i) }.store(0, Ordering::Relaxed);
                }
                let costs: Vec<_> = (0..len)
                    .map(|i| ClusterWork {
                        opens: self.opens[i].swap(0, Ordering::Relaxed),
                        rows: self.rows[i].swap(0, Ordering::Relaxed),
                    })
                    .collect();
                let wave = self.route().budget.select(
                    start,
                    &costs,
                    f64::from_bits(self.spent.load(Ordering::Relaxed)),
                );
                self.wave_end.store(wave.end as u64, Ordering::Relaxed);
                self.spent.store(wave.spent.to_bits(), Ordering::Relaxed);
                self.counters[16].fetch_add(1, Ordering::Relaxed);
            } else {
                self.wave_threshold
                    .store(self.threshold.load(Ordering::Relaxed), Ordering::Relaxed);
            }
            self.arrived.store(0, Ordering::Relaxed);
            self.generation.store(generation + 1, Ordering::Release);
            unsafe { &mut *self.round_cv.get() }.broadcast();
        }
        generation
    }

    pub fn plan(&self) -> PreparedVectorSearch {
        PreparedVectorSearch {
            clusters: unsafe {
                std::slice::from_raw_parts(self.ranked_ptr(), self.route().ranked_len)
            }
            .to_vec(),
            incremental: self.route().incremental,
            num_centroids: self.route().num_centroids,
            precomputed_centroids: self.route().precomputed_centroids,
            shareable: self.route().shareable,
            budget: self.route().budget,
            initial_wave: self.route().initial_wave,
            routing: None,
            routing_time_ns: 0,
            routing_phases: RoutingPhases::default(),
        }
    }

    fn publish(&self, candidates: &[(Score, DocAddress)]) -> bool {
        if self.heap_capacity == 0 {
            return true;
        }
        if self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            self.counters[13].fetch_add(1, Ordering::Relaxed);
            return false;
        }
        let started = Instant::now();
        unsafe {
            let heap = std::slice::from_raw_parts_mut(self.heap_ptr(), self.heap_capacity);
            let len = &mut *self.heap_len.get();
            for &(score, address) in candidates {
                if !score.is_finite()
                    || heap[..*len].iter().any(|entry| {
                        entry.segment == address.segment_ord && entry.doc == address.doc_id
                    })
                {
                    continue;
                }
                let candidate = Candidate {
                    score,
                    segment: address.segment_ord,
                    doc: address.doc_id,
                };
                if *len < heap.len() {
                    let mut i = *len;
                    *len += 1;
                    heap[i] = candidate;
                    while i > 0 {
                        let parent = (i - 1) / 2;
                        if heap[parent].score <= heap[i].score {
                            break;
                        }
                        heap.swap(i, parent);
                        i = parent;
                    }
                } else if score > heap[0].score {
                    heap[0] = candidate;
                    let mut i = 0;
                    loop {
                        let left = 2 * i + 1;
                        if left >= *len {
                            break;
                        }
                        let right = left + 1;
                        let child = if right < *len && heap[right].score < heap[left].score {
                            right
                        } else {
                            left
                        };
                        if heap[i].score <= heap[child].score {
                            break;
                        }
                        heap.swap(i, child);
                        i = child;
                    }
                }
            }
            if *len == heap.len() {
                self.threshold.store(
                    (1u64 << 32) | heap[0].score.to_bits() as u64,
                    Ordering::Relaxed,
                );
            }
        }
        self.locked.store(false, Ordering::Release);
        self.counters[12].fetch_add(started.elapsed().as_nanos() as u64, Ordering::Relaxed);
        true
    }

    fn finish(&self, stats: &ProbeStats) {
        if stats.routing.is_some() && self.route().incremental {
            unsafe {
                self.rank_metrics.get().write(stats.routing);
                self.rank_phases.get().write(stats.routing_phases);
            }
            self.rank_time_ns
                .store(stats.routing_time_ns, Ordering::Relaxed);
            self.rank_precomputed_centroids
                .store(stats.precomputed_centroids as u64, Ordering::Relaxed);
            self.rank_complete.store(true, Ordering::Release);
        }
        let values = [
            stats.candidates_scored as u64,
            stats.vectors_visited as u64,
            stats.pruned_filter as u64,
            stats.pruned_dead as u64,
            stats.pruned_seen as u64,
            stats.segment_opens as u64,
            stats.bounds_skips as u64,
            stats.exact_rows_read as u64,
            stats.filters_built as u64,
            stats.segments_searched as u64,
            stats.filter_time_ns,
            stats.probe_time_ns,
        ];
        for (counter, value) in self.counters.iter().zip(values) {
            counter.fetch_add(value, Ordering::Relaxed);
        }
        self.counters[14].fetch_add(stats.pruned_invisible as u64, Ordering::Relaxed);
        self.counters[15].fetch_add(stats.segment_setup_time_ns, Ordering::Relaxed);
        for (i, &flags) in stats.cluster_flags.iter().enumerate() {
            unsafe { &*self.flags_ptr().add(i) }.fetch_or(flags, Ordering::Relaxed);
        }
    }

    pub fn explain(&self) -> serde_json::Value {
        if self.route_status.load(Ordering::Acquire) != 2 {
            let mut value = serde_json::to_value(ProbeStats::default())
                .expect("vector statistics should serialize");
            value["termination"] = "NotExecuted".into();
            value["prepare_time_ns"] = 0.into();
            value["incremental_router_time_ns"] = 0.into();
            value["routing_other_time_ns"] = 0.into();
            return value;
        }
        let values: Vec<_> = self
            .counters
            .iter()
            .map(|v| v.load(Ordering::Relaxed))
            .collect();
        let mut stats = ProbeStats {
            candidates_scored: values[0] as usize,
            vectors_visited: values[1] as usize,
            pruned_filter: values[2] as usize,
            pruned_dead: values[3] as usize,
            pruned_seen: values[4] as usize,
            segment_opens: values[5] as usize,
            bounds_skips: values[6] as u32,
            exact_rows_read: values[7] as usize,
            filters_built: values[8] as u32,
            segments_searched: values[9] as u32,
            filter_time_ns: values[10],
            probe_time_ns: values[11],
            routing: self.route().routing,
            routing_time_ns: self.route().routing_time_ns,
            routing_phases: self.route().routing_phases,
            pruned_invisible: values[14] as usize,
            segment_setup_time_ns: values[15],
            ..Default::default()
        };
        let mut incremental_router_time_ns = 0;
        if self.rank_complete.load(Ordering::Acquire) {
            stats.routing = unsafe { *self.rank_metrics.get() };
            incremental_router_time_ns = self.rank_time_ns.load(Ordering::Relaxed);
            stats.routing_time_ns += incremental_router_time_ns;
            stats.routing_phases += unsafe { *self.rank_phases.get() };
        }
        let ranked_len = if self.route().incremental {
            self.ranked_len.load(Ordering::Acquire) as usize
        } else {
            self.route().ranked_len
        };
        for i in 0..ranked_len {
            let flags = unsafe { &*self.flags_ptr().add(i) }.load(Ordering::Relaxed);
            if flags & 2 != 0 {
                stats.postings_row += 1;
            } else if flags == 1 {
                stats.postings_skipped += 1;
            }
        }
        let phases = stats.routing_phases;
        let routing_other_time_ns = stats.routing_time_ns.saturating_sub(
            phases.segment_metadata_time_ns
                + phases.router_open_time_ns
                + phases.centroid_precompute_time_ns
                + phases.router_prefix_time_ns,
        );
        let mut value = serde_json::to_value(stats).expect("vector statistics should serialize");
        value["prepare_time_ns"] = self.route().routing_time_ns.into();
        value["incremental_router_time_ns"] = incremental_router_time_ns.into();
        value["routing_other_time_ns"] = routing_other_time_ns.into();
        value["heap_publish_time_ns"] = values[12].into();
        value["heap_publish_deferred"] = values[13].into();
        value["ranked_clusters"] = ranked_len.into();
        value["budgeted_prefix"] = self.route().initial_wave.is_some().into();
        value["precomputed_centroids"] = (self.route().precomputed_centroids as u64
            + self.rank_precomputed_centroids.load(Ordering::Relaxed))
        .into();
        value["shared_heap_capacity"] = self.heap_capacity.into();
        value["probe_rounds"] = values[16].into();
        value["work_claims"] = values[18].into();
        value["coordination_time_ns"] = values[17].into();
        value["native_participants"] = self.native_participants().into();
        value["route_wait_time_ns"] = self.route_wait_ns.load(Ordering::Relaxed).into();
        value["worker_attach_wait_time_ns"] = self.attach_wait_ns.load(Ordering::Relaxed).into();
        if self.native_participants() > 0 {
            for (i, name) in [
                "routing_started_us",
                "routing_finished_us",
                "first_worker_entered_us",
                "last_worker_entered_us",
                "first_wave_authorized_us",
                "first_worker_attached_us",
                "last_worker_attached_us",
            ]
            .iter()
            .enumerate()
            {
                let elapsed = self.timeline_us[i].load(Ordering::Relaxed);
                value[*name] = if elapsed == u64::MAX {
                    serde_json::Value::Null
                } else {
                    elapsed.into()
                };
            }
        }
        value["authorized_clusters"] = self.wave_end.load(Ordering::Relaxed).into();
        value["work_budget"] = self.route().budget.limit.into();
        value["work_charged"] =
            if self.route().incremental && self.route().shareable && ranked_len == 0 {
                self.route().budget.charge(ClusterWork {
                    opens: 0,
                    rows: values[0],
                })
            } else {
                f64::from_bits(self.spent.load(Ordering::Relaxed))
            }
            .into();
        value["termination"] = if (self.wave_end.load(Ordering::Relaxed) as usize) < ranked_len {
            "Ceiling"
        } else {
            "Exhausted"
        }
        .into();
        value
    }
}

pub struct PgVectorSearchControl<'a> {
    pub accept: &'a mut dyn FnMut(DocAddress) -> bool,
    pub shared: Option<NonNull<ParallelVectorState>>,
    pub parallel_context: *mut pgrx::pg_sys::ParallelContext,
}

impl PgVectorSearchControl<'_> {
    fn wait(&mut self, state: &ParallelVectorState, generation: u64) {
        if !self.parallel_context.is_null() {
            let started = Instant::now();
            unsafe { pgrx::pg_sys::WaitForParallelWorkersToAttach(self.parallel_context) };
            state
                .attach_wait_ns
                .store(started.elapsed().as_nanos() as u64, Ordering::Relaxed);
            self.parallel_context = std::ptr::null_mut();
        }
        struct CancelSleep;
        impl Drop for CancelSleep {
            fn drop(&mut self) {
                ConditionVariable::cancel_sleep();
            }
        }
        let start = Instant::now();
        let cv = unsafe { &mut *state.round_cv.get() };
        let _cancel = CancelSleep;
        while state.generation.load(Ordering::Acquire) == generation {
            self.check_interrupt();
            cv.prepare_to_sleep();
            if state.generation.load(Ordering::Acquire) != generation {
                break;
            }
            if cv.sleep_for(10) {
                for i in 0..=state.segment_count {
                    let pid = unsafe { &*state.pids_ptr().add(i) }.load(Ordering::Acquire);
                    if pid != 0 && unsafe { pgrx::pg_sys::BackendPidGetProc(pid as i32).is_null() }
                    {
                        pgrx::error!("vector probe participant exited during a round");
                    }
                }
            }
        }
        state.counters[17].fetch_add(start.elapsed().as_nanos() as u64, Ordering::Relaxed);
    }
}

impl VectorSearchControl for PgVectorSearchControl<'_> {
    fn add_capacity(&mut self, capacity: ClusterWork) {
        if let Some(state) = self.shared {
            let state = unsafe { state.as_ref() };
            state
                .capacity_opens
                .fetch_add(capacity.opens, Ordering::Relaxed);
            state
                .capacity_rows
                .fetch_add(capacity.rows, Ordering::Relaxed);
        }
    }

    fn capacity(&self) -> Option<ClusterWork> {
        self.shared.map(|state| {
            let state = unsafe { state.as_ref() };
            ClusterWork {
                opens: state.capacity_opens.load(Ordering::Relaxed),
                rows: state.capacity_rows.load(Ordering::Relaxed),
            }
        })
    }

    fn routes_clusters(&mut self) -> bool {
        let Some(state) = self.shared else {
            return true;
        };
        let state = unsafe { state.as_ref() };
        let pid = unsafe { pgrx::pg_sys::MyProcPid as u32 };
        state
            .rank_owner
            .compare_exchange(0, pid, Ordering::AcqRel, Ordering::Acquire)
            .unwrap_or_else(|owner| owner)
            == 0
    }

    fn can_overlap_routing(&self) -> bool {
        self.shared.is_some()
    }

    fn extend_clusters(
        &mut self,
        start: usize,
        clusters: &mut Vec<RankedCluster>,
        next: &mut dyn FnMut() -> Option<RankedCluster>,
    ) {
        let target = start + PROBE_WAVE_SIZE + 1;
        let Some(state) = self.shared else {
            let remaining = target.saturating_sub(clusters.len());
            clusters.extend(std::iter::from_fn(next).take(remaining));
            return;
        };
        let state = unsafe { state.as_ref() };
        if state.rank_owner.load(Ordering::Acquire) == unsafe { pgrx::pg_sys::MyProcPid as u32 } {
            struct PublishGuard<'a>(&'a ParallelVectorState, bool);
            impl Drop for PublishGuard<'_> {
                fn drop(&mut self) {
                    if !self.1 {
                        self.0.route_status.store(3, Ordering::Release);
                        unsafe { &mut *self.0.round_cv.get() }.broadcast();
                    }
                }
            }
            let mut guard = PublishGuard(state, false);
            let previous = clusters.len();
            clusters.extend(std::iter::from_fn(next).take(target.saturating_sub(previous)));
            assert!(clusters.len() <= state.cluster_capacity);
            unsafe {
                state.ranked_ptr().add(previous).copy_from_nonoverlapping(
                    clusters.as_ptr().add(previous),
                    clusters.len() - previous,
                );
            }
            state
                .ranked_len
                .store(clusters.len() as u64, Ordering::Release);
            if clusters.len() < target {
                state.rank_exhausted.store(true, Ordering::Release);
            }
            state.timeline_us[1].store(state.elapsed_us(), Ordering::Relaxed);
            guard.1 = true;
            unsafe { &mut *state.round_cv.get() }.broadcast();
            return;
        }
        struct CancelSleep;
        impl Drop for CancelSleep {
            fn drop(&mut self) {
                ConditionVariable::cancel_sleep();
            }
        }
        let _cancel = CancelSleep;
        let started = Instant::now();
        let cv = unsafe { &mut *state.round_cv.get() };
        let end = loop {
            self.check_interrupt();
            cv.prepare_to_sleep();
            if state.route_status.load(Ordering::Acquire) == 3 {
                pgrx::error!("vector routing failed");
            }
            let exhausted = state.rank_exhausted.load(Ordering::Acquire);
            let end = state.ranked_len.load(Ordering::Acquire) as usize;
            if end >= target || exhausted {
                break end;
            }
            if cv.sleep_for(10) {
                let owner = state.rank_owner.load(Ordering::Acquire);
                if owner != 0 && unsafe { pgrx::pg_sys::BackendPidGetProc(owner as i32).is_null() }
                {
                    pgrx::error!("vector routing participant exited");
                }
            }
        };
        state
            .route_wait_ns
            .fetch_add(started.elapsed().as_nanos() as u64, Ordering::Relaxed);
        clusters.extend_from_slice(unsafe {
            std::slice::from_raw_parts(state.ranked_ptr().add(clusters.len()), end - clusters.len())
        });
    }

    fn work_sharing(&self) -> bool {
        self.shared.is_some()
    }

    fn claim_work(&mut self, segment: u32) -> usize {
        let state = unsafe { self.shared.unwrap().as_ref() };
        state.counters[18].fetch_add(1, Ordering::Relaxed);
        assert!((segment as usize) < state.segment_count);
        unsafe { &*state.work_ptr().add(segment as usize) }.fetch_add(1, Ordering::Relaxed) as usize
    }

    fn publish_initial_wave(&mut self, wave: ProbeWave) {
        let state = unsafe { self.shared.unwrap().as_ref() };
        if state
            .wave_end
            .compare_exchange(0, wave.end as u64, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            state.spent.store(wave.spent.to_bits(), Ordering::Relaxed);
            state.counters[16].fetch_add(1, Ordering::Relaxed);
            state.timeline_us[4].fetch_min(state.elapsed_us(), Ordering::Relaxed);
        }
    }

    fn begin(&mut self) {
        if let Some(state) = self.shared {
            let state = unsafe { state.as_ref() };
            state.register_pid(unsafe { pgrx::pg_sys::MyProcPid as u32 });
        }
    }

    fn end(&mut self) {
        if let Some(state) = self.shared {
            unsafe { state.as_ref() }.unregister();
        }
    }

    fn select_wave(
        &mut self,
        segments: usize,
        start: usize,
        costs: &[ClusterWork],
        budget: ProbeBudget,
        spent: f64,
    ) -> ProbeWave {
        let Some(state) = self.shared else {
            return budget.select(start, costs, spent);
        };
        let state = unsafe { state.as_ref() };
        assert!(costs.len() <= PROBE_WAVE_SIZE);
        for (i, cost) in costs.iter().enumerate() {
            state.opens[i].fetch_add(cost.opens, Ordering::Relaxed);
            state.rows[i].fetch_add(cost.rows, Ordering::Relaxed);
        }
        let generation = state.arrive(segments, Some((start, costs.len())));
        self.wait(state, generation);
        state.timeline_us[4].fetch_min(state.elapsed_us(), Ordering::Relaxed);
        ProbeWave {
            end: state.wave_end.load(Ordering::Relaxed) as usize,
            spent: f64::from_bits(state.spent.load(Ordering::Relaxed)),
        }
    }

    fn synchronize(&mut self, segments: usize, local: Option<Score>) -> Option<Score> {
        let Some(state) = self.shared else {
            return local;
        };
        let state = unsafe { state.as_ref() };
        let generation = state.arrive(segments, None);
        self.wait(state, generation);
        let packed = state.wave_threshold.load(Ordering::Relaxed);
        (packed != 0).then(|| f32::from_bits(packed as u32))
    }

    fn accept(&mut self, doc: DocAddress) -> bool {
        (self.accept)(doc)
    }

    fn publish(&mut self, candidates: &[(Score, DocAddress)]) {
        if let Some(state) = self.shared {
            let state = unsafe { state.as_ref() };
            while !state.publish(candidates) {
                self.check_interrupt();
                for i in 0..=state.segment_count {
                    let pid = unsafe { &*state.pids_ptr().add(i) }.load(Ordering::Acquire);
                    if pid != 0 && unsafe { pgrx::pg_sys::BackendPidGetProc(pid as i32).is_null() }
                    {
                        pgrx::error!("vector probe participant exited while publishing results");
                    }
                }
                std::hint::spin_loop();
            }
        }
    }

    fn check_interrupt(&mut self) {
        check_for_interrupts!();
    }

    fn finish(&mut self, stats: &ProbeStats) {
        if let Some(state) = self.shared {
            unsafe { state.as_ref() }.finish(stats);
        }
    }
}
