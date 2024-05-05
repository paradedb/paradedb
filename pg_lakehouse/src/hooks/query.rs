use pgrx::*;
use std::ffi::CStr;
use std::str::Utf8Error;

use crate::datafusion::plan::QueryString;
use crate::fdw::handler::FdwHandler;

#[derive(PartialEq, Clone)]
pub enum QueryType {
    Federated,
    DataFusion,
    Postgres,
}

pub struct PgQuery {
    text: String,
    query_type: QueryType,
}

impl PgQuery {
    pub fn new(text: String, query_type: QueryType) -> Self {
        Self { text, query_type }
    }

    pub fn text(&self) -> String {
        self.text.clone()
    }

    pub fn query_type(&self) -> QueryType {
        self.query_type.clone()
    }
}

impl TryFrom<PgBox<pg_sys::QueryDesc>> for PgQuery {
    type Error = Utf8Error;

    fn try_from(query_desc: PgBox<pg_sys::QueryDesc>) -> Result<Self, Self::Error> {
        let planned_stmt = unsafe { (*query_desc).plannedstmt };
        let query_start_index = unsafe { (*planned_stmt).stmt_location };
        let query_len = unsafe { (*planned_stmt).stmt_len };
        let query = unsafe { CStr::from_ptr((*query_desc).sourceText) }.to_str()?;

        let raw_text = if query_start_index != -1 {
            if query_len == 0 {
                query[(query_start_index as usize)..query.len()].to_string()
            } else {
                query[(query_start_index as usize)..((query_start_index + query_len) as usize)]
                    .to_string()
            }
        } else {
            query.to_string()
        };

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
                    let foreign_table = unsafe { pg_sys::GetForeignTable(pg_relation.oid()) };
                    let foreign_server =
                        unsafe { pg_sys::GetForeignServer((*foreign_table).serverid) };
                    let fdw_handler = FdwHandler::from((*foreign_server).fdwid);
                    if fdw_handler != FdwHandler::Other {
                        col_tables.push(pg_relation)
                    }
                } else {
                    row_tables.push(pg_relation)
                }
            }
        }

        let query_type = match (row_tables.is_empty(), col_tables.is_empty()) {
            (true, true) => QueryType::Postgres,
            (false, true) => QueryType::Postgres,
            (true, false) => QueryType::DataFusion,
            (false, false) => QueryType::Postgres,
        };

        Ok(PgQuery::new(raw_text, query_type))
    }
}
