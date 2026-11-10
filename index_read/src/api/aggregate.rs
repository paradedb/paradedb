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

pub use common::api::aggregate::*;

use crate::api::operator::estimate_matching_rows;
use crate::gucs;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::rel::PgSearchRelation;
use crate::query::SearchQueryInput;

pub trait MvccVisibilityExt {
    fn resolve_filtering_for_sources<'a>(
        &self,
        sources: impl IntoIterator<
            Item = (
                &'a PgSearchRelation,
                &'a SearchQueryInput,
                Option<&'a SearchIndexReader>,
            ),
        >,
    ) -> bool;

    fn resolve_filtering(
        &self,
        indexrel: &PgSearchRelation,
        query: &SearchQueryInput,
        reader: Option<&SearchIndexReader>,
    ) -> bool;
}

impl MvccVisibilityExt for MvccVisibility {
    fn resolve_filtering_for_sources<'a>(
        &self,
        sources: impl IntoIterator<
            Item = (
                &'a PgSearchRelation,
                &'a SearchQueryInput,
                Option<&'a SearchIndexReader>,
            ),
        >,
    ) -> bool {
        match self {
            MvccVisibility::Transaction => true,
            MvccVisibility::Raw => false,
            MvccVisibility::Threshold => {
                let mut largest = 0;
                for (indexrel, query, reader) in sources {
                    if query.has_heap_filters() || query.has_postgres_expressions() {
                        return true;
                    }
                    match estimate_matching_rows(indexrel, query.clone(), reader) {
                        Some(rows) => largest = largest.max(rows),
                        None => return true,
                    }
                }
                largest < gucs::visibility_threshold()
            }
        }
    }

    fn resolve_filtering(
        &self,
        indexrel: &PgSearchRelation,
        query: &SearchQueryInput,
        reader: Option<&SearchIndexReader>,
    ) -> bool {
        self.resolve_filtering_for_sources(std::iter::once((indexrel, query, reader)))
    }
}
