// Copyright (c) 2023-2025 ParadeDB, Inc.
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

use crate::index::merge_policy::LayeredMergePolicy;
use crate::index::mvcc::MvccSatisfies;
use crate::index::writer::index::{Mergeable, SearchIndexMerger};
use crate::postgres::ps_status::{set_ps_display_suffix, MERGING};
use crate::postgres::storage::block::SegmentMetaEntry;
use crate::postgres::storage::buffer::{Buffer, BufferManager};
use crate::postgres::storage::merge::MergeLock;
use crate::postgres::storage::metadata::MetaPage;
use crate::postgres::storage::LinkedBytesList;
use crate::postgres::PgSearchRelation;

use pgrx::bgworkers::*;
use pgrx::{check_for_interrupts, pg_sys};
use pgrx::{pg_guard, FromDatum, IntoDatum};
use std::ffi::CStr;
use tantivy::index::SegmentMeta;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum MergeStyle {
    Insert,
    Vacuum,
}

impl TryFrom<u8> for MergeStyle {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => MergeStyle::Insert,
            1 => MergeStyle::Vacuum,
            _ => anyhow::bail!("invalid merge style: {value}"),
        })
    }
}

#[derive(Debug, Copy, Clone)]
struct BackgroundMergeArgs {
    index_oid: pg_sys::Oid,
    merge_style: MergeStyle,
}

impl BackgroundMergeArgs {
    pub fn new(index_oid: pg_sys::Oid, merge_style: MergeStyle) -> Self {
        Self {
            index_oid,
            merge_style,
        }
    }

    pub fn index_oid(&self) -> pg_sys::Oid {
        self.index_oid
    }

    pub fn merge_style(&self) -> MergeStyle {
        self.merge_style
    }
}

impl IntoDatum for BackgroundMergeArgs {
    fn into_datum(self) -> Option<pg_sys::Datum> {
        let oid = u32::from(self.index_oid) as u64;
        let style = self.merge_style as u8 as u64;
        let raw: u64 = (oid << 8) | style;
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
        let index_oid = pg_sys::Oid::from((raw >> 8) as u32);
        let merge_style = MergeStyle::try_from((raw & 0xFF) as u8).ok()?;
        Some(BackgroundMergeArgs {
            index_oid,
            merge_style,
        })
    }
}

#[derive(Debug, Clone)]
struct IndexLayerSizes {
    foreground_layer_sizes: Vec<u64>,
    background_layer_sizes: Vec<u64>,
}

impl From<&PgSearchRelation> for IndexLayerSizes {
    fn from(index: &PgSearchRelation) -> Self {
        let index_options = index.options();
        let all_entries = unsafe { MetaPage::open(index).segment_metas().list() };
        let index_byte_size = all_entries
            .iter()
            .map(|entry| entry.byte_size())
            .sum::<u64>();
        let target_segment_count = index_options.target_segment_count();
        let target_byte_size = index_byte_size / target_segment_count as u64;

        // clamp the highest layer size to be less than the following:
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
        pgrx::debug1!("target_byte_size for merge: {target_byte_size}");

        let foreground_layer_sizes = index_options.foreground_layer_sizes();
        pgrx::debug1!("foreground_layer_sizes: {foreground_layer_sizes:?}");

        let mut background_layer_sizes = index_options.background_layer_sizes();
        let max_foreground_layer_size = foreground_layer_sizes.iter().max().unwrap_or(&0);
        // additionally, ensure the background layer sizes are greater than the foreground ones
        background_layer_sizes.retain(|&layer_size| {
            layer_size < target_byte_size && layer_size > *max_foreground_layer_size
        });
        pgrx::debug1!("adjusted background_layer_sizes {background_layer_sizes:?}");

        Self {
            foreground_layer_sizes,
            background_layer_sizes,
        }
    }
}
impl IndexLayerSizes {
    fn foreground(&self) -> Vec<u64> {
        self.foreground_layer_sizes.clone()
    }

    fn background(&self) -> Vec<u64> {
        self.background_layer_sizes.clone()
    }
}

/// Kick off a merge of the index, if needed.
///
/// First merge into the smaller layers in the foreground,
/// then launch a background worker to merge down the larger layers.
pub unsafe fn do_merge(
    index: &PgSearchRelation,
    style: MergeStyle,
    current_xid: Option<pg_sys::TransactionId>,
) -> anyhow::Result<()> {
    let merger = SearchIndexMerger::open(MvccSatisfies::Mergeable.directory(index))?;
    let layer_sizes = IndexLayerSizes::from(index);
    let foreground_layers = layer_sizes.foreground();
    let background_layers = layer_sizes.background();

    let metadata = MetaPage::open(index);
    let cleanup_lock = metadata.cleanup_lock_shared();
    let merge_lock = metadata.acquire_merge_lock();

    let needs_background_merge = !background_layers.is_empty() && {
        let mut background_merge_policy = LayeredMergePolicy::new(background_layers.clone());
        background_merge_policy.set_mergeable_segment_entries(&metadata, &merge_lock, &merger);
        let merge_candidates = background_merge_policy.simulate();
        !merge_candidates.is_empty()
    };

    // first merge down the foreground layers
    if !foreground_layers.is_empty() && style == MergeStyle::Insert {
        let foreground_merge_policy = LayeredMergePolicy::new(foreground_layers);
        unsafe {
            merge_index(
                index,
                foreground_merge_policy,
                merge_lock,
                cleanup_lock,
                false,
                current_xid.expect("foreground merging requires a current transaction id"),
            )
        };
    } else {
        // we no longer need to hold the [`MergeLock`] as we're not merging in the foreground
        drop(merge_lock);
    }

    pgrx::debug1!("foreground merge complete, needs_background_merge: {needs_background_merge}");

    // then launch a background process to merge down the background layers
    // only if we determine that there are enough segments to merge in the background
    if needs_background_merge || style == MergeStyle::Vacuum {
        try_launch_background_merger(index, style);
    }

    Ok(())
}

/// Try to launch a background process to merge down the index.
/// Is not guaranteed to launch the process if there are not enough `max_worker_processes` available.
unsafe fn try_launch_background_merger(index: &PgSearchRelation, style: MergeStyle) {
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
        .set_argument(BackgroundMergeArgs::new(index.oid(), style).into_datum())
        .set_extra(&dbname)
        .set_notify_pid(unsafe { pg_sys::MyProcPid })
        .load_dynamic()
        .is_err()
    {
        pgrx::log!("not enough available `max_worker_processes` to launch a background merger");
    }
}

/// Actually do the merge
/// This function is called by the background worker.
#[pg_guard]
#[no_mangle]
extern "C-unwind" fn background_merge(arg: pg_sys::Datum) {
    BackgroundWorker::attach_signal_handlers(SignalWakeFlags::SIGHUP | SignalWakeFlags::SIGTERM);
    BackgroundWorker::connect_worker_to_spi(Some(BackgroundWorker::get_extra()), None);
    BackgroundWorker::transaction(|| {
        unsafe {
            set_ps_display_suffix(MERGING.as_ptr());
            pg_sys::pgstat_report_activity(pg_sys::BackendState::STATE_RUNNING, MERGING.as_ptr());
        };

        pgrx::debug1!(
            "{}: starting background merge",
            BackgroundWorker::get_name()
        );

        let current_xid = unsafe { pg_sys::GetCurrentTransactionId() };
        let args = unsafe { BackgroundMergeArgs::from_datum(arg, false) }.unwrap();
        let index = PgSearchRelation::try_open(args.index_oid());
        if index.is_none() {
            pgrx::debug1!(
                "{}: index not found, suggesting it was just dropped",
                BackgroundWorker::get_name()
            );
            return;
        }
        let index = index.unwrap();
        let metadata = MetaPage::open(&index);
        let layer_sizes = IndexLayerSizes::from(&index);

        if args.merge_style() == MergeStyle::Vacuum {
            pgrx::debug1!(
                "{}: merging foreground layers",
                BackgroundWorker::get_name()
            );

            let foreground_layers = layer_sizes.foreground();
            let merge_policy = LayeredMergePolicy::new(foreground_layers);
            let cleanup_lock = metadata.cleanup_lock_shared();
            let merge_lock = unsafe { metadata.acquire_merge_lock() };
            unsafe {
                merge_index(
                    &index,
                    merge_policy,
                    merge_lock,
                    cleanup_lock,
                    true,
                    current_xid,
                )
            };
        }

        pgrx::debug1!(
            "{}: merging background layers",
            BackgroundWorker::get_name()
        );

        let background_layers = layer_sizes.background();
        let merge_policy = LayeredMergePolicy::new(background_layers);
        let cleanup_lock = metadata.cleanup_lock_shared();
        let merge_lock = unsafe { metadata.acquire_merge_lock() };
        unsafe {
            merge_index(
                &index,
                merge_policy,
                merge_lock,
                cleanup_lock,
                true,
                current_xid,
            )
        };
    });
}

#[inline]
unsafe fn merge_index(
    indexrel: &PgSearchRelation,
    mut merge_policy: LayeredMergePolicy,
    merge_lock: MergeLock,
    cleanup_lock: Buffer,
    gc_after_merge: bool,
    current_xid: pg_sys::TransactionId,
) {
    // take a shared lock on the CLEANUP_LOCK and hold it until this function is done.  We keep it
    // locked here so we can cause `ambulkdelete()` to block, waiting for all merging to finish
    // before it decides to find the segments it should vacuum.  The reason is that it needs to see
    // the final merged segment, not the original segments that will be deleted
    let metadata = MetaPage::open(indexrel);
    let merger = SearchIndexMerger::open(MvccSatisfies::Mergeable.directory(indexrel))
        .expect("should be able to open merger");

    // further reduce the set of segments that the LayeredMergePolicy will operate on by internally
    // simulating the process, allowing concurrent merges to consider segments we're not, only retaining
    // the segments it decides can be merged into one or more candidates
    merge_policy.set_mergeable_segment_entries(&metadata, &merge_lock, &merger);
    let merge_candidates = merge_policy.simulate();
    // before we start merging, tell the merger to release pins on the segments it won't be merging
    let mut merger = merger
        .adjust_pins(merge_policy.mergeable_segments())
        .expect("should be able to adjust merger pins");

    let mut need_gc = !gc_after_merge;
    let ncandidates = merge_candidates.len();
    if ncandidates > 0 {
        // record all the segments the SearchIndexMerger can see, as those are the ones that
        // could be merged
        let merge_entry = merge_lock
            .merge_list()
            .add_segment_ids(merge_policy.mergeable_segments())
            .expect("should be able to write current merge segment_id list");
        drop(merge_lock);

        // we are NOT under the MergeLock at this point, which allows concurrent backends to also merge
        //
        // we defer raising a panic in the face of a merge error as we need to remove the created
        // `merge_entry` whether the merge worked or not

        let mut merge_result: anyhow::Result<Option<SegmentMeta>> = Ok(None);

        for candidate in merge_candidates {
            pgrx::debug1!("merging candidate with {} segments", candidate.0.len());

            merge_result = merger.merge_segments(&candidate.0);
            if merge_result.is_err() {
                break;
            }
            if gc_after_merge {
                garbage_collect_index(indexrel, current_xid);
                need_gc = false;
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
        if need_gc {
            garbage_collect_index(indexrel, current_xid);
        }

        // if merging was cancelled due to a legit interrupt we'd prefer that be provided to the user
        check_for_interrupts!();

        if let Err(e) = merge_result {
            panic!("failed to merge: {e:?}");
        }
    } else {
        drop(merge_lock);
    }
    drop(cleanup_lock);
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
    current_xid: pg_sys::TransactionId,
) {
    // Remove items which are no longer visible to active local transactions from SEGMENT_METAS,
    // and place them in SEGMENT_METAS_RECYLCABLE until they are no longer visible to remote
    // transactions either.
    //
    // SEGMENT_METAS must be updated atomically so that a consistent list is visible for consumers:
    // SEGMENT_METAS_GARBAGE need not be because it is only ever consumed on the physical
    // replication primary.
    let mut segment_metas_linked_list = MetaPage::open(indexrel).segment_metas();
    let mut segment_metas = segment_metas_linked_list.atomically();
    let entries = segment_metas.garbage_collect();

    // Replication is not enabled: immediately free the entries. It doesn't matter when we
    // commit the segment metas list in this case.
    segment_metas.commit();
    free_entries(indexrel, entries, current_xid);
}

/// Chase down all the files in a segment and return them to the FSM
pub fn free_entries(
    indexrel: &PgSearchRelation,
    freeable_entries: Vec<SegmentMetaEntry>,
    current_xid: pg_sys::TransactionId,
) {
    let mut bman = BufferManager::new(indexrel);
    bman.fsm().extend_with_when_recyclable(
        &mut bman,
        current_xid,
        freeable_entries.iter().flat_map(move |entry| {
            // if the entry is a "fake" `DeleteEntry`, we need to free the blocks for the old `DeleteEntry` only
            let iter: Box<dyn Iterator<Item = pg_sys::BlockNumber>> = if entry.is_orphaned_delete()
            {
                let block = entry.delete.as_ref().unwrap().file_entry.starting_block;
                Box::new(LinkedBytesList::open(indexrel, block).freeable_blocks())
            // otherwise, we need to free the blocks for all the files in the `SegmentMetaEntry`
            } else {
                Box::new(entry.file_entries().flat_map(move |(file_entry, _)| {
                    LinkedBytesList::open(indexrel, file_entry.starting_block).freeable_blocks()
                }))
            };
            iter
        }),
    );
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use crate::postgres::options::{
        DEFAULT_BACKGROUND_LAYER_SIZES, DEFAULT_FOREGROUND_LAYER_SIZES,
    };
    use pgrx::prelude::*;

    enum LayerSizes {
        Default,
        Foreground(String),
        Background(String),
    }

    impl LayerSizes {
        fn config_str(&self) -> String {
            match self {
                LayerSizes::Default => "".to_string(),
                LayerSizes::Foreground(sizes) => format!(", layer_sizes = '{sizes}'"),
                LayerSizes::Background(sizes) => format!(", background_layer_sizes = '{sizes}'"),
            }
        }
    }

    fn create_index_with_layer_sizes(layer_sizes: LayerSizes) -> pg_sys::Oid {
        Spi::run("SET client_min_messages = 'debug1';").unwrap();
        Spi::run("CREATE TABLE IF NOT EXISTS t (id SERIAL, data TEXT);").unwrap();
        Spi::run("INSERT INTO t (data) VALUES ('test');").unwrap();
        Spi::run(
            format!(
                "CREATE INDEX t_idx ON t USING bm25(id, data) WITH (key_field = 'id'{})",
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
}
