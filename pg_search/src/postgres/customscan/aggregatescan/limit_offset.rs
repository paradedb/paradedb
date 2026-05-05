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

use crate::gucs;
use crate::postgres::customscan::aggregatescan::{
    AggregateScan, CustomScanBuildError, CustomScanClause,
};
use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
use crate::postgres::customscan::limit_offset::LimitOffset;
use crate::postgres::customscan::CustomScan;
use crate::postgres::rel::PgSearchRelation;
use pgrx::pg_sys;

impl CustomScanClause<AggregateScan> for Option<LimitOffset> {
    type Args = <AggregateScan as CustomScan>::Args;

    fn add_to_custom_path(
        &self,
        builder: CustomPathBuilder<AggregateScan>,
    ) -> CustomPathBuilder<AggregateScan> {
        builder
    }

    fn explain_output(&self) -> Box<dyn Iterator<Item = (String, String)>> {
        let mut output = Vec::new();
        if let Some(lo) = self {
            output.push((String::from("Limit"), lo.limit.to_string()));
            if let Some(off) = &lo.offset {
                output.push((String::from("Offset"), off.to_string()));
            }
        }
        Box::new(output.into_iter())
    }

    fn from_pg(
        args: &Self::Args,
        _heap_rti: pg_sys::Index,
        _index: &PgSearchRelation,
    ) -> Result<Self, CustomScanBuildError> {
        let parse = args.root().parse;
        let limit_offset = unsafe { LimitOffset::from_parse(parse) };

        // Bucket-limit guard only applies when the row count is statically
        // known. Parameterized LIMIT/OFFSET values flow through unchecked at
        // planning time; the bucket cap is enforced lazily by Tantivy at
        // execution time.
        if let Some(fetch) = limit_offset.as_ref().and_then(|lo| lo.static_fetch()) {
            unsafe {
                if !(*parse).groupClause.is_null() && fetch > gucs::max_term_agg_buckets() as usize
                {
                    return Err("limit + offset exceeds max_term_agg_buckets".into());
                }
            }
        }

        Ok(limit_offset)
    }
}
