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

pub mod aggregate_type;
pub mod build;
pub mod exec;
pub mod explain;
pub mod filterquery;
pub mod groupby;
pub mod join_targetlist;
pub mod json_rewrite;
pub mod limit_offset;
pub mod orderby;
pub mod searchquery;
pub mod targetlist;

pub use aggregate_type::AggregateType;
pub use groupby::{GroupByClause, GroupingColumn};
pub use targetlist::TargetListEntry;

use crate::postgres::customscan::CustomScan;
use crate::postgres::customscan::builders::custom_path::CustomPathBuilder;
use crate::postgres::customscan::explainer::Explainer;
use crate::postgres::customscan::pullup::SourceContext;

/// Why AggregateScan cannot preserve PostgreSQL's grouping semantics.
///
/// These are planner-level eligibility failures: plain aggregates fall back to
/// PostgreSQL, while `pdb.agg()` queries error at plan time with the reason.
pub enum GroupingPushdownDeclineReason {
    GroupingSets,
    MissingPathKeys,
    NondeterministicCollation,
    NondeterministicAggregateKey,
    // TODO(https://github.com/paradedb/paradedb/issues/7187): restore `GROUP BY unnest(...)`
    // pushdown once fast fields preserve inner array NULLs and empty arrays.
    UnnestNullElements,
}

impl GroupingPushdownDeclineReason {
    pub fn detail(&self) -> &'static str {
        match self {
            Self::GroupingSets => "GROUPING SETS are not supported",
            Self::MissingPathKeys => "could not verify GROUP BY semantics",
            Self::NondeterministicCollation => "GROUP BY uses a nondeterministic collation",
            Self::NondeterministicAggregateKey => {
                "an aggregate has a DISTINCT or ORDER BY key with a nondeterministic collation"
            }
            Self::UnnestNullElements => {
                "GROUP BY unnest() needs the NULL elements of the array, which the index does not store"
            }
        }
    }
}

/// A collection of index information that is necessary for making result-rewriting decisions
pub struct AggIndexInfo {
    pub created_by_version: Option<crate::api::version::Version>,
    pub schema: crate::schema::SearchIndexSchema,
}
impl From<&crate::postgres::rel::PgSearchRelation> for AggIndexInfo {
    fn from(value: &crate::postgres::rel::PgSearchRelation) -> Self {
        Self {
            created_by_version: value.created_by_version(),
            schema: value.schema().expect("schema should be initialized by now"),
        }
    }
}

pub enum CustomScanBuildError {
    NotInteresting,
    Incompatible(String),
}

impl From<String> for CustomScanBuildError {
    fn from(s: String) -> Self {
        CustomScanBuildError::Incompatible(s)
    }
}

impl From<&str> for CustomScanBuildError {
    fn from(s: &str) -> Self {
        CustomScanBuildError::Incompatible(s.to_string())
    }
}

pub trait CustomScanClause {
    type Args;

    fn from_pg(args: &Self::Args, context: &SourceContext) -> Result<Self, CustomScanBuildError>
    where
        Self: Sized;

    fn add_to_custom_path<CS: CustomScan<Args = Self::Args>>(
        &self,
        builder: CustomPathBuilder<CS>,
    ) -> CustomPathBuilder<CS>;

    fn explain_output(&self) -> Box<dyn Iterator<Item = (String, String)>> {
        Box::new(std::iter::empty())
    }

    fn add_to_explainer(&self, explainer: &mut Explainer) {
        for (key, value) in self.explain_output() {
            explainer.add_text(&format!("  {}", key), &value);
        }
    }

    fn build<CS: CustomScan<Args = Self::Args>>(
        builder: CustomPathBuilder<CS>,
        context: &SourceContext,
    ) -> Result<(CustomPathBuilder<CS>, Self), CustomScanBuildError>
    where
        Self: Sized,
    {
        let clause = Self::from_pg(builder.args(), context)?;
        let builder = clause.add_to_custom_path(builder);
        Ok((builder, clause))
    }
}
