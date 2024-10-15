// Copyright (c) 2023-2024 Retake, Inc.
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

use crate::index::reader::{SearchIndexReader, SearchResults};
use crate::postgres::customscan::builders::custom_state::CustomScanStateWrapper;
use crate::postgres::customscan::pdbscan::exec_methods::ExecState;
use crate::postgres::customscan::pdbscan::projections::snippet::SnippetInfo;
use crate::postgres::customscan::pdbscan::PdbScan;
use crate::postgres::customscan::CustomScanState;
use crate::postgres::visibility_checker::VisibilityChecker;
use crate::schema::SearchConfig;
use pgrx::{name_data_to_str, pg_sys};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use tantivy::query::Query;
use tantivy::snippet::SnippetGenerator;

const SORT_ASCENDING: u32 = pg_sys::BTLessStrategyNumber;
const SORT_DESCENDING: u32 = pg_sys::BTGreaterStrategyNumber;

#[derive(Debug, Default, Copy, Clone)]
#[repr(u32)]
pub enum SortDirection {
    #[default]
    Asc = pg_sys::BTLessStrategyNumber,
    Desc = pg_sys::BTGreaterStrategyNumber,
}

impl AsRef<str> for SortDirection {
    fn as_ref(&self) -> &str {
        match self {
            SortDirection::Asc => "asc",
            SortDirection::Desc => "desc",
        }
    }
}

impl Display for SortDirection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

impl From<SortDirection> for crate::index::reader::SortDirection {
    fn from(value: SortDirection) -> Self {
        match value {
            SortDirection::Asc => crate::index::reader::SortDirection::Asc,
            SortDirection::Desc => crate::index::reader::SortDirection::Desc,
        }
    }
}

impl From<i32> for SortDirection {
    fn from(value: i32) -> Self {
        SortDirection::from(value as u32)
    }
}

impl From<u32> for SortDirection {
    fn from(value: u32) -> Self {
        match value {
            pg_sys::BTLessStrategyNumber => SortDirection::Asc,
            pg_sys::BTGreaterStrategyNumber => SortDirection::Desc,
            _ => panic!("unrecognized sort strategy number: {value}"),
        }
    }
}

impl From<SortDirection> for u32 {
    fn from(value: SortDirection) -> Self {
        value as _
    }
}

#[derive(Default)]
pub struct PdbScanState {
    pub heaprelid: pg_sys::Oid,
    pub indexrelid: pg_sys::Oid,
    pub rti: pg_sys::Index,

    pub index_name: String,
    pub index_uuid: String,
    pub key_field: String,

    pub query: Option<Box<dyn Query>>,
    pub search_config: SearchConfig,
    pub search_reader: Option<SearchIndexReader>,

    pub search_results: SearchResults,

    pub limit: Option<usize>,
    pub sort_direction: Option<SortDirection>,
    pub retry_count: usize,
    pub invisible_tuple_count: usize,

    pub heaprel: Option<pg_sys::Relation>,
    pub indexrel: Option<pg_sys::Relation>,
    pub lockmode: pg_sys::LOCKMODE,

    pub snapshot: Option<pg_sys::Snapshot>,
    pub visibility_checker: Option<VisibilityChecker>,

    pub need_scores: bool,
    pub snippet_generators: HashMap<SnippetInfo, Option<SnippetGenerator>>,
    pub score_funcoid: pg_sys::Oid,
    pub snippet_funcoid: pg_sys::Oid,
    pub var_attname_lookup: HashMap<(i32, pg_sys::AttrNumber), String>,

    pub scan_func:
        Option<fn(&mut CustomScanStateWrapper<PdbScan>, *mut std::ffi::c_void) -> ExecState>,
    pub inner_scan_state: Option<*mut std::ffi::c_void>,
}

impl CustomScanState for PdbScanState {}

impl PdbScanState {
    #[inline(always)]
    pub fn need_scores(&self) -> bool {
        self.need_scores
    }

    #[inline(always)]
    pub fn need_snippets(&self) -> bool {
        !self.snippet_generators.is_empty()
    }

    #[inline(always)]
    pub fn snapshot(&self) -> pg_sys::Snapshot {
        self.snapshot.unwrap()
    }

    #[inline(always)]
    pub fn heaprel(&self) -> pg_sys::Relation {
        self.heaprel.unwrap()
    }

    #[inline(always)]
    pub fn heaprelname(&self) -> &str {
        unsafe { name_data_to_str(&(*(*self.heaprel()).rd_rel).relname) }
    }

    #[inline(always)]
    pub fn heaptupdesc(&self) -> pg_sys::TupleDesc {
        unsafe { (*self.heaprel()).rd_att }
    }

    #[inline(always)]
    pub fn visibility_checker(&mut self) -> &mut VisibilityChecker {
        self.visibility_checker.as_mut().unwrap()
    }
}
