use pgrx::*;
use thiserror::Error;

pub static TUPLES_PER_PAGE: u16 = pg_sys::MaxOffsetNumber - pg_sys::FirstOffsetNumber;
pub static FIRST_ROW_NUMBER: i64 = 1;
pub static METADATA_BLOCKNO: u32 = 0;

#[derive(Copy, Clone, Debug)]
pub struct RowNumber(pub i64);

#[derive(Copy, Clone, Debug)]
pub struct BlockNumber(pub i64);

impl TryFrom<RowNumber> for pg_sys::ItemPointerData {
    type Error = TidError;

    fn try_from(row_number: RowNumber) -> Result<Self, Self::Error> {
        let RowNumber(row_number) = row_number;

        let mut tid = pg_sys::ItemPointerData::default();
        let block_number = row_number / (TUPLES_PER_PAGE as i64);
        let offset_number =
            (row_number % (TUPLES_PER_PAGE as i64)) + (pg_sys::FirstOffsetNumber as i64);

        item_pointer_set_all(&mut tid, block_number as u32, offset_number as u16);

        Ok(tid)
    }
}

impl TryFrom<pg_sys::ItemPointerData> for RowNumber {
    type Error = TidError;

    fn try_from(tid: pg_sys::ItemPointerData) -> Result<Self, Self::Error> {
        let (block_number, offset_number) = item_pointer_get_both(tid);
        let block_number = block_number as i64;
        let offset_number = offset_number as i64;

        let row_number = block_number * (TUPLES_PER_PAGE as i64) + offset_number
            - (pg_sys::FirstOffsetNumber as i64);

        if row_number < FIRST_ROW_NUMBER {
            return Err(TidError::InvalidRowNumber(row_number));
        }

        Ok(RowNumber(row_number))
    }
}

impl From<RowNumber> for BlockNumber {
    fn from(row_number: RowNumber) -> Self {
        let RowNumber(row_number) = row_number;
        let block_number = row_number / (TUPLES_PER_PAGE as i64);

        BlockNumber(block_number)
    }
}

#[derive(Error, Debug)]
pub enum TidError {
    #[error("Unexpected invalid row number {0}")]
    InvalidRowNumber(i64),
}
