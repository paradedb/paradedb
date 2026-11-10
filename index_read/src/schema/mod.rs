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

pub use storage::schema::*;

use crate::query::{QueryError, SearchQueryInput, pdb_query::pdb};

pub trait SearchIndexSchemaQueryExt {
    fn null_guard(&self, query: &SearchQueryInput) -> Option<SearchQueryInput>;
}

impl SearchIndexSchemaQueryExt for SearchIndexSchema {
    /// Returns an additional existence check without replacing the original query.
    ///
    /// For a scalar fast field, `color @@@ 'blue'` gets an `exists(color)` guard.
    /// Both 'red' and NULL fail to match 'blue', but the guard distinguishes them:
    /// 'red' exists, so the predicate returns FALSE; NULL is missing, so it returns NULL.
    ///
    /// A compound search query such as `exists(color) AND NOT term(color, 'red')`
    /// gets no guard: a missing color makes that whole query FALSE, not NULL.
    fn null_guard(&self, query: &SearchQueryInput) -> Option<SearchQueryInput> {
        let field = match query {
            SearchQueryInput::WithIndex { query, .. }
            | SearchQueryInput::Boost { query, .. }
            | SearchQueryInput::ConstScore { query, .. } => return self.null_guard(query),
            SearchQueryInput::FieldedQuery { field, .. } if !query.is_exists() => field,
            _ => return None,
        };
        let search_field = self.search_field(field)?;
        if !search_field.is_fast() {
            return None;
        }

        // Empty arrays and JSON can have no indexed values without being SQL NULL.
        let root = search_field.field_name().root();
        if self
            .categorized_fields()
            .iter()
            .find(|(field, _)| field.field_name().root() == root)
            .is_some_and(|(_, data)| data.is_array || data.is_json)
        {
            return None;
        }

        Some(SearchQueryInput::FieldedQuery {
            field: field.root().into(),
            query: pdb::Query::Exists,
        })
    }
}

pub trait SearchFieldQueryExt: Sized {
    fn with_positions(self) -> Result<Self, QueryError>;
}

impl SearchFieldQueryExt for SearchField {
    fn with_positions(self) -> Result<Self, QueryError> {
        if self.supports_positions() {
            Ok(self)
        } else {
            let tokenizer = self
                .field_config()
                .tokenizer()
                .map(|t| t.name().to_string());

            Err(QueryError::TokenizerDoesNotSupportQueryType {
                field: self.field_name().clone(),
                tokenizer,
            })
        }
    }
}
