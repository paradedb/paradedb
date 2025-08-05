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

use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::PgItem;
use pgrx::pg_sys::OffsetNumber;
use pgrx::{pg_sys, PgMemoryContexts};
use std::fmt::Debug;
use std::sync::LazyLock;

/// Matches Postgres's [`MAX_BUFFERS_TO_EXTEND_BY`]
pub const MAX_BUFFERS_TO_EXTEND_BY: usize = 64;

pub trait BM25Page {
    /// Read the opaque, non-decoded [`PgItem`] at `offno`.
    unsafe fn read_item(&self, offno: OffsetNumber) -> Option<PgItem>;

    /// Read the fully decoded/deserialized item at `offno` into an instance of `T`
    unsafe fn deserialize_item<T: From<PgItem>>(
        &self,
        offsetno: pg_sys::OffsetNumber,
    ) -> Option<(T, pg_sys::Size)>;
}

impl BM25Page for pg_sys::Page {
    unsafe fn deserialize_item<T: From<PgItem>>(
        &self,
        offno: OffsetNumber,
    ) -> Option<(T, pg_sys::Size)> {
        let pg_item = self.read_item(offno)?;
        let size = pg_item.1;
        Some((T::from(pg_item), size))
    }

    unsafe fn read_item(&self, offno: OffsetNumber) -> Option<PgItem> {
        let item_id = pg_sys::PageGetItemId(*self, offno);

        if (*item_id).lp_flags() != pg_sys::LP_NORMAL {
            return None;
        }

        let item = pg_sys::PageGetItem(*self, item_id);
        let size = (*item_id).lp_len() as pg_sys::Size;
        Some(PgItem(item, size))
    }
}

/// [`RelationBufferAccess`] a lower level interface for directly reading existing, and creating new,
/// buffers in an efficient manner.
///
/// Every new [`pg_sys::Buffer`] it creates is as the result of relation extension and is returned
/// with an exclusive lock.  Its [`pg_sys::Page`] representation has not been initialized.
#[derive(Clone, Debug)]
pub struct RelationBufferAccess {
    rel: PgSearchRelation,
}

unsafe impl Send for RelationBufferAccess {}
unsafe impl Sync for RelationBufferAccess {}

impl RelationBufferAccess {
    pub fn open(rel: &PgSearchRelation) -> Self {
        Self { rel: rel.clone() }
    }

    pub fn rel(&self) -> &PgSearchRelation {
        &self.rel
    }

    /// Return one [`pg_sys::BUFFER_LOCK_EXCLUSIVE`] locked [`pg_sys:Buffer`].  This buffer
    /// is guaranteed to be "new" in that it was created by extending the relation
    ///
    /// The [`pg_sys::Page`] representation has not been initialized.  The caller must do this.
    pub fn new_buffer(&self) -> pg_sys::Buffer {
        unsafe {
            pg_sys::LockRelationForExtension(self.rel.as_ptr(), pg_sys::ExclusiveLock as i32);
            let pg_buffer = extend_by_one_buffer(self.rel.as_ptr());
            pg_sys::UnlockRelationForExtension(self.rel.as_ptr(), pg_sys::ExclusiveLock as i32);
            pg_buffer
        }
    }

    /// Return an iterator of [`pg_sys::BUFFER_LOCK_EXCLUSIVE`] locked [`pg_sys:Buffer`]s.  The buffers
    /// are pinned _a priori_ but are locked during iteration.
    ///
    /// These buffers are guaranteed to be "new" in that they were created by extending the relation.
    ///
    /// The [`pg_sys::Page`] representation of each buffer has not been initialized.  The caller must do this.
    pub fn new_buffers(&self, npages: usize) -> impl Iterator<Item = pg_sys::Buffer> {
        unsafe {
            // a simple wrapper so we can make sure the buffer is released if the iterator
            // is dropped before exhaustion.
            struct BufferIter<I: Iterator<Item = pg_sys::Buffer>> {
                iter: I,
            }
            impl<I: Iterator<Item = pg_sys::Buffer>> Drop for BufferIter<I> {
                fn drop(&mut self) {
                    unsafe {
                        if !pg_sys::IsTransactionState() {
                            return;
                        }
                    }

                    for pg_buffer in &mut self.iter {
                        unsafe {
                            pg_sys::ReleaseBuffer(pg_buffer);
                        }
                    }
                }
            }
            impl<I: Iterator<Item = pg_sys::Buffer>> Iterator for BufferIter<I> {
                type Item = pg_sys::Buffer;

                #[inline]
                fn next(&mut self) -> Option<Self::Item> {
                    self.iter.next()
                }
            }

            let rel = self.rel.as_ptr();
            let iter = (0..npages)
                .step_by(MAX_BUFFERS_TO_EXTEND_BY)
                .flat_map(move |i| {
                    let many = (npages - i).min(MAX_BUFFERS_TO_EXTEND_BY);

                    // bulk_extend_relation() returns `pg_sys::Buffer` instances that are not locked...
                    let buffers = bulk_extend_relation(rel, many);
                    buffers.into_iter().take(many)
                })
                .inspect(move |&pg_buffer| {
                    // ... so we need to lock them here
                    pg_sys::LockBuffer(pg_buffer, pg_sys::BUFFER_LOCK_EXCLUSIVE as _);
                });
            BufferIter { iter }
        }
    }

    /// Retrieve an existing [`pg_sys::Buffer`] by its number.  The returned buffer is always pinned
    /// and if `lock` is `Some`, it'll be locked with that lock level.
    pub fn get_buffer(&self, blockno: pg_sys::BlockNumber, lock: Option<u32>) -> pg_sys::Buffer {
        self.get_buffer_with_strategy(blockno, std::ptr::null_mut(), lock)
    }

    fn get_buffer_with_strategy(
        &self,
        blockno: pg_sys::BlockNumber,
        strategy: pg_sys::BufferAccessStrategy,
        lock: Option<u32>,
    ) -> pg_sys::Buffer {
        debug_assert!(
            blockno != pg_sys::InvalidBlockNumber,
            "RelationBufferAccess::get_buffer_with_strategy:  blockno is invalid"
        );
        unsafe {
            let buffer = pg_sys::ReadBufferExtended(
                self.rel.as_ptr(),
                pg_sys::ForkNumber::MAIN_FORKNUM,
                blockno,
                pg_sys::ReadBufferMode::RBM_NORMAL,
                strategy,
            );
            debug_assert!(buffer != pg_sys::InvalidBuffer as pg_sys::Buffer);
            if let Some(lock) = lock {
                pg_sys::LockBuffer(buffer, lock as i32);
            }
            buffer
        }
    }
}

/// Extend the relation by one buffer.
///
/// # Safety
///
/// Requires that the caller have the relation locked for extension with a [`pg_sys::ExclusiveLock`],
/// otherwise Postgres will trap an Assert in debug mode
#[inline]
pub unsafe fn extend_by_one_buffer(rel: pg_sys::Relation) -> pg_sys::Buffer {
    #[cfg(any(feature = "pg14", feature = "pg15"))]
    {
        pg_sys::ReadBufferExtended(
            rel,
            pg_sys::ForkNumber::MAIN_FORKNUM,
            pg_sys::InvalidBlockNumber,
            pg_sys::ReadBufferMode::RBM_NORMAL,
            determine_rel_extend_bas(),
        )
    }

    #[cfg(any(feature = "pg16", feature = "pg17"))]
    {
        pg_sys::ExtendBufferedRel(
            pg_sys::BufferManagerRelation {
                rel,
                ..Default::default()
            },
            pg_sys::ForkNumber::MAIN_FORKNUM,
            determine_rel_extend_bas(),
            // failure to clear the size cache, which is attached to `self.rel.rd_smgr` can cause
            // future reads from the relation to fail complaining that the block number returned
            // here doesn't exist because internally the Relation doesn't realize that it has
            // actually grown in size
            pg_sys::ExtendBufferedFlags::EB_CLEAR_SIZE_CACHE

                // we don't need/want this call to ExtendBufferedRel to do any relation locking
                // because we already own the lock, as the SAFETY requirement to extend_by_one_buffer
                // requires it
                | pg_sys::ExtendBufferedFlags::EB_SKIP_EXTENSION_LOCK,
        )
    }
}

/// Extend the relation by `npages`.  We'll ever extend by more than [`MAX_BUFFERS_TO_EXTEND_BY`]
/// and also won't extend by less than `npages`
unsafe fn bulk_extend_relation(
    rel: pg_sys::Relation,
    npages: usize,
) -> [pg_sys::Buffer; MAX_BUFFERS_TO_EXTEND_BY] {
    let mut buffers = [pg_sys::InvalidBuffer as pg_sys::Buffer; MAX_BUFFERS_TO_EXTEND_BY];
    assert!(
        npages <= buffers.len(),
        "requested too many pages for relation extension: npages={npages}, buffers.len={}",
        buffers.len()
    );

    #[cfg(any(feature = "pg14", feature = "pg15"))]
    {
        pg_sys::LockRelationForExtension(rel, pg_sys::ExclusiveLock as i32);
        for slot in buffers.iter_mut().take(npages) {
            let pg_buffer = extend_by_one_buffer(rel);
            debug_assert!(pg_buffer != pg_sys::InvalidBuffer as pg_sys::Buffer);
            *slot = pg_buffer;
        }
        pg_sys::UnlockRelationForExtension(rel, pg_sys::ExclusiveLock as i32);
        buffers
    }

    #[cfg(any(feature = "pg16", feature = "pg17"))]
    {
        let mut filled = 0;
        let mut extended_by = 0;
        loop {
            let bmr = pg_sys::BufferManagerRelation {
                rel,
                ..Default::default()
            };
            pg_sys::ExtendBufferedRelBy(
                bmr,
                pg_sys::ForkNumber::MAIN_FORKNUM,
                determine_rel_extend_bas(),
                // failure to clear the size cache, which is attached to `self.rel.rd_smgr` can cause
                // future reads from the relation to fail complaining that the block number returned
                // here doesn't exist because internally the Relation doesn't realize that it has
                // actually grown in size
                pg_sys::ExtendBufferedFlags::EB_CLEAR_SIZE_CACHE,
                (npages - filled) as _,
                buffers[filled..].as_mut_ptr(),
                &mut extended_by,
            );
            filled += extended_by as usize;
            extended_by = 0;
            if filled == npages {
                break;
            }
        }

        buffers
    }
}

/// Using hints from the runtime environment, determine which [`pg_sys::BufferAccessStrategy`] we
/// can use when extending a relation
#[inline(always)]
fn determine_rel_extend_bas() -> pg_sys::BufferAccessStrategy {
    struct BufferAccessStrategyHolder(pg_sys::BufferAccessStrategy);
    unsafe impl Send for BufferAccessStrategyHolder {}
    unsafe impl Sync for BufferAccessStrategyHolder {}

    static BAS_BULKWRITE: LazyLock<BufferAccessStrategyHolder> = LazyLock::new(|| {
        BufferAccessStrategyHolder(unsafe {
            // SAFETY:  Allocated in `TopMemoryContext`, once, so that it's always available
            PgMemoryContexts::TopMemoryContext.switch_to(|_| {
                pg_sys::GetAccessStrategy(pg_sys::BufferAccessStrategyType::BAS_BULKWRITE)
            })
        })
    });

    // if the backend is one of these types then we can use the BULKWRITE BufferAccessStrategy.
    unsafe {
        if pg_sys::MyBackendType == pg_sys::BackendType::B_BG_WORKER
            || pg_sys::MyBackendType == pg_sys::BackendType::B_BACKEND
        {
            BAS_BULKWRITE.0
        } else {
            // specifically, the B_AUTOVAC_WORKER backend type cannot!!
            std::ptr::null_mut()
        }
    }
}
