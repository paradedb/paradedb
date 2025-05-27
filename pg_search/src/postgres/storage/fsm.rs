use crate::postgres::storage::block::{BM25PageSpecialData, PgItem};
use crate::postgres::storage::buffer::BufferManager;
use pgrx::pg_sys;

#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct FreeBlockNumber(pg_sys::BlockNumber);

impl From<pg_sys::BlockNumber> for FreeBlockNumber {
    fn from(val: pg_sys::BlockNumber) -> Self {
        FreeBlockNumber(val)
    }
}

impl From<FreeBlockNumber> for pg_sys::BlockNumber {
    fn from(val: FreeBlockNumber) -> Self {
        val.0
    }
}

impl From<FreeBlockNumber> for PgItem {
    fn from(val: FreeBlockNumber) -> Self {
        let bytes = val.0.to_ne_bytes();
        let ptr = unsafe { pg_sys::palloc(bytes.len()) } as *mut i8;
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr() as *const i8, ptr, bytes.len()) };
        PgItem(
            ptr as pg_sys::Item,
            std::mem::size_of::<pg_sys::BlockNumber>(),
        )
    }
}

impl From<PgItem> for FreeBlockNumber {
    fn from(pg_item: PgItem) -> Self {
        let bytes = unsafe {
            std::slice::from_raw_parts(
                pg_item.0 as *const u8,
                std::mem::size_of::<pg_sys::BlockNumber>(),
            )
        };
        FreeBlockNumber(pg_sys::BlockNumber::from_ne_bytes(
            bytes.try_into().unwrap(),
        ))
    }
}

pub struct FreeBlockList {
    start_block_number: pg_sys::BlockNumber,
    bman: BufferManager,
}

impl FreeBlockList {
    ///
    /// Open a new [`FreeBlockList`].
    ///
    /// # Arguments
    ///
    /// * `relation_oid` - The OID of the relation to vacuum.
    /// * `start_block_number` - The block number of the first block in the list.
    pub fn open(relation_oid: pg_sys::Oid, start_block_number: pg_sys::BlockNumber) -> Self {
        let bman = BufferManager::new(relation_oid);
        Self {
            start_block_number,
            bman,
        }
    }

    ///
    /// Pop a free block number from end of the list
    ///
    /// # Returns
    ///
    /// A [`FreeBlockNumber`] if one is available, or `None` if the list is empty.
    pub fn pop(&mut self) -> Option<FreeBlockNumber> {
        // Go to the end of the list, creating a list of block numbers along the way
        let mut blockno = self.start_block_number;
        let mut buffer = self.bman.get_buffer(blockno);
        let mut blocknos = Vec::new();

        while blockno != pg_sys::InvalidBlockNumber {
            if blockno != self.start_block_number {
                buffer = self.bman.get_buffer_exchange(blockno, buffer);
            }

            blocknos.push(blockno);
            let page = buffer.page();
            blockno = page.next_blockno();
        }

        drop(buffer);

        // Pop a [`FreeBlockNumber`] from the end of the list
        for blockno in blocknos.into_iter().rev() {
            let mut buffer = self.bman.get_buffer_mut(blockno);
            let mut page = buffer.page_mut();
            let max_offset = page.max_offset_number();

            if max_offset == pg_sys::InvalidOffsetNumber {
                drop(buffer);
                continue;
            }

            if let Some((item, _)) = page.deserialize_item::<FreeBlockNumber>(max_offset) {
                page.delete_item(max_offset);
                return Some(item);
            }
        }

        None
    }

    ///
    /// Append a list of free block numbers to the FSM.
    ///
    /// # Arguments
    ///
    /// * `items` - The list of free block numbers to append.
    pub unsafe fn append_list(&mut self, items: &[FreeBlockNumber]) {
        let mut buffer = self.bman.get_buffer_mut(self.start_block_number);

        for item in items {
            let PgItem(pg_item, size) = item.clone().into();

            'append_loop: loop {
                let mut page = buffer.page_mut();
                let offsetno = page.append_item(pg_item, size, 0);
                if offsetno != pg_sys::InvalidOffsetNumber {
                    // it added to this block
                    break 'append_loop;
                } else if page.next_blockno() != pg_sys::InvalidBlockNumber {
                    // go to the next block
                    let next_blockno = page.next_blockno();
                    buffer = self.bman.get_buffer_mut(next_blockno);
                } else {
                    // The FSM cannot call new_buffer() because it would cause a circular dependency
                    // new_buffer() itself relies on the FSM - we have to extend the relation here
                    let mut new_page = self.bman.extend_relation();
                    let new_blockno = new_page.number();
                    new_page.init_page();

                    let special = page.special_mut::<BM25PageSpecialData>();
                    special.next_blockno = new_blockno;

                    buffer = new_page;
                }
            }
        }
    }
}
