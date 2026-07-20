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

//! An experimental backend-local cache that reuses the most recently opened index reader
//! across queries (gated behind `paradedb.enable_reader_cache`).
//!
//! Opening a reader costs work linear in the segment count: walking the segment-metas list,
//! MVCC-checking and pinning every segment, and constructing a tantivy `SegmentReader` per
//! segment.  All of it re-derives the same answer as the previous query whenever the segment
//! list hasn't changed.  This cache parks the opened components and proves "unchanged" with
//! two on-disk counters read in a single metapage access:
//!
//! - `segment_metas_version`: bumped by every write to the segment-metas list (inserts,
//!   merges, garbage collection, mutable-segment updates).  Segment visibility in this
//!   codebase is purely a function of list contents (`SegmentMetaEntry::visible()` checks a
//!   frozen-xmax tombstone, not a snapshot), so an unchanged list means an identical visible
//!   set for every later query.  Recycling of segment files is likewise impossible without a
//!   list change: only tombstoned entries are recyclable, and tombstoning is a list write.
//! - `ambulkdelete_epoch`: bumped by vacuum, which can swap delete files under segments.
//!
//! This is the same optimistic cache-then-validate pattern as the btree and hash index
//! metapage caches, applied to a bigger object.
//!
//! # Lifetime management
//!
//! The parked structures hold nothing that can dangle across transactions:
//!
//! - **No buffer pins.**  Parked segment component readers run in "copy" mode (decoded
//!   blocks are backend-local copies, never pinned-page views; see
//!   `LinkedBytesList::open_copying`), and the pin cushion is moved out of the parked
//!   directory into the current query's reader, which drops it inside the transaction.
//!   Between queries the parked segments need no pin protection: recycling requires a
//!   segment-metas write, which fails validation before the parked structures are read.
//!   At reuse, a fresh cushion is pinned under the current query's resource owner.
//! - **The relation pointer** embedded in the parked structures is retargeted to the current
//!   query's open relation at every reuse (`PgSearchRelation::retarget`), and never
//!   dereferenced between queries.  Its close duty is stolen at park time
//!   (`PgSearchRelation::steal_close_duty`) and handed to the current query's reader as a
//!   [`RelationCloseGuard`], which closes inside the transaction — the parked clones would
//!   otherwise suppress the last-reference close and leak a relcache reference.
//! - **Mutable (in-memory) segments**: their contents change with every insert, so a reader
//!   whose visible set includes one is never parked.

use crate::index::directory::mvcc::{MVCCDirectory, PinCushion};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::SegmentMetaEntry;
use crate::postgres::storage::metadata::MetaPage;
use pgrx::pg_sys;
use std::cell::{Cell, RefCell};
use std::ptr::NonNull;
use std::sync::Arc;

use super::index::IndexComponents;

/// Everything a reader must keep alive for exactly the duration of its query: the pins that
/// protect its segments from recycling, and (when a reader was parked during this query) the
/// stolen duty to close the relation the parked structures were built against.  Dropped with
/// the last clone of the reader, inside the transaction.
#[derive(Default)]
pub(crate) struct QueryLifetime {
    _pin_cushion: Option<PinCushion>,
    _close_guard: Option<RelationCloseGuard>,
}

// SAFETY: like `MVCCDirectory`, this is only ever used within a single Postgres backend,
// which is single-threaded; it never actually crosses a thread boundary.
unsafe impl Send for QueryLifetime {}
unsafe impl Sync for QueryLifetime {}

/// Closes a relation on drop, iff still inside a transaction (on abort, the transaction's own
/// cleanup releases the reference silently).
pub(crate) struct RelationCloseGuard {
    relation: NonNull<pg_sys::RelationData>,
    lockmode: Option<pg_sys::LOCKMODE>,
}

impl Drop for RelationCloseGuard {
    fn drop(&mut self) {
        unsafe {
            if pg_sys::IsTransactionState() && !std::thread::panicking() {
                match self.lockmode {
                    Some(lockmode) => pg_sys::relation_close(self.relation.as_ptr(), lockmode),
                    None => pg_sys::RelationClose(self.relation.as_ptr()),
                }
            }
        }
    }
}

/// The validation stamp captured *before* a reader is built.  Capturing before (not after)
/// means a concurrent list write between capture and build can only cause a spurious rebuild
/// on the next query, never a stale reuse.
#[derive(Copy, Clone, PartialEq, Eq)]
pub(crate) struct ReaderStamp {
    relfilenode: pg_sys::Oid,
    segment_metas_version: u64,
    ambulkdelete_epoch: u32,
}

struct ParkedReader {
    index_oid: pg_sys::Oid,
    stamp: ReaderStamp,
    /// The relation handle whose allocation every internal clone shares; disarmed of its
    /// close duty at park time, retargeted at every reuse, never dereferenced between
    /// queries.
    rel: PgSearchRelation,
    directory: MVCCDirectory,
    // The parked pieces hold plain memory only — deliberately NOT an `IndexComponents`,
    // which carries query-scoped pins (the cleanup lock, the QueryLifetime).  Each checkout
    // assembles fresh components around these.
    index: tantivy::Index,
    reader: tantivy::IndexReader,
    searcher: tantivy::Searcher,
    schema: crate::schema::SearchIndexSchema,
    total_segment_count: usize,
    total_docs: u64,
    segment_meta_entries: Arc<crate::api::HashMap<tantivy::index::SegmentId, SegmentMetaEntry>>,
}

thread_local! {
    // one slot: the most recently opened cacheable reader.  Backends are single-threaded.
    static PARKED: RefCell<Option<ParkedReader>> = const { RefCell::new(None) };
    static CALLBACK_REGISTERED: Cell<bool> = const { Cell::new(false) };
}

fn relfilenode(indexrel: &PgSearchRelation) -> pg_sys::Oid {
    unsafe { (*indexrel.rd_rel).relfilenode }
}

/// Read the current validation stamp with a single metapage access.
pub(crate) unsafe fn stamp(indexrel: &PgSearchRelation) -> ReaderStamp {
    let metapage = MetaPage::open(indexrel);
    ReaderStamp {
        relfilenode: relfilenode(indexrel),
        segment_metas_version: metapage.segment_metas_version(),
        ambulkdelete_epoch: metapage.ambulkdelete_epoch(),
    }
}

/// Try to satisfy an open with the parked reader.  On success the caller gets components
/// sharing the parked tantivy structures, with a freshly pinned cleanup lock, a fresh pin
/// cushion owned by this query, and no close duty (this query's scan owns its own relation).
pub(crate) unsafe fn checkout(indexrel: &PgSearchRelation) -> Option<IndexComponents> {
    PARKED.with(|slot| {
        let mut slot = slot.borrow_mut();
        let parked = slot.as_ref()?;

        if parked.index_oid != indexrel.oid() {
            return None; // different index: leave the parked reader alone
        }

        // one metapage access validates the stamp AND provides the cleanup lock
        let metapage = MetaPage::open(indexrel);
        let current = ReaderStamp {
            relfilenode: relfilenode(indexrel),
            segment_metas_version: metapage.segment_metas_version(),
            ambulkdelete_epoch: metapage.ambulkdelete_epoch(),
        };
        if parked.stamp != current {
            // stale: the segment list (or a vacuum, or a REINDEX) changed underneath us.
            // dropping the parked reader is pure memory cleanup: it holds no pins and no
            // close duty
            *slot = None;
            return None;
        }

        // still current: revalidate the embedded relation pointer and re-arm this query's
        // segment pins under the current resource owner
        let parked = slot.as_ref().unwrap();
        parked.rel.retarget(indexrel.as_ptr());
        let pin_cushion = parked.directory.pin_cushion_for_query(indexrel);

        Some(IndexComponents {
            cleanup_lock: Arc::new(metapage.cleanup_lock_pinned()),
            index: parked.index.clone(),
            reader: parked.reader.clone(),
            searcher: parked.searcher.clone(),
            total_segment_count: parked.total_segment_count,
            total_docs: parked.total_docs,
            schema: parked.schema.clone(),
            segment_meta_entries: parked.segment_meta_entries.clone(),
            query_lifetime: Arc::new(QueryLifetime {
                _pin_cushion: Some(pin_cushion),
                _close_guard: None,
            }),
        })
    })
}

/// Park a freshly built reader for reuse by later queries, returning the [`QueryLifetime`]
/// the *current* query's reader must own (the pins moved out of the parked directory, plus
/// the relation close duty stolen from the now-shared handle).  `stamp` must have been
/// captured with [`stamp`] *before* the reader was built.
pub(crate) unsafe fn store(
    indexrel: &PgSearchRelation,
    directory: &MVCCDirectory,
    components: &IndexComponents,
    stamp: ReaderStamp,
) -> Option<QueryLifetime> {
    if directory.has_mutable_segments() {
        // mutable segment contents change with every insert; never cacheable
        return None;
    }

    ensure_callback_registered();

    // this query's reader takes ownership of the pins (dropped in-transaction with the
    // reader) and of the relation close duty (the parked clones suppress the normal
    // last-reference close)
    let query_lifetime = QueryLifetime {
        _pin_cushion: directory.take_pin_cushion(),
        _close_guard: indexrel
            .steal_close_duty()
            .map(|(relation, lockmode)| RelationCloseGuard { relation, lockmode }),
    };

    let parked = ParkedReader {
        index_oid: indexrel.oid(),
        stamp,
        rel: indexrel.clone(),
        directory: directory.clone(),
        index: components.index.clone(),
        reader: components.reader.clone(),
        searcher: components.searcher.clone(),
        schema: components.schema.clone(),
        total_segment_count: components.total_segment_count,
        total_docs: components.total_docs,
        segment_meta_entries: components.segment_meta_entries.clone(),
    };

    PARKED.with(|slot| {
        // dropping a previously parked reader is pure memory cleanup
        *slot.borrow_mut() = Some(parked);
    });

    Some(query_lifetime)
}

/// Evict on relcache invalidation (DDL, REINDEX, DROP, aborted CREATE INDEX).  Dropping the
/// parked reader is pure memory cleanup — it holds no pins and no close duty — so this is
/// safe in any invalidation context.
unsafe extern "C-unwind" fn relcache_callback(_arg: pg_sys::Datum, relid: pg_sys::Oid) {
    let _ = PARKED.try_with(|slot| {
        if let Ok(mut slot) = slot.try_borrow_mut() {
            let evict = matches!(&*slot, Some(parked)
                if relid == pg_sys::InvalidOid || parked.index_oid == relid);
            if evict {
                *slot = None;
            }
        }
    });
}

fn ensure_callback_registered() {
    CALLBACK_REGISTERED.with(|registered| {
        if !registered.get() {
            unsafe {
                pg_sys::CacheRegisterRelcacheCallback(
                    Some(relcache_callback),
                    pg_sys::Datum::from(0),
                );
            }
            registered.set(true);
        }
    });
}
