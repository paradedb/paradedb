use pgrx::*;
use std::ffi::CStr;
use thiserror::Error;

pub trait Query {
    // Extracts the query string from a PlannedStmt,
    // accounting for multi-line queries where we only want a
    // specific line of the entire query.
    fn get_query_string(self, source_text: &CStr) -> Result<String, QueryStringError>;
}

impl Query for *mut pg_sys::PlannedStmt {
    fn get_query_string(self, source_text: &CStr) -> Result<String, QueryStringError> {
        let query_start_index = unsafe { (*self).stmt_location };
        let query_len = unsafe { (*self).stmt_len };
        let mut query = source_text.to_str()?;

        if query_start_index != -1 {
            if query_len == 0 {
                query = &query[(query_start_index as usize)..query.len()];
            } else {
                query = &query
                    [(query_start_index as usize)..((query_start_index + query_len) as usize)];
            }
        }

        println!("query: {:?}", query.to_string());

        Ok(query.to_string())
    }
}

#[derive(Error, Debug)]
pub enum QueryStringError {
    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),
}
