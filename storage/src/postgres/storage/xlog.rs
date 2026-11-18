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
#![allow(unsafe_op_in_unsafe_fn)]

use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::buffer::BufferMut;
use pgrx::pg_sys;
use std::ptr::NonNull;

#[derive(Debug, Copy, Clone, Default)]
#[repr(i32)]
pub enum XlogFlag {
    #[default]
    ExistingBuffer = 0,
    NewBuffer = pg_sys::GENERIC_XLOG_FULL_IMAGE as i32,
}

impl XlogFlag {
    pub fn into_style(self, rel: &PgSearchRelation) -> XlogStyle {
        if rel.need_wal() {
            match self {
                XlogFlag::ExistingBuffer => unsafe {
                    XlogStyle::GenericXlog(GenericXlogState::Uninitialized {
                        rel: NonNull::new_unchecked(rel.as_ptr()),
                        flag: self,
                    })
                },
                XlogFlag::NewBuffer => XlogStyle::FullPageImage,
            }
        } else {
            XlogStyle::Unlogged
        }
    }
}

#[derive(Debug)]
pub enum GenericXlogState {
    Uninitialized {
        rel: NonNull<pg_sys::RelationData>,
        flag: XlogFlag,
    },
    Modified {
        pg_state: NonNull<pg_sys::GenericXLogState>,
        pg_page: pg_sys::Page,
    },
}

#[derive(Debug)]
pub enum XlogStyle {
    Unlogged,
    GenericXlog(GenericXlogState),
    FullPageImage,
}

impl XlogStyle {
    pub fn get_page(&self, buffer: pg_sys::Buffer) -> pg_sys::Page {
        unsafe {
            match self {
                XlogStyle::GenericXlog(GenericXlogState::Modified { pg_page, .. }) => *pg_page,
                _ => pgrx::BufferGetPage!(buffer).get(),
            }
        }
    }

    pub fn get_page_mut(&mut self, buffer: pg_sys::Buffer) -> pg_sys::Page {
        unsafe {
            match self {
                XlogStyle::Unlogged | XlogStyle::FullPageImage => {
                    pgrx::BufferGetPage!(buffer).get()
                }
                XlogStyle::GenericXlog(GenericXlogState::Uninitialized { rel, flag }) => {
                    let pg_state = pg_sys::GenericXLogStart(rel.as_ptr());
                    let pg_page = pg_sys::GenericXLogRegisterBuffer(pg_state, buffer, *flag as _);
                    let state = GenericXlogState::Modified {
                        pg_state: NonNull::new_unchecked(pg_state),
                        pg_page,
                    };
                    *self = XlogStyle::GenericXlog(state);
                    pg_page
                }
                XlogStyle::GenericXlog(GenericXlogState::Modified { pg_page, .. }) => *pg_page,
            }
        }
    }
}

pub unsafe fn finish_xlog(buffer: &mut BufferMut) {
    if buffer.dirty {
        match buffer.style {
            XlogStyle::Unlogged => {
                pg_sys::MarkBufferDirty(buffer.pg_buffer);
            }

            XlogStyle::FullPageImage => {
                pg_sys::CritSectionCount += 1;
                pg_sys::MarkBufferDirty(buffer.pg_buffer);
                pg_sys::log_newpage_buffer(buffer.pg_buffer, true);
                pg_sys::CritSectionCount -= 1;
            }

            XlogStyle::GenericXlog(GenericXlogState::Uninitialized { .. }) => {
                // noop
            }

            XlogStyle::GenericXlog(GenericXlogState::Modified { pg_state, .. }) => {
                pg_sys::GenericXLogFinish(pg_state.as_ptr());
            }
        }
    } else {
        match buffer.style {
            XlogStyle::GenericXlog(GenericXlogState::Modified { pg_state, .. }) => {
                pg_sys::GenericXLogAbort(pg_state.as_ptr());
            }
            _ => {
                // noop
            }
        }
    }
}

/// Log a snapshot conflict before publishing index contents that omit dead heap versions.
///
/// # Safety
/// Must run in a PostgreSQL backend with an open index relation. The horizon must come from
/// dead heap versions omitted by the replacement being published.
pub unsafe fn log_snapshot_conflict(indexrel: &PgSearchRelation, horizon: pg_sys::TransactionId) {
    let xid = horizon;
    if xid == pg_sys::InvalidTransactionId
        || !indexrel.need_wal()
        || pg_sys::wal_level < pg_sys::WalLevel::WAL_LEVEL_REPLICA as i32
    {
        return;
    }
    #[cfg(any(feature = "pg16", feature = "pg17", feature = "pg18"))]
    let heaprel = indexrel
        .heap_relation()
        .expect("Should have a heap relation.");
    let next_xid = pg_sys::ReadNextFullTransactionId();
    // The removed XID is in this epoch or the preceding one. Mirror
    // FullTransactionIdFromAllowableAt, also supporting PostgreSQL 15/16.
    let mut epoch = (next_xid.value >> 32) as u32;
    if xid.into_inner() > next_xid.value as u32 {
        epoch -= 1;
    }
    let full_xid = pg_sys::FullTransactionIdFromEpochAndXid(epoch, xid);
    // TODO: Add a dedicated snapshot-conflict record in custom_rmgr.rs that calls
    // ResolveRecoveryConflictWithSnapshot directly, without FullTransactionId epoch
    // conversion or the gist_private.h dependency.
    // GiST's PAGE_REUSE record only resolves snapshot conflicts during redo.
    // It registers no buffer and does not read or modify any index page.
    #[cfg(feature = "pg15")]
    pg_sys::gistXLogPageReuse(indexrel.as_ptr(), pg_sys::InvalidBlockNumber, full_xid);
    #[cfg(any(feature = "pg16", feature = "pg17", feature = "pg18"))]
    pg_sys::gistXLogPageReuse(
        indexrel.as_ptr(),
        heaprel.as_ptr(),
        pg_sys::InvalidBlockNumber,
        full_xid,
    );
}
