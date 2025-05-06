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

use std::sync::Arc;

use crate::index::fast_fields_helper::FFType;
use crate::index::reader::index::FastFieldCache;
use crate::postgres::visibility_checker::VisibilityChecker;

use parking_lot::Mutex;

use tantivy::collector::{Acceptor, TopNAcceptor};
use tantivy::{DocAddress, DocId, Searcher, SegmentOrdinal};

#[derive(Clone)]
pub struct MvccAcceptor {
    searcher: Searcher,
    ff_lookup: Arc<Mutex<FastFieldCache>>,
    checker: VisibilityChecker,
}

impl MvccAcceptor {
    pub fn new(searcher: Searcher, checker: VisibilityChecker) -> Self {
        Self {
            searcher,
            ff_lookup: Default::default(),
            checker,
        }
    }
}

impl Acceptor for MvccAcceptor {
    type Child = MvccSegmentAcceptor;

    fn for_segment(&self, segment_ord: SegmentOrdinal) -> Self::Child {
        MvccSegmentAcceptor {
            acceptor: self.clone(),
            ctid_ff: FFType::new_ctid(self.searcher.segment_reader(segment_ord).fast_fields()),
        }
    }
}

impl TopNAcceptor<DocAddress> for MvccAcceptor {
    fn accept(&mut self, doc_address: &DocAddress) -> bool {
        let ctid = {
            let mut ff_lookup = self.ff_lookup.lock();
            ff_lookup
                .entry(doc_address.segment_ord)
                .or_insert_with(|| {
                    FFType::new_ctid(
                        self.searcher
                            .segment_reader(doc_address.segment_ord)
                            .fast_fields(),
                    )
                })
                .as_u64(doc_address.doc_id)
                .expect("ctid should be present")
        };

        self.checker.is_visible(ctid)
    }
}

#[derive(Clone)]
pub struct MvccSegmentAcceptor {
    acceptor: MvccAcceptor,
    ctid_ff: FFType,
}

impl TopNAcceptor<DocId> for MvccSegmentAcceptor {
    fn accept(&mut self, doc_id: &DocId) -> bool {
        let ctid = self
            .ctid_ff
            .as_u64(*doc_id)
            .expect("ctid should be present");

        self.acceptor.checker.is_visible(ctid)
    }
}
