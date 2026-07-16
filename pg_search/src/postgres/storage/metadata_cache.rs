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

//! A backend-local cache of write-once BM25 index metadata.
//!
//! Opening a [`SearchIndexReader`] re-reads the metapage (block 0), the schema chain, and the
//! settings chain through the Postgres buffer manager on every query, and re-deserializes the
//! schema/settings JSON each time.  All of that data is write-once: the schema and settings
//! chains are only ever written when empty (see `save_schema`/`save_settings`), and the
//! metapage's block-number pointers and version stamps never change after `MetaPage::init` /
//! first-open upgrade.
//!
//! This module caches those write-once facts in backend-local memory, keyed by the index's
//! relation oid and validated against its relfilenode.  This is the same idea as the btree and
//! hash index metapage caches (`rel->rd_amcache`), but kept on the Rust side so `Drop` runs for
//! the cached tantivy objects.
//!
//! Invalidation:
//! - a relcache invalidation callback evicts the entry (DDL, REINDEX, DROP, transaction abort
//!   after a CREATE INDEX all fire one), and
//! - every lookup revalidates the stored relfilenode, so a `REINDEX`ed index (new relfilenode)
//!   misses the cache even if the callback was somehow not delivered.
//!
//! The `ambulkdelete_epoch` metapage field is deliberately NOT served from this cache: it is
//! mutated by every `ambulkdelete()`.  [`MetaPage`] instances constructed from cached data
//! refuse to answer `ambulkdelete_epoch()` — callers that need the epoch must use
//! [`MetaPage::open`].

use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::metadata::MetaPageData;
use pgrx::pg_sys;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use tantivy::index::IndexSettings;
use tantivy::schema::Schema;

/// Everything we cache for one index.  Fields are populated independently as the various load
/// paths first run; all of them are write-once for the lifetime of a relfilenode.
#[derive(Default)]
struct CachedIndexMetadata {
    meta: Option<MetaPageData>,
    schema: Option<Schema>,
    settings: Option<IndexSettings>,
}

struct CacheKeyed {
    relfilenode: pg_sys::Oid,
    cached: CachedIndexMetadata,
}

// Backends are single-threaded, so a thread-local map is effectively backend-local.
thread_local! {
    static CACHE: RefCell<HashMap<pg_sys::Oid, CacheKeyed>> = RefCell::new(HashMap::new());
    static CALLBACK_REGISTERED: Cell<bool> = const { Cell::new(false) };
}

/// An arbitrary bound so a backend that touches many indexes over its lifetime cannot grow the
/// map without limit.  Entries are tiny (a couple of KB each); precision eviction isn't worth
/// complexity here.
const MAX_ENTRIES: usize = 64;

/// Relcache invalidation callback: evict the invalidated relation, or everything on a global
/// invalidation (`relid == InvalidOid`).
///
/// NB: must not panic — it is called from deep inside Postgres cache-invalidation processing.
unsafe extern "C-unwind" fn relcache_callback(_arg: pg_sys::Datum, relid: pg_sys::Oid) {
    let _ = CACHE.try_with(|cache| {
        if let Ok(mut cache) = cache.try_borrow_mut() {
            if relid == pg_sys::InvalidOid {
                cache.clear();
            } else {
                cache.remove(&relid);
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

fn relfilenode(indexrel: &PgSearchRelation) -> pg_sys::Oid {
    // pg_class.relfilenode: changes on REINDEX and any other relation rewrite, on every
    // supported Postgres version.  User indexes are never mapped relations, so it's never 0.
    unsafe { (*indexrel.rd_rel).relfilenode }
}

fn get<T>(
    indexrel: &PgSearchRelation,
    read: impl FnOnce(&CachedIndexMetadata) -> Option<T>,
) -> Option<T> {
    let oid = indexrel.oid();
    let relfilenode = relfilenode(indexrel);
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        match cache.get(&oid) {
            Some(entry) if entry.relfilenode == relfilenode => read(&entry.cached),
            Some(_) => {
                // stale relfilenode: the index was rewritten under the same oid
                cache.remove(&oid);
                None
            }
            None => None,
        }
    })
}

fn store(indexrel: &PgSearchRelation, write: impl FnOnce(&mut CachedIndexMetadata)) {
    ensure_callback_registered();
    let oid = indexrel.oid();
    let relfilenode = relfilenode(indexrel);
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() >= MAX_ENTRIES && !cache.contains_key(&oid) {
            cache.clear();
        }
        let entry = cache.entry(oid).or_insert_with(|| CacheKeyed {
            relfilenode,
            cached: CachedIndexMetadata::default(),
        });
        if entry.relfilenode != relfilenode {
            entry.relfilenode = relfilenode;
            entry.cached = CachedIndexMetadata::default();
        }
        write(&mut entry.cached);
    });
}

pub fn cached_metapage_data(indexrel: &PgSearchRelation) -> Option<MetaPageData> {
    get(indexrel, |c| c.meta)
}

pub fn store_metapage_data(indexrel: &PgSearchRelation, data: MetaPageData) {
    store(indexrel, |c| c.meta = Some(data));
}

pub fn cached_schema(indexrel: &PgSearchRelation) -> Option<Schema> {
    get(indexrel, |c| c.schema.clone())
}

pub fn store_schema(indexrel: &PgSearchRelation, schema: &Schema) {
    store(indexrel, |c| c.schema = Some(schema.clone()));
}

pub fn cached_settings(indexrel: &PgSearchRelation) -> Option<IndexSettings> {
    get(indexrel, |c| c.settings.clone())
}

pub fn store_settings(indexrel: &PgSearchRelation, settings: &IndexSettings) {
    store(indexrel, |c| c.settings = Some(settings.clone()));
}
