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
    Started {
        flag: XlogFlag,
        pg_state: NonNull<pg_sys::GenericXLogState>,
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
    pub unsafe fn start_generic(rel: pg_sys::Relation) -> XlogStyle {
        let pg_state = pg_sys::GenericXLogStart(rel);
        let state = GenericXlogState::Started {
            flag: XlogFlag::ExistingBuffer,
            pg_state: NonNull::new_unchecked(pg_state),
        };
        XlogStyle::GenericXlog(state)
    }

    pub fn get_page_mut(&mut self, buffer: pg_sys::Buffer) -> pg_sys::Page {
        unsafe {
            match self {
                XlogStyle::Unlogged | XlogStyle::FullPageImage => pg_sys::BufferGetPage(buffer),
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
                XlogStyle::GenericXlog(GenericXlogState::Started { flag, pg_state }) => {
                    let pg_page =
                        pg_sys::GenericXLogRegisterBuffer(pg_state.as_ptr(), buffer, *flag as _);
                    let state = GenericXlogState::Modified {
                        pg_state: *pg_state,
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
                pg_sys::MarkBufferDirty(buffer.pg_buffer);
                pg_sys::CritSectionCount += 1;
                pg_sys::log_newpage_buffer(buffer.pg_buffer, true);
                pg_sys::CritSectionCount -= 1;
            }

            XlogStyle::GenericXlog(GenericXlogState::Uninitialized { .. }) => {
                // noop
            }

            XlogStyle::GenericXlog(GenericXlogState::Started { .. }) => {
                // BufferMut started a generic xlog record but never got the page and modified it
                // noop
            }

            XlogStyle::GenericXlog(GenericXlogState::Modified { pg_state, .. }) => {
                pg_sys::GenericXLogFinish(pg_state.as_ptr());
            }
        }
    } else {
        match buffer.style {
            XlogStyle::GenericXlog(
                GenericXlogState::Started { pg_state, .. }
                | GenericXlogState::Modified { pg_state, .. },
            ) => {
                pg_sys::GenericXLogAbort(pg_state.as_ptr());
            }
            _ => {
                // noop
            }
        }
    }
}
