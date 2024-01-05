use datafusion::arrow::record_batch::RecordBatch;
use datafusion::common::DFSchema;
use lazy_static::lazy_static;
use parking_lot::RwLock;
use pgrx::*;

pub struct BulkInsertState {
    pub batches: Vec<RecordBatch>,
    pub schema: Option<DFSchema>,
    pub nslots: usize,
}

impl BulkInsertState {
    pub const fn new() -> Self {
        BulkInsertState {
            batches: vec![],
            schema: None,
            nslots: 0,
        }
    }
}

lazy_static! {
    pub static ref BULK_INSERT_STATE: RwLock<BulkInsertState> = RwLock::new(BulkInsertState::new());
}

pub unsafe fn get_pg_relation(rte: *mut pg_sys::RangeTblEntry) -> Result<PgRelation, String> {
    let relation = pg_sys::RelationIdGetRelation((*rte).relid);
    Ok(PgRelation::from_pg_owned(relation))
}
