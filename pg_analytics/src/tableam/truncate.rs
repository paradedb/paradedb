use pgrx::*;
use thiserror::Error;

#[pg_guard]
pub extern "C" fn deltalake_relation_nontransactional_truncate(_rel: pg_sys::Relation) {
    panic!("{}", TruncateError::TruncateNotSupported.to_string());
}

#[derive(Error, Debug)]
pub enum TruncateError {
    #[error("relation_nontransactional_truncate not yet implemented")]
    TruncateNotSupported,
}
