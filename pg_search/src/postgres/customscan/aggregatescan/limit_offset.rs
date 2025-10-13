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

use crate::nodecast;
use crate::postgres::customscan::aggregatescan::AggregateClause;
use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
use crate::postgres::customscan::CreateUpperPathsHookArgs;
use crate::postgres::customscan::CustomScan;
use crate::postgres::PgSearchRelation;
use pgrx::pg_sys;
use pgrx::FromDatum;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LimitOffsetClause {
    limit: Option<u32>,
    offset: Option<u32>,
}

impl LimitOffsetClause {
    pub fn limit(&self) -> Option<u32> {
        self.limit
    }
    pub fn offset(&self) -> Option<u32> {
        self.offset
    }
}

impl AggregateClause for LimitOffsetClause {
    fn add_to_custom_path<CS>(&self, builder: CustomPathBuilder<CS>) -> CustomPathBuilder<CS>
    where
        CS: CustomScan,
    {
        builder
    }

    fn from_pg(
        args: &CreateUpperPathsHookArgs,
        heap_rti: pg_sys::Index,
        index: &PgSearchRelation,
    ) -> Option<Self> {
        let parse = args.root().parse;
        let (limit, offset) = unsafe {
            let limit_count = (*parse).limitCount;
            let offset_count = (*parse).limitOffset;

            let extract_const = |node: *mut pg_sys::Node| -> Option<u32> {
                let const_node = nodecast!(Const, T_Const, node);
                if let Some(const_node) = const_node {
                    u32::from_datum((*const_node).constvalue, (*const_node).constisnull)
                } else {
                    None
                }
            };

            (extract_const(limit_count), extract_const(offset_count))
        };

        Some(LimitOffsetClause { limit, offset })
    }
}
