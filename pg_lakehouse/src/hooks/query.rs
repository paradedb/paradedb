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

use pgrx::*;
use std::ffi::CStr;
use std::str::Utf8Error;

use crate::fdw::handler::FdwHandler;

#[allow(dead_code)]
#[derive(PartialEq, Clone)]
pub enum QueryType {
    Federated,
    DataFusion,
    Postgres,
}

pub fn get_current_query(
    planned_stmt: *mut pg_sys::PlannedStmt,
    query_string: &CStr,
) -> Result<String, Utf8Error> {
    let query_start_index = unsafe { (*planned_stmt).stmt_location };
    let query_len = unsafe { (*planned_stmt).stmt_len };
    let full_query = query_string.to_str()?;

    let current_query = if query_start_index != -1 {
        if query_len == 0 {
            full_query[(query_start_index as usize)..full_query.len()].to_string()
        } else {
            full_query[(query_start_index as usize)..((query_start_index + query_len) as usize)]
                .to_string()
        }
    } else {
        full_query.to_string()
    };

    Ok(current_query)
}

pub fn get_query_type(planned_stmt: *mut pg_sys::PlannedStmt) -> QueryType {
    let mut row_tables: Vec<PgRelation> = vec![];
    let mut col_tables: Vec<PgRelation> = vec![];

    unsafe {
        let rtable = (*planned_stmt).rtable;
        #[cfg(feature = "pg12")]
        let mut current_cell = (*rtable).head;
        #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
        let elements = (*rtable).elements;

        for i in 0..(*rtable).length {
            let rte: *mut pg_sys::RangeTblEntry;
            #[cfg(feature = "pg12")]
            {
                rte = (*current_cell).data.ptr_value as *mut pg_sys::RangeTblEntry;
                current_cell = (*current_cell).next;
            }
            #[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
            {
                rte = (*elements.offset(i as isize)).ptr_value as *mut pg_sys::RangeTblEntry;
            }

            if (*rte).rtekind != pg_sys::RTEKind_RTE_RELATION {
                continue;
            }
            let relation = pg_sys::RelationIdGetRelation((*rte).relid);
            let pg_relation = PgRelation::from_pg_owned(relation);

            if pg_relation.is_foreign_table() {
                let foreign_table = pg_sys::GetForeignTable(pg_relation.oid());
                let foreign_server = pg_sys::GetForeignServer((*foreign_table).serverid);
                let fdw_handler = FdwHandler::from(foreign_server);
                if fdw_handler != FdwHandler::Other {
                    col_tables.push(pg_relation)
                }
            } else {
                row_tables.push(pg_relation)
            }
        }
    }

    match (row_tables.is_empty(), col_tables.is_empty()) {
        (true, true) => QueryType::Postgres,
        (false, true) => QueryType::Postgres,
        (true, false) => QueryType::DataFusion,
        (false, false) => QueryType::Postgres,
    }
}
