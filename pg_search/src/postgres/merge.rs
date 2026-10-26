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

use crate::api::HashSet;
use crate::index::merge_policy::{LayeredMergeCandidate, LayeredMergePolicy};
use crate::index::mvcc::MvccSatisfies;
use crate::index::writer::index::{Mergeable, SearchIndexMerger};
use crate::postgres::PgSearchRelation;
use crate::postgres::delete::VacuumSignal;
use crate::postgres::locks::AdvisoryLock;
use crate::postgres::ps_status::{MERGING, set_ps_display_suffix};
use crate::postgres::storage::block::{MVCCEntry, SegmentMetaEntry};
use crate::postgres::storage::buffer::BufferManager;
use crate::postgres::storage::fsm::FreeSpaceManager;
use crate::postgres::storage::merge::MergeLock;
use crate::postgres::storage::metadata::{MergePins, MetaPage};

use pgrx::bgworkers::*;
use pgrx::pg_sys::panic::CaughtError;
use pgrx::{FromDatum, IntoDatum, PgTryBuilder, check_for_interrupts, pg_guard, pg_sys};
use std::cell::RefCell;
use std::ffi::CStr;
use std::panic::AssertUnwindSafe;
use tantivy::index::SegmentMeta;
use tantivy::indexer::MergeCandidate;

const BACKPRESSURE_MULTIPLIER: usize = 4;

thread_local! {
    static WARNED_VECTOR_INDEXES: RefCell<HashSet<pg_sys::Oid>> = RefCell::new(HashSet::default());
}

/// Warns once per backend and index when merge selection drops unsupported vector storage.
fn warn_unsupported_vector_merge(index: &PgSearchRelation, skipped: usize) {
    if skipped > 0 && WARNED_VECTOR_INDEXES.with(|warned| warned.borrow_mut().insert(index.oid())) {
        pgrx::warning!(
            "index {:?}: skipped {skipped} segment(s) with unsupported vector storage during merge; rebuild with REINDEX",
            index.name()
        );
    }
}

/// The DST details payload for a background-merge worker crash, or `None` when `caught` is not a
/// bug. Returns `Some` only for bug-class errors — internal-error / corruption SQLSTATEs and Rust
/// panics. An interrupt-driven cancellation is not a bug and never reaches here: `merge_index`
/// downgrades it to a `warning!`, so the faults we deliberately inject do not trip the assertion.
fn merge_crash_details(caught: &CaughtError) -> Option<serde_json::Value> {
    use pgrx::PgSqlErrorCode::*;

    let report = match caught {
        // A Rust panic (a failed `expect`, or the `panic!("failed to merge…")` in `merge_index`)
        // is always a bug.
        CaughtError::RustPanic { ereport, .. } => ereport,
        // A Postgres/ereport error is a bug only when it is an internal error or corruption;
        // cancellations, shutdowns and connection faults are the chaos we are injecting.
        CaughtError::PostgresError(report) | CaughtError::ErrorReport(report) => {
            if !matches!(
                report.sql_error_code(),
                ERRCODE_INTERNAL_ERROR
                    | ERRCODE_DATA_CORRUPTED
                    | ERRCODE_INDEX_CORRUPTED
                    | ERRCODE_ASSERT_FAILURE
            ) {
                return None;
            }
            report
        }
    };

    Some(serde_json::json!({
        "sqlstate": format!("{:?}", report.sql_error_code()),
        "message": report.message(),
    }))
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum MergeStyle {
    /// An INSERT created a segment. A foreground merge runs inside that statement's transaction
    /// and publishes under these ids.
    Insert {
        current_xid: pg_sys::FullTransactionId,
        next_xid: pg_sys::FullTransactionId,
    },
    /// VACUUM finished; only a background merge may follow.
    Vacuum,
}

/// Who runs a merge, which decides what happens between candidates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MergeMode {
    /// Runs inside the statement that triggered it: ignores the VACUUM signal and returns blocks
    /// to the FSM once, after the last candidate.
    Foreground,
    /// Runs in a background worker: yields to a waiting VACUUM between candidates and returns
    /// blocks to the FSM after each one.
    Background,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq)]
struct BackgroundMergeArgs {
    index_oid: pg_sys::Oid,
    slot_variant: MergeSlotVariant,
}

impl BackgroundMergeArgs {
    pub fn new(index_oid: pg_sys::Oid, slot_variant: MergeSlotVariant) -> Self {
        Self {
            index_oid,
            slot_variant,
        }
    }

    pub fn index_oid(&self) -> pg_sys::Oid {
        self.index_oid
    }

    pub fn slot_variant(&self) -> MergeSlotVariant {
        self.slot_variant
    }
}

impl IntoDatum for BackgroundMergeArgs {
    fn into_datum(self) -> Option<pg_sys::Datum> {
        let upper = u32::from(self.index_oid) as u64; // top 32 bits
        let lower = self.slot_variant as u64; // bottom 32 bits
        let raw: u64 = (upper << 32) | (lower & 0xFFFF_FFFF);
        Some(pg_sys::Datum::from(raw as i64))
    }

    fn type_oid() -> pg_sys::Oid {
        pg_sys::INT8OID
    }
}

impl FromDatum for BackgroundMergeArgs {
    unsafe fn from_polymorphic_datum(
        datum: pg_sys::Datum,
        is_null: bool,
        typoid: pg_sys::Oid,
    ) -> Option<Self> {
        if is_null {
            return None;
        }

        let raw = i64::from_polymorphic_datum(datum, is_null, typoid).unwrap() as u64;
        let index_oid = ((raw >> 32) & 0xFFFF_FFFF) as std::os::raw::c_uint;
        let slot_variant = (raw & 0xFFFF_FFFF) as u8;

        Some(Self {
            index_oid: index_oid.into(),
            slot_variant: slot_variant.try_into().unwrap(),
        })
    }
}

#[derive(Debug, Clone)]
struct IndexLayerSizes {
    user_configured_bg_layers: bool,
    foreground_layer_sizes: Vec<u64>,
    background_layer_sizes: Vec<u64>,
    backpressure_layer_size: Option<u64>,
}

impl From<&PgSearchRelation> for IndexLayerSizes {
    fn from(index: &PgSearchRelation) -> Self {
        let index_options = index.options();

        let mut segment_cnt = 0;
        let mut index_byte_size = 0;
        unsafe {
            MetaPage::open(index).segment_metas().for_each(|_, entry| {
                if entry.visible() {
                    index_byte_size += entry.byte_size();
                    segment_cnt += 1;
                }
            });
        }

        Self::new(
            index_options.foreground_layer_sizes(),
            index_options.background_layer_sizes(),
            segment_cnt,
            index_byte_size,
            index_options.target_segment_count(),
        )
    }
}

impl IndexLayerSizes {
    fn new(
        foreground_layer_sizes: Vec<u64>,
        mut background_layer_sizes: Vec<u64>,
        segment_cnt: usize,
        index_byte_size: u64,
        target_segment_count: usize,
    ) -> Self {
        let mut target_segment_byte_size = index_byte_size / target_segment_count as u64;

        // reduce by a third, which is what the LayeredMergePolicy does
        //
        // this is probably a terrible place to sneak in this adjustment but it's super important
        // as we inject this into the background layer sizes as a final layer and we don't want it to
        // be big enough that segments merge into it unnecessarily.  LayeredMergePolicy assumes that
        // a merged segment will be a third smaller, and that's what we account for here
        target_segment_byte_size -= target_segment_byte_size / 3;

        // clamp the highest layer size to be less than `target_segment_byte_size`:
        //
        // `index_byte_size` / `target_segment_count`
        //
        // i.e. how big should each segment be if we want to have exactly `target_segment_count` segments?
        //
        // for instance, imagine:
        // - layer sizes: [1mb, 10mb, 100mb]
        // - index size: 200mb
        // - target segment count: 10
        //
        // then our recomputed layer sizes would be:
        // - [1mb, 10mb]
        //
        // why? the 100mb layer gets excluded because the target segment size is 20mb

        let has_bg_layers = !background_layer_sizes.is_empty();

        if segment_cnt <= target_segment_count {
            return Self {
                user_configured_bg_layers: has_bg_layers,
                foreground_layer_sizes: Vec::new(),
                background_layer_sizes: Vec::new(),
                backpressure_layer_size: None,
            };
        }
        let mut backpressure_layer_size = None;
        if !background_layer_sizes.is_empty() {
            let has_larger_layer = background_layer_sizes
                .iter()
                .any(|&layer_size| layer_size > target_segment_byte_size);
            // additionally, ensure that the background layer sizes are <= to the target segment size
            background_layer_sizes.retain(|&layer_size| layer_size <= target_segment_byte_size);

            // Pushing the target size unconditionally makes a large, unbalanced index merge itself
            // down in the background.  Only step in when the clamp left nothing to merge into; the
            // target size is then below the smallest configured layer.  This happens to small
            // indexes on hosts with many cores, where the default `target_segment_count` is large.
            if background_layer_sizes.is_empty() {
                background_layer_sizes.push(target_segment_byte_size);
            } else if has_larger_layer
                && segment_cnt > BACKPRESSURE_MULTIPLIER * target_segment_count
            {
                backpressure_layer_size = Some(target_segment_byte_size);
            }
        }

        // NB:  it's possible a user could configure "layer_sizes = '10TB'" or something ridiculous
        //      and that would cause us to merge the entire index into a single segment
        //      uncommenting this would prevent that, but light up some unit tests
        // ensure the foreground layer sizes are <= to the target segment size
        // foreground_layer_sizes.retain(|&layer_size| layer_size <= target_segment_byte_size);

        Self {
            user_configured_bg_layers: has_bg_layers,
            foreground_layer_sizes,
            background_layer_sizes,
            backpressure_layer_size,
        }
    }

    fn user_configured_background_layers(&self) -> bool {
        self.user_configured_bg_layers
    }

    fn combined(&self) -> Vec<u64> {
        let mut combined = self.foreground_layer_sizes.clone();
        combined.extend_from_slice(&self.background_layer_sizes);
        combined.sort_unstable();
        combined.dedup();
        combined
    }
}

#[inline]
fn backpressure(
    candidates: &[LayeredMergeCandidate],
    target_segment_count: usize,
) -> Option<&MergeCandidate> {
    candidates
        .chunk_by(|(left, _), (right, _)| left == right)
        .filter(|layer| {
            // Layer 0 converts mutable segments, so apply backpressure above two eligible inputs.
            // Immutable layers allow a backlog proportional to the target segment count.
            let threshold = if layer[0].0 == 0 {
                2
            } else {
                BACKPRESSURE_MULTIPLIER * target_segment_count
            };
            layer
                .iter()
                .map(|(_, candidate)| candidate.0.len())
                .sum::<usize>()
                > threshold
        })
        .min_by_key(|layer| layer[0].0)
        .map(|layer| &layer[0].1)
}

/// Kick off a merge of the index, if needed.
///
/// Prefer a background worker; help with one small merge when workers are busy or unavailable.
pub unsafe fn do_merge(index: &PgSearchRelation, style: MergeStyle) -> anyhow::Result<()> {
    // don't kick off merges for invalid indexes
    if !index.is_valid() {
        return Ok(());
    }

    let layer_sizes = IndexLayerSizes::from(index);
    let metadata = MetaPage::open(index);
    let merge_pins = metadata.pin_merge();
    let merge_lock = metadata.acquire_merge_lock();
    let merger = SearchIndexMerger::open(index, MvccSatisfies::Mergeable)?;
    let mut merge_policy = LayeredMergePolicy::new(layer_sizes.combined());
    #[cfg(feature = "pg_test")]
    crate::postgres::storage::coordination_test::selection_pause();
    merge_policy.set_mergeable_segment_entries(&metadata, &merge_lock, &merger);
    let (candidates, largest_layer_size, skipped) = merge_policy.simulate_supported(&merger)?;
    warn_unsupported_vector_merge(index, skipped);
    let needs_background_merge =
        layer_sizes.user_configured_background_layers() && !candidates.is_empty();

    if needs_background_merge && try_launch_background_merger(index, largest_layer_size) {
        return Ok(());
    }
    if let MergeStyle::Insert {
        current_xid,
        next_xid,
    } = style
    {
        let foreground_candidate = candidates
            .iter()
            .filter(|(layer, _)| layer_sizes.foreground_layer_sizes.contains(layer))
            .min_by_key(|(layer, _)| *layer)
            .map(|(_, candidate)| candidate);
        merge_policy.prepare_backpressure(layer_sizes.backpressure_layer_size);
        let (backpressure_candidates, _, skipped) = merge_policy.simulate_supported(&merger)?;
        warn_unsupported_vector_merge(index, skipped);
        let candidate = backpressure(
            &backpressure_candidates,
            index.options().target_segment_count(),
        )
        .or(foreground_candidate);
        if let Some(candidate) = candidate {
            merge_index(
                index,
                merger,
                std::slice::from_ref(candidate),
                merge_lock,
                merge_pins,
                MergeMode::Foreground,
                current_xid,
                next_xid,
            );
        }
    }
    Ok(())
}

/// Try to launch a background process to merge down the index.
/// Is not guaranteed to launch the process if there are not enough `max_worker_processes` available.
unsafe fn try_launch_background_merger(index: &PgSearchRelation, largest_layer_size: u64) -> bool {
    let slot = MergeSlot::for_layer_size(index.oid(), largest_layer_size);
    if !slot.is_available() {
        return false;
    }

    let dbname = CStr::from_ptr(pg_sys::get_database_name(pg_sys::MyDatabaseId))
        .to_string_lossy()
        .into_owned();

    let worker_name = format!(
        "background merger for {}.{}",
        index.namespace(),
        index.name()
    );

    if BackgroundWorkerBuilder::new(&worker_name)
        .enable_spi_access()
        .enable_shmem_access(None)
        .set_library("pg_search")
        .set_function("background_merge")
        .set_argument(BackgroundMergeArgs::new(index.oid(), slot.variant()).into_datum())
        .set_extra(&dbname)
        .load_dynamic()
        .is_err()
    {
        pgrx::log!("not enough available `max_worker_processes` to launch a background merger");
        false
    } else {
        true
    }
}

/// Actually do the merge
/// This function is called by the background worker.
#[pg_guard]
#[unsafe(no_mangle)]
unsafe extern "C-unwind" fn background_merge(arg: pg_sys::Datum) {
    // Bind the PostgreSQL symbol directly: pg_sys::die is a Rust FFI wrapper, not a
    // C signal callback, and its FFI guard must not run inside a signal handler.
    unsafe extern "C-unwind" {
        #[link_name = "die"]
        fn merge_worker_sigterm(signal: i32);
    }

    // PostgreSQL's waits and interrupt checks act on ProcDiePending. pgrx's SIGTERM handler
    // only sets a flag for a worker's own polling loop, and this one-shot worker has none.
    #[cfg(any(feature = "pg15", feature = "pg16", feature = "pg17"))]
    pg_sys::pqsignal(pg_sys::SIGTERM as i32, Some(merge_worker_sigterm));
    // Not gated on pg18: a future major whose `die` signature changes must fail to build here
    // rather than silently keep PostgreSQL's generic bgworker_die.
    #[cfg(not(any(feature = "pg15", feature = "pg16", feature = "pg17")))]
    pg_sys::pqsignal_be(pg_sys::SIGTERM as i32, Some(merge_worker_sigterm));
    // attach_signal_handlers unblocks signals, so install SIGTERM first.
    BackgroundWorker::attach_signal_handlers(SignalWakeFlags::SIGHUP);
    BackgroundWorker::connect_worker_to_spi(Some(BackgroundWorker::get_extra()), None);
    // A merge changes nothing a query can see, so its commit needs no standby acknowledgement.
    // Commit locally, as AutoVacWorkerMain does, so a missing synchronous standby cannot park
    // this worker at commit holding its merge slot. Only ever lowered, never raised from `off`.
    if pg_sys::synchronous_commit > pg_sys::SyncCommitLevel::SYNCHRONOUS_COMMIT_LOCAL_FLUSH as i32 {
        pg_sys::SetConfigOption(
            c"synchronous_commit".as_ptr(),
            c"local".as_ptr(),
            pg_sys::GucContext::PGC_SUSET,
            pg_sys::GucSource::PGC_S_OVERRIDE,
        );
    }
    BackgroundWorker::transaction(|| {
        set_ps_display_suffix(MERGING.as_ptr());
        pg_sys::pgstat_report_activity(pg_sys::BackendState::STATE_RUNNING, MERGING.as_ptr());

        pgrx::debug1!(
            "{}: starting background merge",
            BackgroundWorker::get_name()
        );

        let current_xid = pg_sys::GetCurrentFullTransactionId();
        let next_xid = current_xid;
        let args = BackgroundMergeArgs::from_datum(arg, false).unwrap();
        let index = PgSearchRelation::try_open(
            args.index_oid(),
            pg_sys::AccessShareLock as pg_sys::LOCKMODE,
        );
        if index.is_none() {
            pgrx::debug1!(
                "{}: index not found, suggesting it was just dropped",
                BackgroundWorker::get_name()
            );
            return;
        }
        let index = index.unwrap();
        // we allow up to 2 mergers per index: one for "small" layers and one for "large" layers
        // this checks to see if a merger is already running for the given layer
        let merge_slot = MergeSlot::new(index.oid(), args.slot_variant()).lock();
        // todo: this could potentially wait instead of returning immediately if the lock is not available
        // to avoid racing with other backends trying to probe the slot
        if merge_slot.is_none() {
            return;
        }
        let metadata = MetaPage::open(&index);

        let layer_sizes = IndexLayerSizes::from(&index);
        let mut merge_policy = LayeredMergePolicy::new(layer_sizes.combined());

        let merge_pins = metadata.pin_merge();
        // this ensures there's only one merge running at a time for the given index,
        // while the lock is held
        let merge_lock = metadata.acquire_merge_lock();

        PgTryBuilder::new(AssertUnwindSafe(|| {
            let merger = SearchIndexMerger::open(&index, MvccSatisfies::Mergeable)
                .expect("should be able to open merger");
            #[cfg(feature = "pg_test")]
            crate::postgres::storage::coordination_test::selection_pause();
            merge_policy.set_mergeable_segment_entries(&metadata, &merge_lock, &merger);
            let (candidates, _, skipped) = merge_policy
                .simulate_supported(&merger)
                .expect("should be able to validate merge candidates");
            warn_unsupported_vector_merge(&index, skipped);
            let candidates = candidates
                .into_iter()
                .map(|(_, candidate)| candidate)
                .collect::<Vec<_>>();
            merge_index(
                &index,
                merger,
                &candidates,
                merge_lock,
                merge_pins,
                MergeMode::Background,
                current_xid,
                next_xid,
            )
        }))
        // A crash here dies in this background worker: it never reaches a client
        // connection, and Postgres logs only to the container's stdout, so nothing
        // outside would fail on it. Report it to the DST harness before rethrowing so the
        // fuzzer's run fails instead of silently passing. Rethrowing preserves the
        // existing behaviour (Postgres logs the error and the worker exits).
        .catch_others(|caught| {
            dst::observe!(|| {
                if let Some(details) = merge_crash_details(&caught) {
                    dst::assert_unreachable!("pg_search background merge crashed", &details);
                }
            });
            caught.rethrow()
        })
        .execute();
    })
}

#[inline]
#[allow(clippy::too_many_arguments)]
pub(crate) unsafe fn merge_index(
    indexrel: &PgSearchRelation,
    merger: SearchIndexMerger,
    merge_candidates: &[MergeCandidate],
    merge_lock: MergeLock,
    merge_pins: MergePins,
    mode: MergeMode,
    current_xid: pg_sys::FullTransactionId,
    next_xid: pg_sys::FullTransactionId,
) {
    // Hold both pins until the output is published and the reservation removed. ambulkdelete must
    // see the final merged segment, not the originals about to be deleted, and must not deactivate
    // its exclusion list while this merger still works from an older metadata view.
    let metadata = MetaPage::open(indexrel);
    // before we start merging, tell the merger to release pins on the segments it won't be merging
    let mut merger = merger
        .adjust_pins(merge_candidates.iter().flat_map(|candidate| &candidate.0))
        .expect("should be able to adjust merger pins");

    let ncandidates = merge_candidates.len();
    if ncandidates > 0 {
        // record all the segments the SearchIndexMerger can see, as those are the ones that
        // could be merged
        let merge_entry = merge_lock
            .merge_list()
            .add_segment_ids(
                merge_candidates.iter().flat_map(|candidate| &candidate.0),
                current_xid,
            )
            .expect("should be able to write current merge segment_id list");
        drop(merge_lock);

        #[cfg(feature = "pg_test")]
        crate::postgres::storage::coordination_test::merge_pause();

        // we are NOT under the MergeLock at this point, which allows concurrent backends to also merge
        //
        // we defer raising a panic in the face of a merge error as we need to remove the created
        // `merge_entry` whether the merge worked or not (see
        // https://github.com/paradedb/paradedb/issues/6962)

        let mut merge_result: anyhow::Result<Option<SegmentMeta>> = Ok(None);

        for candidate in merge_candidates {
            if mode == MergeMode::Background && VacuumSignal::new(indexrel.oid()).wants_cancel() {
                pgrx::debug1!("VACUUM waiting, exiting merge early");
                break;
            }

            pgrx::debug1!("merging candidate with {} segments", candidate.0.len());

            merge_result = merger.merge_segments(&candidate.0);
            if merge_result.is_err() {
                break;
            }
            if mode == MergeMode::Background {
                garbage_collect_index(indexrel, current_xid, next_xid);
            }
        }

        // re-acquire the MergeLock to remove the entry we made above
        let merge_lock = metadata.acquire_merge_lock();
        merge_lock
            .merge_list()
            .remove_entry(merge_entry)
            .expect("should be able to remove MergeEntry");
        drop(merge_lock);

        // we can garbage collect and return blocks back to the FSM without being under the MergeLock
        if mode == MergeMode::Foreground {
            garbage_collect_index(indexrel, current_xid, next_xid);
        }

        // if merging was cancelled due to a legit interrupt we'd prefer that be provided to the user
        check_for_interrupts!();

        if let Err(e) = merge_result {
            if unsafe { pg_sys::InterruptPending } != 0 {
                pgrx::warning!("failed to merge: {e:?} because of interrupt");
            } else {
                panic!("failed to merge: {e:?}");
            }
        }
    } else {
        drop(merge_lock);
    }
    drop(merge_pins);
}

///
/// Garbage collect the segments, removing any which are no longer visible in transactions
/// occurring in this process.
///
/// If physical replicas might still be executing transactions on some segments, then they are
/// moved to the `SEGMENT_METAS_GARBAGE` list until those replicas indicate that they are no longer
/// in use, at which point they can be freed by `free_garbage`.
///
pub unsafe fn garbage_collect_index(
    indexrel: &PgSearchRelation,
    current_xid: pg_sys::FullTransactionId,
    next_xid: pg_sys::FullTransactionId,
) {
    // Remove items which are no longer visible to active local transactions from SEGMENT_METAS,
    // and place them in SEGMENT_METAS_RECYLCABLE until they are no longer visible to remote
    // transactions either.
    //
    // SEGMENT_METAS must be updated atomically so that a consistent list is visible for consumers:
    // SEGMENT_METAS_GARBAGE need not be because it is only ever consumed on the physical
    // replication primary.
    let mut segment_metas_linked_list = MetaPage::open(indexrel).segment_metas();
    let mut has_recyclable = false;
    segment_metas_linked_list.for_each(|bman, entry| {
        has_recyclable = has_recyclable || entry.recyclable(bman);
    });
    if !has_recyclable {
        return;
    }

    let mut segment_metas = segment_metas_linked_list.atomically();
    let entries = segment_metas.garbage_collect(next_xid);

    // Replication is not enabled: immediately free the entries. It doesn't matter when we
    // commit the segment metas list in this case.
    segment_metas.commit();
    free_entries(indexrel, entries, current_xid);
}

/// Chase down all the files in a segment and return them to the FSM.
pub fn free_entries(
    indexrel: &PgSearchRelation,
    freeable_entries: Vec<SegmentMetaEntry>,
    current_xid: pg_sys::FullTransactionId,
) {
    let mut bman = BufferManager::new(indexrel);
    bman.fsm().extend_with_when_recyclable(
        &mut bman,
        current_xid,
        freeable_entries
            .iter()
            .flat_map(move |entry| entry.freeable_blocks(indexrel)),
    );
}

// random key, ensures that this advisory slot key doesn't conflict
// with other Postgres advisory locks
const MERGE_SLOT_KEY_BASE: u32 = 0x5047534D;

// We at most allow 2 concurrent background merges
// The first background merge is for "small" merges, the second is for "large" merges
// This makes it so that a long-running large merge doesn't block smaller merges from happening
// We arbitrarily say that a merge is "large" if the largest layer size is greater than
// or equal to this threshold
const LARGE_MERGE_THRESHOLD: u64 = 100 * 1024 * 1024; // 100mb
const SLOT_BITS: u32 = 8; // slot is u8

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum MergeSlotVariant {
    Small = 0,
    Large = 1,
}

impl TryFrom<u8> for MergeSlotVariant {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => MergeSlotVariant::Small,
            1 => MergeSlotVariant::Large,
            _ => anyhow::bail!("invalid merge slot variant: {value}"),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MergeSlot {
    index_oid: pg_sys::Oid,
    variant: MergeSlotVariant,
}

impl MergeSlot {
    fn new(index_oid: pg_sys::Oid, variant: MergeSlotVariant) -> Self {
        Self { index_oid, variant }
    }

    fn for_layer_size(index_oid: pg_sys::Oid, largest_layer_size: u64) -> Self {
        if largest_layer_size >= LARGE_MERGE_THRESHOLD {
            MergeSlot::new(index_oid, MergeSlotVariant::Large)
        } else {
            MergeSlot::new(index_oid, MergeSlotVariant::Small)
        }
    }

    fn lock(&self) -> Option<AdvisoryLock> {
        let key = self.key();
        AdvisoryLock::new_transaction(key)
    }

    // Checks to see if a merge is already running for the given slot
    //
    // NOTE: This only prevents concurrent merges from different backends,
    // will always return true if called repeatedly from the same transaction
    // See: https://www.postgresql.org/docs/current/explicit-locking.html#ADVISORY-LOCKS
    fn is_available(&self) -> bool {
        let key = self.key();
        if let Some(lock) = AdvisoryLock::conditional_lock_session(key) {
            drop(lock); // explicitly unlock immediately
            true
        } else {
            false
        }
    }

    fn key(&self) -> i64 {
        let variant = self.variant as u8;
        let payload = ((u32::from(self.index_oid) as u64) << SLOT_BITS) | (variant as u64);
        ((MERGE_SLOT_KEY_BASE as i64) << 32) | (payload as i64)
    }

    fn variant(&self) -> MergeSlotVariant {
        self.variant
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use crate::api::vector_test_support::*;
    use crate::index::merge_policy::tests::create_segment_meta_entry;
    use crate::postgres::options::{
        DEFAULT_BACKGROUND_LAYER_SIZES, DEFAULT_FOREGROUND_LAYER_SIZES,
    };
    use pgrx::prelude::*;

    #[pg_test]
    fn test_immutable_backpressure_uses_full_candidates() {
        let mut all_entries = Vec::new();
        for &size in DEFAULT_BACKGROUND_LAYER_SIZES {
            for (count, bytes, expected) in
                [(5, size / 100, false), (4, size, false), (6, size, true)]
            {
                let entries = (0..count)
                    .map(|_| create_segment_meta_entry(bytes, 10, 0))
                    .collect::<Vec<_>>();
                if expected {
                    all_entries.extend(entries.clone());
                }
                let mut policy = LayeredMergePolicy::new(vec![size]);
                policy.set_mergeable_segments_for_test(entries);
                let (candidates, _) = policy.simulate();
                let selected = backpressure(&candidates, 1);
                assert_eq!(selected.is_some(), expected);
                assert!(backpressure(&candidates, 2).is_none());
                if let Some(selected) = selected {
                    assert_eq!(selected.0.len(), 2);
                    assert!(candidates.len() > 1);
                }
            }
        }
        let smallest_pressured_ids = all_entries[6..12]
            .iter()
            .map(SegmentMetaEntry::segment_id)
            .collect::<HashSet<_>>();
        all_entries.drain(2..6);
        let mut policy = LayeredMergePolicy::new(DEFAULT_BACKGROUND_LAYER_SIZES.to_vec());
        policy.set_mergeable_segments_for_test(all_entries);
        let (candidates, largest_layer) = policy.simulate();
        assert_eq!(
            largest_layer,
            *DEFAULT_BACKGROUND_LAYER_SIZES.last().unwrap()
        );
        assert!(
            backpressure(&candidates, 1)
                .unwrap()
                .0
                .iter()
                .all(|id| smallest_pressured_ids.contains(id))
        );
    }

    #[pg_test]
    fn test_backpressure_uses_background_layer_limits() {
        for (count, background_expected, backpressure_expected) in [
            (40, false, false),
            (41, false, false),
            (50, false, true),
            (80, true, true),
        ] {
            let bytes = 2 * 1024 * 1024;
            let layers = IndexLayerSizes::new(
                vec![],
                DEFAULT_BACKGROUND_LAYER_SIZES.to_vec(),
                count,
                bytes * count as u64,
                10,
            );
            let mut policy = LayeredMergePolicy::new(layers.combined());
            let entries = (0..count)
                .map(|_| create_segment_meta_entry(bytes, 10, 0))
                .collect::<Vec<_>>();
            policy.set_mergeable_segments_for_test(entries.clone());
            let (candidates, _) = policy.simulate();
            assert_eq!(!candidates.is_empty(), background_expected);

            policy.set_mergeable_segments_for_test(entries);
            policy.prepare_backpressure(layers.backpressure_layer_size);
            let (candidates, _) = policy.simulate();
            assert_eq!(
                backpressure(&candidates, 10).is_some(),
                backpressure_expected
            );
        }
    }

    #[pg_test]
    fn test_foreground_backpressure_fills_clamped_layer_gap() {
        Spi::run(
            "CREATE TABLE backpressure_gap (id bigint, body text);
            CREATE INDEX backpressure_gap_idx ON backpressure_gap USING paradedb (id, body)
            WITH (target_segment_count=2, mutable_segment_rows=0, background_layer_sizes='0');",
        )
        .unwrap();
        for id in 0..12 {
            Spi::run(&format!(
                "INSERT INTO backpressure_gap VALUES ({id}, 'beer wine cheese')"
            ))
            .unwrap();
        }
        Spi::run(
            "ALTER INDEX backpressure_gap_idx SET (background_layer_sizes='1 bytes,100mb');
            INSERT INTO backpressure_gap VALUES (12, 'beer wine cheese');",
        )
        .unwrap();
        let segments = Spi::get_one::<i64>(
            "SELECT count(*) FROM paradedb.index_info('backpressure_gap_idx') WHERE visible",
        )
        .unwrap()
        .unwrap();
        assert!(segments < 12, "foreground backpressure must close the gap");
        assert_eq!(
            Spi::get_one::<i64>("SELECT count(*) FROM backpressure_gap WHERE id @@@ pdb.all()")
                .unwrap(),
            Some(13)
        );
        assert_eq!(Spi::get_one::<bool>("SELECT bool_and(passed) FROM pdb.verify_index('backpressure_gap_idx', heapallindexed => true)").unwrap(), Some(true));
    }

    #[pg_test]
    unsafe fn test_backpressure_excludes_reserved_segments_and_claims_one_merge() {
        Spi::run(
            "CREATE TABLE backpressure (id bigint, body text);
            CREATE INDEX backpressure_idx ON backpressure USING paradedb (id, body)
            WITH (target_segment_count=2, mutable_segment_rows=0, background_layer_sizes='0');",
        )
        .unwrap();
        for id in 0..12 {
            Spi::run(&format!(
                "INSERT INTO backpressure VALUES ({id}, 'beer wine cheese')"
            ))
            .unwrap();
        }
        let oid = Spi::get_one::<pg_sys::Oid>("SELECT 'backpressure_idx'::regclass::oid")
            .unwrap()
            .unwrap();
        let index = PgSearchRelation::open(oid);
        let entries = MetaPage::open(&index).segment_metas().list(None);
        let bytes = entries
            .iter()
            .map(SegmentMetaEntry::byte_size)
            .max()
            .unwrap();
        let ids = entries
            .iter()
            .map(SegmentMetaEntry::segment_id)
            .collect::<Vec<_>>();
        drop(index);
        Spi::run(&format!(
            "ALTER INDEX backpressure_idx SET (background_layer_sizes='{bytes} bytes')"
        ))
        .unwrap();
        let index = PgSearchRelation::open(oid);
        let mut metadata = MetaPage::open(&index);
        let merge_lock = metadata.acquire_merge_lock();
        let xid = pg_sys::GetCurrentFullTransactionId();
        let reservation = merge_lock
            .merge_list()
            .add_segment_ids(ids[..6].iter(), xid)
            .unwrap();
        metadata.vacuum_list().write_list(ids[6..8].iter());
        let vacuum_sentinel = metadata.pin_ambulkdelete_sentinel();
        let merger = SearchIndexMerger::open(&index, MvccSatisfies::Mergeable).unwrap();
        let mut policy = LayeredMergePolicy::new(IndexLayerSizes::from(&index).combined());
        policy.set_mergeable_segment_entries(&metadata, &merge_lock, &merger);
        let (candidates, _, _) = policy.simulate_supported(&merger).unwrap();
        assert_eq!(
            candidates
                .iter()
                .map(|(_, candidate)| candidate.0.len())
                .sum::<usize>(),
            4
        );
        assert!(backpressure(&candidates, 2).is_none());
        merge_lock.merge_list().remove_entry(reservation).unwrap();
        drop(vacuum_sentinel);
        let mut policy = LayeredMergePolicy::new(IndexLayerSizes::from(&index).combined());
        policy.set_mergeable_segment_entries(&metadata, &merge_lock, &merger);
        let (candidates, _, _) = policy.simulate_supported(&merger).unwrap();
        let selected = backpressure(&candidates, 2).unwrap().clone();
        assert!(candidates.len() > 1);
        merge_index(
            &index,
            merger,
            std::slice::from_ref(&selected),
            merge_lock,
            metadata.pin_merge(),
            MergeMode::Foreground,
            xid,
            xid,
        );
        let remaining = metadata
            .segment_metas()
            .list(None)
            .into_iter()
            .filter(|entry| entry.visible())
            .map(|entry| entry.segment_id())
            .collect::<HashSet<_>>();
        assert_eq!(remaining.len(), ids.len() - selected.0.len() + 1);
        assert_eq!(
            ids.iter().filter(|id| !remaining.contains(id)).count(),
            selected.0.len()
        );
        assert!(
            metadata
                .acquire_merge_lock()
                .merge_list()
                .list_segment_ids()
                .next()
                .is_none()
        );
        drop(index);
        assert_eq!(
            Spi::get_one::<i64>("SELECT count(*) FROM backpressure WHERE id @@@ pdb.all()")
                .unwrap(),
            Some(12)
        );
        assert_eq!(Spi::get_one::<bool>("SELECT bool_and(passed) FROM pdb.verify_index('backpressure_idx', heapallindexed => true)").unwrap(), Some(true));
    }

    #[pg_test]
    fn foreground_vector_merge_skips_unsupported_storage() {
        let indexrel = vector_metadata_fixture();
        let oid = indexrel.oid();
        let layer_bytes = unsafe { MetaPage::open(&indexrel).segment_metas().list(None) }
            .iter()
            .map(|entry| entry.byte_size())
            .max()
            .unwrap();
        drop(indexrel);
        Spi::run(&format!("ALTER INDEX metadata_vectors_idx SET (layer_sizes='{layer_bytes} bytes',background_layer_sizes='0',mutable_segment_rows=0)")).unwrap();
        let indexrel = PgSearchRelation::open(oid);
        replace_vector_version(&indexrel, 3);
        let old = unsafe { MetaPage::open(&indexrel).segment_metas().list(None) }
            .into_iter()
            .filter(|entry| !entry.is_deleted())
            .map(|entry| entry.segment_id())
            .collect::<Vec<_>>();
        Spi::run(
            "INSERT INTO metadata_vectors SELECT g, ARRAY(SELECT ((g+i)%17+1)::real FROM generate_series(1,1024) i)::vector FROM generate_series(2049,4096) g",
        ).unwrap();
        let current = unsafe { MetaPage::open(&indexrel).segment_metas().list(None) };
        assert!(old.iter().all(|id| {
            current
                .iter()
                .any(|entry| entry.segment_id() == *id && !entry.is_deleted())
        }));
        assert!(WARNED_VECTOR_INDEXES.with(|warned| warned.borrow().contains(&oid)));
        // A repeated write keeps the same backend warning state while skipping the unsupported segment.
        Spi::run("UPDATE metadata_vectors SET vec = vec WHERE id = 4096").unwrap();
        assert!(WARNED_VECTOR_INDEXES.with(|warned| warned.borrow().contains(&oid)));
        expect_reindex(&vector_query());
        drop(indexrel);
        Spi::run("REINDEX INDEX metadata_vectors_idx").unwrap();
        Spi::run(&vector_query()).unwrap();
        assert_eq!(
            Spi::get_one::<i64>("SELECT count(*) FROM metadata_vectors").unwrap(),
            Some(4096)
        );
    }

    enum LayerSizes {
        Default,
        Foreground(String),
        Background(String),
    }

    impl LayerSizes {
        fn config_str(&self) -> String {
            match self {
                LayerSizes::Default => "".to_string(),
                LayerSizes::Foreground(sizes) => format!(" WITH (layer_sizes = '{sizes}')"),
                LayerSizes::Background(sizes) => {
                    format!(" WITH (background_layer_sizes = '{sizes}')")
                }
            }
        }
    }

    fn create_index_with_layer_sizes(layer_sizes: LayerSizes) -> pg_sys::Oid {
        Spi::run("SET client_min_messages = 'debug1';").unwrap();
        Spi::run("CREATE TABLE IF NOT EXISTS t (id SERIAL, data TEXT);").unwrap();
        Spi::run("INSERT INTO t (data) VALUES ('test');").unwrap();
        Spi::run(
            format!(
                "CREATE INDEX t_idx ON t USING paradedb (id, data){}",
                layer_sizes.config_str()
            )
            .as_str(),
        )
        .unwrap();
        Spi::get_one::<pg_sys::Oid>(
            "SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';",
        )
        .expect("spi should succeed")
        .unwrap()
    }

    #[pg_test]
    unsafe fn test_garbage_collect_skips_live_and_pinned_segments() {
        use crate::postgres::storage::block::LinkedList;

        let indexrel = PgSearchRelation::open(create_index_with_layer_sizes(LayerSizes::Default));
        let mut metas = MetaPage::open(&indexrel).segment_metas();
        let start_blockno = metas.get_linked_list_data().start_blockno;
        let xid = pg_sys::ReadNextFullTransactionId();
        garbage_collect_index(&indexrel, xid, xid);
        assert_eq!(start_blockno, {
            metas.get_linked_list_data().start_blockno
        });

        let entry = metas.list(None)[0];
        let pin = metas.bman().pinned_buffer(entry.pintest_blockno());
        metas
            .update_item(
                |candidate| candidate.segment_id() == entry.segment_id(),
                |candidate| candidate.set_xmax(pg_sys::FrozenTransactionId),
            )
            .unwrap();
        garbage_collect_index(&indexrel, xid, xid);
        assert_eq!(start_blockno, {
            metas.get_linked_list_data().start_blockno
        });
        assert_eq!(metas.list(None).len(), 1);

        drop(pin);
        garbage_collect_index(&indexrel, xid, xid);
        assert!(metas.list(None).is_empty());
    }

    #[pg_test]
    fn test_configured_foreground_layer_sizes() {
        let index_oid = create_index_with_layer_sizes(LayerSizes::Foreground(
            "1kb, 10kb, 100kb, 1mb".to_string(),
        ));
        let index = PgSearchRelation::open(index_oid);
        let layer_sizes = index.options().foreground_layer_sizes();
        assert_eq!(layer_sizes, vec![1024, 10240, 102400, 1048576]);
    }

    #[pg_test]
    fn test_configured_background_layer_sizes() {
        let index_oid = create_index_with_layer_sizes(LayerSizes::Background(
            "1kb, 10kb, 100kb, 1mb".to_string(),
        ));
        let index = PgSearchRelation::open(index_oid);
        let layer_sizes = index.options().background_layer_sizes();
        assert_eq!(layer_sizes, vec![1024, 10240, 102400, 1048576]);
    }

    #[pg_test]
    fn test_zeroed_foreground_layer_sizes() {
        let index_oid = create_index_with_layer_sizes(LayerSizes::Foreground("0".to_string()));
        let index = PgSearchRelation::open(index_oid);
        let layer_sizes = index.options().foreground_layer_sizes();
        assert!(layer_sizes.is_empty());
    }

    #[pg_test]
    fn test_zeroed_background_layer_sizes() {
        let index_oid = create_index_with_layer_sizes(LayerSizes::Background("0".to_string()));
        let index = PgSearchRelation::open(index_oid);
        let layer_sizes = index.options().background_layer_sizes();
        assert!(layer_sizes.is_empty());
    }

    #[pg_test]
    fn test_default_foreground_layer_sizes() {
        let index_oid = create_index_with_layer_sizes(LayerSizes::Default);
        let index = PgSearchRelation::open(index_oid);
        let layer_sizes = index.options().foreground_layer_sizes();
        assert_eq!(layer_sizes, DEFAULT_FOREGROUND_LAYER_SIZES.to_vec());
    }

    #[pg_test]
    fn test_default_background_layer_sizes() {
        let index_oid = create_index_with_layer_sizes(LayerSizes::Default);
        let index = PgSearchRelation::open(index_oid);
        let layer_sizes = index.options().background_layer_sizes();
        assert_eq!(layer_sizes, DEFAULT_BACKGROUND_LAYER_SIZES.to_vec());
    }

    #[pg_test]
    fn test_layer_sizes_clamped_to_target_segment_size() {
        // 200mb over 10 segments: the target segment size is ~13.3mb, so the 100mb layer goes
        for count in [20, 41, 100] {
            let layers = IndexLayerSizes::new(
                vec![],
                vec![1024 * 1024, 10 * 1024 * 1024, 100 * 1024 * 1024],
                count,
                200 * 1024 * 1024,
                10,
            );
            assert_eq!(
                layers.background_layer_sizes,
                vec![1024 * 1024, 10 * 1024 * 1024]
            );
        }
        let layers = IndexLayerSizes::new(vec![], vec![1024, 64 * 1024], 500, 500 * 687, 1);
        assert_eq!(layers.background_layer_sizes, vec![1024, 64 * 1024]);
        assert!(layers.backpressure_layer_size.is_none());
    }

    #[pg_test]
    fn test_layer_sizes_clamped_away_merge_to_target_segment_size() {
        // 4.5mb of single-row segments with a target of 224 segments: the target segment size is
        // ~14kb, below every default layer, so merge up to it rather than not merging at all
        let layers = IndexLayerSizes::new(
            vec![],
            DEFAULT_BACKGROUND_LAYER_SIZES.to_vec(),
            3438,
            4_739_219,
            224,
        );
        let target_segment_byte_size = 4_739_219 / 224 - 4_739_219 / 224 / 3;
        assert_eq!(
            layers.background_layer_sizes,
            vec![target_segment_byte_size]
        );
        assert_eq!(layers.combined(), vec![target_segment_byte_size]);
        assert!(layers.backpressure_layer_size.is_none());
    }

    #[pg_test]
    fn test_layer_sizes_from_index_with_small_segments() {
        // 20 single-row segments under a small target: every default layer is larger than the
        // target segment size
        Spi::run("CREATE TABLE t (id SERIAL, data TEXT);").unwrap();
        Spi::run(
            "CREATE INDEX t_idx ON t USING paradedb (id, data) \
             WITH (mutable_segment_rows = 0, target_segment_count = 8);",
        )
        .unwrap();
        for _ in 0..20 {
            Spi::run("INSERT INTO t (data) VALUES ('test');").unwrap();
        }
        let index_oid = Spi::get_one::<pg_sys::Oid>(
            "SELECT oid FROM pg_class WHERE relname = 't_idx' AND relkind = 'i';",
        )
        .expect("spi should succeed")
        .unwrap();

        let layers = IndexLayerSizes::from(&PgSearchRelation::open(index_oid));
        let combined = layers.combined();
        assert_eq!(combined.len(), 1, "{combined:?}");
        assert!(
            combined[0] < DEFAULT_BACKGROUND_LAYER_SIZES[0],
            "{combined:?}"
        );
    }

    #[pg_test]
    fn test_layer_sizes_at_target_segment_count() {
        let layers = IndexLayerSizes::new(
            vec![],
            DEFAULT_BACKGROUND_LAYER_SIZES.to_vec(),
            224,
            4_739_219,
            224,
        );
        assert!(layers.background_layer_sizes.is_empty());
        assert!(layers.backpressure_layer_size.is_none());
        assert!(layers.user_configured_background_layers());
    }

    #[pg_test]
    fn test_layer_sizes_background_merging_disabled() {
        let layers = IndexLayerSizes::new(vec![], vec![], 3438, 4_739_219, 224);
        assert!(layers.background_layer_sizes.is_empty());
        assert!(layers.backpressure_layer_size.is_none());
        assert!(!layers.user_configured_background_layers());
    }

    #[pg_test]
    fn test_background_merge_args() {
        let args = BackgroundMergeArgs::new(pg_sys::Oid::from(100), MergeSlotVariant::Large);
        let datum = args.into_datum().unwrap();
        let args2 = unsafe { BackgroundMergeArgs::from_datum(datum, false).unwrap() };
        assert_eq!(args, args2);

        let args = BackgroundMergeArgs::new(pg_sys::Oid::from(0), MergeSlotVariant::Small);
        let datum = args.into_datum().unwrap();
        let args2 = unsafe { BackgroundMergeArgs::from_datum(datum, false).unwrap() };
        assert_eq!(args, args2);

        let args = BackgroundMergeArgs::new(pg_sys::Oid::from(u32::MAX), MergeSlotVariant::Large);
        let datum = args.into_datum().unwrap();
        let args2 = unsafe { BackgroundMergeArgs::from_datum(datum, false).unwrap() };
        assert_eq!(args, args2);
    }

    #[pg_test]
    unsafe fn test_merge_slot() {
        let index_oid = create_index_with_layer_sizes(LayerSizes::Background("0".to_string()));
        let small_layer = 1024;
        let large_layer = 1024 * 1024 * 1024;

        let small_slot = MergeSlot::for_layer_size(index_oid, small_layer);
        let large_slot = MergeSlot::for_layer_size(index_oid, large_layer);
        assert_eq!(small_slot.variant(), MergeSlotVariant::Small);
        assert_eq!(large_slot.variant(), MergeSlotVariant::Large);
        assert!(small_slot.is_available());
        assert!(large_slot.is_available());

        let small_locked = small_slot.lock();
        assert!(small_locked.is_some());

        let large_locked = large_slot.lock();
        assert!(large_locked.is_some());

        let pid = pg_sys::MyProcPid;
        let cnt = Spi::get_one::<i64>(&format!(
            "SELECT count(*)::bigint FROM pg_locks WHERE pid = {} AND locktype = 'advisory'",
            pid
        ))
        .unwrap()
        .unwrap();
        assert_eq!(cnt, 2);
    }
}
