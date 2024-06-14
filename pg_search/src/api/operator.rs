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

use crate::env::needs_commit;
use crate::index::state::SearchStateManager;
use crate::schema::SearchConfig;
use crate::{globals::WriterGlobal, postgres::utils::get_search_index};
use pgrx::*;
use rustc_hash::FxHashSet;

#[pg_extern]
fn search_tantivy(
    element: AnyElement,
    config_json: JsonB,
    fcinfo: pg_sys::FunctionCallInfo,
) -> bool {
    let default_hash_set = || {
        let JsonB(search_config_json) = &config_json;
        let search_config: SearchConfig = serde_json::from_value(search_config_json.clone())
            .expect("could not parse search config");

        let writer_client = WriterGlobal::client();
        let search_index = get_search_index(&search_config.index_name);
        let scan_state = search_index
            .search_state(&writer_client, &search_config, needs_commit())
            .unwrap();
        let top_docs = scan_state.search(search_index.executor);
        let mut hs = FxHashSet::default();

        for (_score, _doc_address, key, _ctid) in top_docs {
            hs.insert(key);
        }

        SearchStateManager::set_state(scan_state).expect("could not store search state in manager");
        (search_config, hs)
    };

    let cached = unsafe { pg_func_extra(fcinfo, default_hash_set) };
    let search_config = &cached.0;
    let hash_set = &cached.1;
    let key_field_name = &search_config.key_field;

    let key_field_value = match unsafe { i64::from_datum(element.datum(), false) } {
        None => panic!("no value present in key_field {key_field_name} in tuple"),
        Some(value) => value,
    };

    hash_set.contains(&key_field_value)
}

extension_sql!(
    r#"
CREATE OPERATOR pg_catalog.@@@ (
    PROCEDURE = search_tantivy,
    LEFTARG = anyelement,
    RIGHTARG = jsonb
);

CREATE OPERATOR CLASS anyelement_bm25_ops DEFAULT FOR TYPE anyelement USING bm25 AS
    OPERATOR 1 pg_catalog.@@@(anyelement, jsonb),
    STORAGE anyelement;

"#,
    name = "bm25_ops_anyelement_operator"
);
