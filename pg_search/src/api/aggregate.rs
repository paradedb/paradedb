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

//! Aggregate functions for ParadeDB search.

// Older upgrade scripts create this C function before the final migration drops
// it. PostgreSQL validates the exported symbol even during extension upgrades.
#[pgrx::pg_extern(sql = false)]
fn aggregate() -> pgrx::JsonB {
    pgrx::error!("paradedb.aggregate() has been removed. Use pdb.agg() instead.")
}

#[pgrx::pg_schema]
mod pdb {
    use pgrx::aggregate::{Aggregate, ParallelOption};
    use pgrx::{Internal, JsonB, pg_extern};

    #[derive(pgrx::AggregateName, Default)]
    #[aggregate_name = "agg"]
    pub struct AggPlaceholder;

    #[pgrx::pg_aggregate]
    impl Aggregate<AggPlaceholder> for AggPlaceholder {
        const PARALLEL: Option<ParallelOption> = Some(ParallelOption::Safe);
        type Args = JsonB;
        type State = Internal;
        type Finalize = JsonB;

        fn state(
            _current: Self::State,
            _arg: Self::Args,
            _fcinfo: pgrx::pg_sys::FunctionCallInfo,
        ) -> Self::State {
            pgrx::error!(
                "pdb.agg() must be handled by ParadeDB's custom scan. \
             This error usually means the query syntax is not supported. \
             Try adding '@@@ pdb.all()' to your WHERE clause to force custom scan usage, \
             or file an issue at https://github.com/paradedb/paradedb/issues if this should be supported."
            )
        }

        fn finalize(
            _current: Self::State,
            _direct_arg: Self::OrderedSetArgs,
            _fcinfo: pgrx::pg_sys::FunctionCallInfo,
        ) -> Self::Finalize {
            pgrx::error!(
                "pdb.agg() must be handled by ParadeDB's custom scan. \
             This error usually means the query syntax is not supported. \
             Try adding '@@@ paradedb.all()' to your WHERE clause to force custom scan usage, \
             or file an issue at https://github.com/paradedb/paradedb/issues if this should be supported."
            )
        }
    }

    #[derive(pgrx::AggregateName, Default)]
    #[aggregate_name = "agg"]
    pub struct AggPlaceholderWithMvcc;

    #[pgrx::pg_aggregate]
    impl Aggregate<AggPlaceholderWithMvcc> for AggPlaceholderWithMvcc {
        const PARALLEL: Option<ParallelOption> = Some(ParallelOption::Safe);
        type Args = (JsonB, bool);
        type State = Internal;
        type Finalize = JsonB;

        fn state(
            _current: Self::State,
            _arg: Self::Args,
            _fcinfo: pgrx::pg_sys::FunctionCallInfo,
        ) -> Self::State {
            pgrx::error!(
                "pdb.agg() must be handled by ParadeDB's custom scan. \
             This error usually means the query syntax is not supported. \
             Try adding '@@@ pdb.all()' to your WHERE clause to force custom scan usage, \
             or file an issue at https://github.com/paradedb/paradedb/issues if this should be supported."
            )
        }

        fn finalize(
            _current: Self::State,
            _direct_arg: Self::OrderedSetArgs,
            _fcinfo: pgrx::pg_sys::FunctionCallInfo,
        ) -> Self::Finalize {
            pgrx::error!(
                "pdb.agg() must be handled by ParadeDB's custom scan. \
             This error usually means the query syntax is not supported. \
             Try adding '@@@ paradedb.all()' to your WHERE clause to force custom scan usage, \
             or file an issue at https://github.com/paradedb/paradedb/issues if this should be supported."
            )
        }
    }

    #[derive(pgrx::AggregateName, Default)]
    #[aggregate_name = "agg"]
    pub struct AggPlaceholderVisibility;

    #[pgrx::pg_aggregate]
    impl Aggregate<AggPlaceholderVisibility> for AggPlaceholderVisibility {
        const PARALLEL: Option<ParallelOption> = Some(ParallelOption::Safe);
        type Args = (JsonB, String);
        type State = Internal;
        type Finalize = JsonB;

        fn state(
            _current: Self::State,
            _arg: Self::Args,
            _fcinfo: pgrx::pg_sys::FunctionCallInfo,
        ) -> Self::State {
            pgrx::error!(
                "pdb.agg() must be handled by ParadeDB's custom scan. \
             This error usually means the query syntax is not supported. \
             Try adding '@@@ pdb.all()' to your WHERE clause to force custom scan usage, \
             or file an issue at https://github.com/paradedb/paradedb/issues if this should be supported."
            )
        }

        fn finalize(
            _current: Self::State,
            _direct_arg: Self::OrderedSetArgs,
            _fcinfo: pgrx::pg_sys::FunctionCallInfo,
        ) -> Self::Finalize {
            pgrx::error!(
                "pdb.agg() must be handled by ParadeDB's custom scan. \
             This error usually means the query syntax is not supported. \
             Try adding '@@@ paradedb.all()' to your WHERE clause to force custom scan usage, \
             or file an issue at https://github.com/paradedb/paradedb/issues if this should be supported."
            )
        }
    }

    #[derive(pgrx::AggregateName, Default)]
    #[aggregate_name = "combine_agg"]
    pub struct CombineAggStub;

    #[pgrx::pg_aggregate]
    impl Aggregate<CombineAggStub> for CombineAggStub {
        const PARALLEL: Option<ParallelOption> = Some(ParallelOption::Safe);
        type Args = JsonB;
        type State = Internal;
        type Finalize = JsonB;

        fn state(
            _current: Self::State,
            _arg: Self::Args,
            _fcinfo: pgrx::pg_sys::FunctionCallInfo,
        ) -> Self::State {
            pgrx::error!(
                "`pdb.combine_agg()` must be used as `pdb.combine_agg(pdb.agg(...)) OVER ()` \
                 in a `GROUP BY` query handled by ParadeDB's custom scan."
            )
        }

        fn finalize(
            _current: Self::State,
            _direct_arg: Self::OrderedSetArgs,
            _fcinfo: pgrx::pg_sys::FunctionCallInfo,
        ) -> Self::Finalize {
            pgrx::error!(
                "`pdb.combine_agg()` must be used as `pdb.combine_agg(pdb.agg(...)) OVER ()` \
                 in a `GROUP BY` query handled by ParadeDB's custom scan."
            )
        }
    }

    #[pg_extern(volatile, parallel_safe, name = "agg_fn")]
    pub fn agg_fn_placeholder(_agg_name: &str) -> JsonB {
        pgrx::error!(
            "pdb.agg_fn() placeholder should not be executed - \
             custom scan should have intercepted this."
        )
    }
}
