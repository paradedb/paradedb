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

use crate::api::operator::anyelement_query_input_opoid;
use crate::postgres::customscan::aggregatescan::{CustomScanClause, AggregateScan};
use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
use crate::postgres::customscan::builders::custom_path::{restrict_info, RestrictInfoType};
use crate::postgres::customscan::qual_inspect::{extract_quals, QualExtractState};
use crate::postgres::customscan::CustomScan;
use crate::postgres::PgSearchRelation;
use crate::query::SearchQueryInput;
use pgrx::pg_sys;

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchQueryClause {
    query: SearchQueryInput,
}

impl SearchQueryClause {
    pub fn query(&self) -> &SearchQueryInput {
        &self.query
    }

    pub fn query_mut(&mut self) -> &mut SearchQueryInput {
        &mut self.query
    }
}

impl CustomScanClause<AggregateScan> for SearchQueryClause {
    type Args = <AggregateScan as CustomScan>::Args;

    fn add_to_custom_path(
        &self,
        builder: CustomPathBuilder<AggregateScan>,
    ) -> CustomPathBuilder<AggregateScan> {
        builder
    }

    fn from_pg(
        args: &Self::Args,
        heap_rti: pg_sys::Index,
        index: &PgSearchRelation,
    ) -> Option<Self> {
        // We can't handle HAVING yet
        if args.root().hasHavingQual {
            return None;
        }

        let (restrict_info, ri_type) = restrict_info(args.input_rel());
        if matches!(ri_type, RestrictInfoType::Join) {
            // This relation is a join, or has no restrictions (WHERE clause predicates), so there's no need
            // for us to do anything.
            return None;
        }

        let has_where_clause = matches!(ri_type, RestrictInfoType::BaseRelation);
        if !has_where_clause {
            return Some(SearchQueryClause {
                query: SearchQueryInput::All,
            });
        }

        let mut where_qual_state = QualExtractState::default();
        let quals = unsafe {
            extract_quals(
                args.root,
                heap_rti,
                restrict_info.as_ptr().cast(),
                anyelement_query_input_opoid(),
                ri_type,
                index,
                false,
                &mut where_qual_state,
                true,
            )?
        };

        Some(SearchQueryClause {
            query: SearchQueryInput::from(&quals),
        })
    }
}
