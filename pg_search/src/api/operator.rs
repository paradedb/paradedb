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

mod searchconfig;
mod searchqueryinput;
mod text;

use crate::env::needs_commit;
use crate::globals::WriterGlobal;
use crate::index::SearchIndex;
use crate::postgres::types::TantivyValue;
use crate::postgres::utils::locate_bm25_index;
use crate::schema::SearchConfig;
use crate::writer::WriterDirectory;
use pgrx::callconv::{BoxRet, FcInfo};
use pgrx::datum::Datum;
use pgrx::pg_sys::planner_rt_fetch;
use pgrx::pgrx_sql_entity_graph::metadata::{
    ArgumentError, Returns, ReturnsError, SqlMapping, SqlTranslatable,
};
use pgrx::*;
use rustc_hash::FxHashSet;
use std::ptr::NonNull;

const UNKNOWN_SELECTIVITY: f64 = 0.00001;

#[repr(transparent)]
pub struct ReturnedNodePointer(Option<NonNull<pg_sys::Node>>);

unsafe impl BoxRet for ReturnedNodePointer {
    unsafe fn box_into<'fcx>(self, fcinfo: &mut FcInfo<'fcx>) -> Datum<'fcx> {
        self.0
            .map(|nonnull| {
                let datum = pg_sys::Datum::from(nonnull.as_ptr());
                fcinfo.return_raw_datum(datum)
            })
            .unwrap_or_else(|| Datum::null())
    }
}

unsafe impl SqlTranslatable for ReturnedNodePointer {
    fn argument_sql() -> Result<SqlMapping, ArgumentError> {
        Ok(SqlMapping::As("internal".into()))
    }

    fn return_sql() -> Result<Returns, ReturnsError> {
        Ok(Returns::One(SqlMapping::As("internal".into())))
    }
}

fn anyelement_jsonb_procoid() -> pg_sys::Oid {
    unsafe {
        direct_function_call::<pg_sys::Oid>(
            pg_sys::regprocedurein,
            &[c"paradedb.search_with_search_config(anyelement, jsonb)".into_datum()],
        )
        .expect("the `paradedb.search_with_search_config(anyelement, jsonb) function should exist")
    }
}
fn anyelement_text_opoid() -> pg_sys::Oid {
    unsafe {
        direct_function_call::<pg_sys::Oid>(
            pg_sys::regoperatorin,
            &[c"@@@(anyelement, text)".into_datum()],
        )
        .expect("the `@@@(anyelement, text)` operator should exist")
    }
}

fn anyelement_jsonb_opoid() -> pg_sys::Oid {
    unsafe {
        direct_function_call::<pg_sys::Oid>(
            pg_sys::regoperatorin,
            &[c"@@@(anyelement, jsonb)".into_datum()],
        )
        .expect("the `@@@(anyelement, jsonb)` operator should exist")
    }
}

fn estimate_selectivity(heaprelid: pg_sys::Oid, search_config: &SearchConfig) -> Option<f64> {
    let directory = WriterDirectory::from_index_oid(search_config.index_oid);
    let search_index = SearchIndex::from_cache(&directory, &search_config.uuid)
        .unwrap_or_else(|err| panic!("error loading index from directory: {err}"));
    let writer_client = WriterGlobal::client();
    let state = search_index
        .search_state(&writer_client, &search_config, false)
        .expect("SearchState creation should not fail");

    let reltuples = unsafe { PgRelation::open(heaprelid).reltuples().unwrap_or(1.0) as f64 };
    let estimate = state.estimate_docs().unwrap_or(1) as f64;

    let mut selectivity = estimate / reltuples;
    if selectivity > 1.0 {
        selectivity = 1.0;
    }

    Some(selectivity)
}

extension_sql!(
    r#"
ALTER FUNCTION paradedb.search_with_search_config SUPPORT paradedb.search_config_support;
ALTER FUNCTION paradedb.search_with_text SUPPORT paradedb.text_support;

CREATE OPERATOR pg_catalog.@@@ (
    PROCEDURE = search_with_search_config,
    LEFTARG = anyelement,
    RIGHTARG = jsonb,
    RESTRICT = search_config_restrict
);

CREATE OPERATOR pg_catalog.@@@ (
    PROCEDURE = search_with_text,
    LEFTARG = anyelement,
    RIGHTARG = text,
    RESTRICT = text_restrict
);

CREATE OPERATOR CLASS anyelement_bm25_ops DEFAULT FOR TYPE anyelement USING bm25 AS
    OPERATOR 1 pg_catalog.@@@(anyelement, jsonb),                        /* for querying with a full SearchConfig jsonb object */
    OPERATOR 2 pg_catalog.@@@(anyelement, text),                         /* for querying with a tantivy-compatible text query */
    STORAGE anyelement;

"#,
    name = "bm25_ops_anyelement_operator",
    requires = [
        // for using a SearchConfig on the rhs
        searchconfig::search_with_search_config,
        searchconfig::search_config_restrict,
        searchconfig::search_config_support,
        // for using plain text on the rhs
        text::search_with_text,
        text::text_restrict,
        text::text_support,
    ]
);
