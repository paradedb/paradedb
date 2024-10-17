use pgrx::*;
use std::ptr::null_mut;

// The first block of the index is the metadata block, which is essentially a map for how the rest of the blocks are organized.
// It is our responsibility to ensure that the metadata block is the first block by creating it immediately when the index is built.
const SEARCH_META_BLOCKNO: pg_sys::BlockNumber = 0;

pub(crate) struct SearchMetaSpecialData {
    next_blockno: pg_sys::BlockNumber,
    tantivy_meta_blockno: pg_sys::BlockNumber,
    tantivy_managed_blockno: pg_sys::BlockNumber,
}

pub(crate) struct TantivyMetaSpecialData {
    next_blockno: pg_sys::BlockNumber,
    len: u32,
}

#[derive(Clone, Debug)]
pub struct BaseDirectory {
    relation: pg_sys::Relation,
}

pub struct PgBuffer(pub pg_sys::Buffer);
impl PgBuffer {
    pub unsafe fn from_pg_owned(buffer: pg_sys::Buffer) -> Self {
        PgBuffer(buffer)
    }

    pub unsafe fn block_number(&self) -> pg_sys::BlockNumber {
        pg_sys::BufferGetBlockNumber(self.0)
    }

    pub unsafe fn page(&self) -> pg_sys::Page {
        pg_sys::BufferGetPage(self.0)
    }

    pub unsafe fn page_size(&self) -> usize {
        pg_sys::BufferGetPageSize(self.0)
    }

    pub unsafe fn mark_dirty(&self) {
        pg_sys::MarkBufferDirty(self.0);
    }
}

impl Drop for PgBuffer {
    fn drop(&mut self) {
        unsafe {
            pg_sys::UnlockReleaseBuffer(self.0);
        }
    }
}

impl BaseDirectory {
    pub unsafe fn new(relation_oid: u32) -> Self {
        Self {
            relation: pg_sys::relation_open(relation_oid.into(), pg_sys::AccessShareLock as i32),
        }
    }

    pub unsafe fn add_item(
        &self,
        buffer: &PgBuffer,
        offsetno: pg_sys::OffsetNumber,
        item: pg_sys::Item,
        item_size: usize,
        flags: u32,
    ) {
        let page = buffer.page();
        let offsetno = pg_sys::PageAddItemExtended(page, item, item_size, offsetno, flags as i32);
        buffer.mark_dirty();
    }

    pub unsafe fn add_page(&self, special_size: usize) -> pg_sys::BlockNumber {
        // Providing an InvalidBlockNumber creates a new page
        let buffer = self.get_buffer(pg_sys::InvalidBlockNumber, pg_sys::BUFFER_LOCK_EXCLUSIVE);
        let blockno = buffer.block_number();

        pg_sys::PageInit(buffer.page(), buffer.page_size(), special_size);
        buffer.mark_dirty();
        // Returns the BlockNumber of the newly-created page
        blockno
    }

    pub unsafe fn get_item(
        &self,
        blockno: pg_sys::BlockNumber,
        offsetno: pg_sys::OffsetNumber,
    ) -> pg_sys::Item {
        let buffer = self.get_buffer(blockno, pg_sys::BUFFER_LOCK_SHARE);
        let page = buffer.page();
        let item = pg_sys::PageGetItem(page, pg_sys::PageGetItemId(page, offsetno));
        item
    }

    pub unsafe fn get_buffer(&self, blockno: pg_sys::BlockNumber, lock: u32) -> PgBuffer {
        let buffer = pg_sys::ReadBufferExtended(
            self.relation,
            pg_sys::ForkNumber::MAIN_FORKNUM,
            blockno,
            pg_sys::ReadBufferMode::RBM_NORMAL,
            null_mut(),
        );
        pg_sys::LockBuffer(buffer, lock as i32);
        PgBuffer::from_pg_owned(buffer)
    }
}

impl Drop for BaseDirectory {
    fn drop(&mut self) {
        unsafe {
            pg_sys::RelationClose(self.relation);
        }
    }
}

#[derive(Clone, Debug)]
pub struct TantivyMetaDirectory {
    base: BaseDirectory,
    tantivy_meta_blockno: pg_sys::BlockNumber,
    tantivy_managed_blockno: pg_sys::BlockNumber,
}

impl TantivyMetaDirectory {
    pub unsafe fn new(relation_oid: u32) -> Self {
        let base = BaseDirectory::new(relation_oid);
        let buffer = base.get_buffer(SEARCH_META_BLOCKNO, pg_sys::BUFFER_LOCK_SHARE);
        let page = buffer.page();
        let special = pg_sys::PageGetSpecialPointer(page) as *mut SearchMetaSpecialData;

        Self {
            base: BaseDirectory::new(relation_oid),
            tantivy_meta_blockno: (*special).tantivy_meta_blockno,
            tantivy_managed_blockno: (*special).tantivy_managed_blockno,
        }
    }

    pub unsafe fn read_meta(&self) -> Vec<u8> {
        self.read_bytes(self.tantivy_meta_blockno)
    }

    pub unsafe fn read_managed(&self) -> Vec<u8> {
        self.read_bytes(self.tantivy_managed_blockno)
    }

    pub unsafe fn write_meta(&self, data: &[u8]) {
        self.write_bytes(data, self.tantivy_meta_blockno);
    }

    pub unsafe fn write_managed(&self, data: &[u8]) {
        self.write_bytes(data, self.tantivy_managed_blockno);
    }

    unsafe fn read_bytes(&self, blockno: pg_sys::BlockNumber) -> Vec<u8> {
        let buffer = self.base.get_buffer(blockno, pg_sys::BUFFER_LOCK_SHARE);
        let page = buffer.page();
        let special = pg_sys::PageGetSpecialPointer(page) as *mut TantivyMetaSpecialData;
        let item = self.base.get_item(blockno, pg_sys::FirstOffsetNumber);
        let len = (*special).len as usize;

        let mut vec = Vec::with_capacity(len);
        std::ptr::copy(item as *mut u8, vec.as_mut_ptr(), len);
        vec.set_len(len);
        vec
    }

    unsafe fn write_bytes(&self, data: &[u8], blockno: pg_sys::BlockNumber) {
        pgrx::info!("writing {} bytes to blockno {}", data.len(), blockno);
        let buffer = self.base.get_buffer(blockno, pg_sys::BUFFER_LOCK_EXCLUSIVE);
        let page = buffer.page();
        let special = pg_sys::PageGetSpecialPointer(page) as *mut TantivyMetaSpecialData;
        (*special).len = data.len() as u32;

        self.base.add_item(
            &buffer,
            pg_sys::FirstOffsetNumber,
            data.as_ptr() as pg_sys::Item,
            data.len(),
            pg_sys::PAI_OVERWRITE,
        );

        pgrx::info!("wrote {} bytes to blockno {}", data.len(), blockno);

        buffer.mark_dirty();

        pgrx::info!("released buffer");
    }
}

pub unsafe fn create_metadata(relation_oid: u32) {
    let base = BaseDirectory::new(relation_oid);
    let blockno = base.add_page(std::mem::size_of::<SearchMetaSpecialData>());
    assert!(
        blockno == SEARCH_META_BLOCKNO,
        "expected metadata blockno to be 0 but got {SEARCH_META_BLOCKNO}"
    );

    let buffer = base.get_buffer(blockno, pg_sys::BUFFER_LOCK_EXCLUSIVE);
    let page = buffer.page();
    let special = pg_sys::PageGetSpecialPointer(page) as *mut SearchMetaSpecialData;

    (*special).tantivy_meta_blockno = base.add_page(std::mem::size_of::<TantivyMetaSpecialData>());
    (*special).tantivy_managed_blockno =
        base.add_page(std::mem::size_of::<TantivyMetaSpecialData>());

    buffer.mark_dirty();
}
