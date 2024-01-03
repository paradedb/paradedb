use datafusion::prelude::SessionContext;
use lazy_static::lazy_static;
use parking_lot::RwLock;

pub static PARADE_CATALOG: &str = "datafusion";
pub static PARADE_SCHEMA: &str = "public";
pub static PARADE_DIRECTORY: &str = "paradedb";

lazy_static! {
    pub static ref CONTEXT: RwLock<Option<SessionContext>> = RwLock::new(None);
}
