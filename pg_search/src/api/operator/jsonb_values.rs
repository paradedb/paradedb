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

#[pgrx::pg_schema]
mod pdb {
    use pgrx::{extension_sql, pg_extern, InOutFuncs, Json, JsonB, PostgresType, StringInfo};
    use serde::{Deserialize, Serialize};
    use std::ffi::CStr;

    #[allow(non_camel_case_types)]
    #[derive(Debug, Clone, PostgresType, Serialize, Deserialize)]
    #[inoutfuncs]
    #[repr(transparent)]
    pub struct jsonb_values(i32);

    impl InOutFuncs for jsonb_values {
        fn input(_input: &CStr) -> Self {
            jsonb_values(0)
        }

        fn output(&self, buffer: &mut StringInfo) {
            buffer.push_str("jsonb_values");
        }
    }

    #[pg_extern(immutable, parallel_safe)]
    pub fn jsonb_to_jsonb_values(_input: JsonB) -> jsonb_values {
        pgrx::error!(
            "pdb.jsonb_values requires a BM25 index with jsonb_values_paths configured.\n\
             HINT: 1) Ensure a BM25 index exists on this JSON column\n\
                   2) Configure jsonb_values_paths: ALTER INDEX idx SET (jsonb_values_paths = '{{...}}')\n\
                   3) Use explicit cast syntax: column::pdb.jsonb_values"
        );
    }

    #[pg_extern(immutable, parallel_safe)]
    pub fn json_to_jsonb_values(_input: Json) -> jsonb_values {
        pgrx::error!(
            "pdb.jsonb_values requires a BM25 index with jsonb_values_paths configured.\n\
             HINT: 1) Ensure a BM25 index exists on this JSON column\n\
                   2) Configure jsonb_values_paths: ALTER INDEX idx SET (jsonb_values_paths = '{{...}}')\n\
                   3) Use explicit cast syntax: column::pdb.jsonb_values"
        );
    }

    extension_sql!(
        r#"
            CREATE CAST (jsonb AS pdb.jsonb_values)
            WITH FUNCTION pdb.jsonb_to_jsonb_values(jsonb);

            CREATE CAST (json AS pdb.jsonb_values)
            WITH FUNCTION pdb.json_to_jsonb_values(json);
        "#,
        name = "jsonb_values_casts",
        requires = [jsonb_to_jsonb_values, json_to_jsonb_values]
    );
}
