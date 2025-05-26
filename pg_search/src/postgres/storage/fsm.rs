use crate::postgres::storage::block::{MVCCEntry, PgItem};
use crate::postgres::storage::buffer::BufferManager;
use pgrx::pg_sys;

#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct FreeBlockNumber(pg_sys::BlockNumber);

impl From<FreeBlockNumber> for PgItem {
    fn from(val: FreeBlockNumber) -> Self {
        PgItem(
            val.0 as pg_sys::Item,
            std::mem::size_of::<pg_sys::BlockNumber>(),
        )
    }
}

impl From<PgItem> for FreeBlockNumber {
    fn from(pg_item: PgItem) -> Self {
        FreeBlockNumber(pg_item.0 as pg_sys::BlockNumber)
    }
}

// TODO: This is for compatibility with LinkedItemList, which requires MVCCEntry to be implemented
// Eventually we shouldn't have to implement this trait
impl MVCCEntry for FreeBlockNumber {
    fn pintest_blockno(&self) -> pg_sys::BlockNumber {
        self.0
    }

    unsafe fn visible(&self) -> bool {
        true
    }

    unsafe fn recyclable(&self, bman: &mut BufferManager) -> bool {
        false
    }

    unsafe fn mergeable(&self) -> bool {
        false
    }
}
