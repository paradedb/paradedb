use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::{BM25PageSpecialData, PgItem};
use crate::postgres::storage::fsm::FreeSpaceManager;
use crate::postgres::storage::metadata::MetaPage;
use crate::postgres::storage::utils::{BM25BufferCache, BM25Page};
use crate::postgres::storage::MAX_BUFFERS_TO_EXTEND_BY;
use pgrx::pg_sys;

#[derive(Debug)]
pub struct Buffer {
    pg_buffer: pg_sys::Buffer,
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            if self.pg_buffer != pg_sys::InvalidBuffer as pg_sys::Buffer
                && pg_sys::InterruptHoldoffCount > 0    // if it's not we're likely unwinding the stack due to a panic and unlocking buffers isn't possible anymore
                && crate::postgres::utils::IsTransactionState()
            {
                pg_sys::UnlockReleaseBuffer(self.pg_buffer);
            }
        }
    }
}

impl Buffer {
    fn new(pg_buffer: pg_sys::Buffer) -> Self {
        assert!(
            unsafe { pg_sys::IsTransactionState() },
            "buffer cannot be allocated outside of a transaction"
        );
        assert!(pg_buffer != pg_sys::InvalidBuffer as pg_sys::Buffer);
        Self { pg_buffer }
    }

    pub fn page(&self) -> Page {
        let pg_page = unsafe { pg_sys::BufferGetPage(self.pg_buffer) };
        Page {
            pg_page,
            _buffer: self,
        }
    }

    pub fn number(&self) -> pg_sys::BlockNumber {
        unsafe { pg_sys::BufferGetBlockNumber(self.pg_buffer) }
    }

    pub fn page_size(&self) -> pg_sys::Size {
        unsafe { pg_sys::BufferGetPageSize(self.pg_buffer) }
    }
}

#[derive(Debug)]
pub struct BufferMut {
    dirty: bool,
    inner: Buffer,
}

impl Drop for BufferMut {
    fn drop(&mut self) {
        unsafe {
            if crate::postgres::utils::IsTransactionState() && self.dirty {
                pg_sys::MarkBufferDirty(self.inner.pg_buffer);
            }
        }
    }
}

impl BufferMut {
    pub fn init_page(&mut self) -> PageMut {
        let page_size = self.page_size();
        let page = self.page_mut();
        page.buffer.dirty = true;
        unsafe {
            pg_sys::PageInit(page.pg_page, page_size, size_of::<BM25PageSpecialData>());

            let special = pg_sys::PageGetSpecialPointer(page.pg_page) as *mut BM25PageSpecialData;
            (*special).next_blockno = pg_sys::InvalidBlockNumber;
            (*special).xmax = pg_sys::InvalidTransactionId;
        }
        page
    }

    #[allow(dead_code)]
    pub fn page(&self) -> Page {
        unsafe {
            Page {
                pg_page: pg_sys::BufferGetPage(self.inner.pg_buffer),
                _buffer: &self.inner,
            }
        }
    }

    pub fn page_mut(&mut self) -> PageMut {
        let pg_page = unsafe { pg_sys::BufferGetPage(self.inner.pg_buffer) };
        PageMut {
            buffer: self,
            pg_page,
        }
    }

    pub fn number(&self) -> pg_sys::BlockNumber {
        self.inner.number()
    }

    pub fn page_size(&self) -> pg_sys::Size {
        self.inner.page_size()
    }

    /// Return this [`BufferMut`] instance back to Postgres' Free Space Map, making
    /// it available for future reuse as a new buffer.
    pub fn return_to_fsm(self, bman: &mut BufferManager) {
        let blockno = self.number();
        bman.fsm().extend(bman, std::iter::once(blockno));
    }
}

/// Holds an array of block numbers -- used for bulk allocating new buffers.
pub struct BufferMutVec {
    inner: [pg_sys::Buffer; MAX_BUFFERS_TO_EXTEND_BY],
    cursor: usize,
}

impl BufferMutVec {
    pub fn new(buffers: [pg_sys::Buffer; MAX_BUFFERS_TO_EXTEND_BY]) -> Self {
        Self {
            inner: buffers,
            cursor: 0,
        }
    }

    pub fn empty() -> Self {
        Self {
            inner: [pg_sys::InvalidBuffer as pg_sys::Buffer; MAX_BUFFERS_TO_EXTEND_BY],
            cursor: MAX_BUFFERS_TO_EXTEND_BY + 1, // positioned after the end
        }
    }
}

impl Iterator for BufferMutVec {
    type Item = BufferMut;

    /// Claim a buffer from the start, which ensures that the buffers are in the same order as they were created.
    /// Typically this means in order of increasing block number.
    fn next(&mut self) -> Option<BufferMut> {
        if self.cursor >= MAX_BUFFERS_TO_EXTEND_BY {
            return None;
        }

        let pg_buffer = self.inner[self.cursor];
        self.cursor += 1;

        if pg_buffer == pg_sys::InvalidBuffer as pg_sys::Buffer {
            return None;
        }

        unsafe { pg_sys::LockBuffer(pg_buffer, pg_sys::BUFFER_LOCK_EXCLUSIVE as _) };
        Some(BufferMut {
            dirty: false,
            inner: Buffer { pg_buffer },
        })
    }
}

#[derive(Debug)]
pub struct PinnedBuffer {
    pg_buffer: pg_sys::Buffer,
}

impl Drop for PinnedBuffer {
    fn drop(&mut self) {
        unsafe {
            if crate::postgres::utils::IsTransactionState() {
                pg_sys::ReleaseBuffer(self.pg_buffer);
            }
        }
    }
}

impl PinnedBuffer {
    fn new(pg_buffer: pg_sys::Buffer) -> Self {
        assert!(pg_buffer != pg_sys::InvalidBuffer as pg_sys::Buffer);
        Self { pg_buffer }
    }
}

pub struct Page<'a> {
    pg_page: pg_sys::Page,

    // we never use this directly, but we hold onto it so that its Drop impl
    // won't run while we're live
    _buffer: &'a Buffer,
}

impl Page<'_> {
    #[allow(dead_code)]
    pub fn free_space(&self) -> usize {
        unsafe { pg_sys::PageGetFreeSpace(self.pg_page) }
    }

    pub fn is_empty(&self) -> bool {
        unsafe { pg_sys::PageIsEmpty(self.pg_page) }
    }

    pub fn max_offset_number(&self) -> pg_sys::OffsetNumber {
        unsafe { pg_sys::PageGetMaxOffsetNumber(self.pg_page) }
    }

    pub fn deserialize_item<T: From<PgItem>>(
        &self,
        offno: pg_sys::OffsetNumber,
    ) -> Option<(T, pg_sys::Size)> {
        unsafe { self.pg_page.deserialize_item(offno) }
    }

    pub fn read_item(&self, offno: pg_sys::OffsetNumber) -> Option<PgItem> {
        unsafe { self.pg_page.read_item(offno) }
    }

    pub fn header(&self) -> &pg_sys::PageHeaderData {
        unsafe { &*(self.pg_page as *const pg_sys::PageHeaderData) }
    }

    pub fn special<T>(&self) -> &T {
        unsafe { &*(pg_sys::PageGetSpecialPointer(self.pg_page) as *const T) }
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe {
            let header_size = std::mem::offset_of!(pg_sys::PageHeaderData, pd_linp);
            let slice_len = self.header().pd_lower as usize - header_size;
            std::slice::from_raw_parts((self.pg_page as *const u8).add(header_size), slice_len)
        }
    }

    pub fn as_slice_range(&self, offset: usize, len: usize) -> &[u8] {
        unsafe {
            let header_size = std::mem::offset_of!(pg_sys::PageHeaderData, pd_linp);
            std::slice::from_raw_parts((self.pg_page as *const u8).add(header_size + offset), len)
        }
    }

    pub fn contents<T: Copy>(&self) -> T {
        unsafe { (pg_sys::PageGetContents(self.pg_page) as *const T).read_unaligned() }
    }

    pub fn next_blockno(&self) -> pg_sys::BlockNumber {
        unsafe {
            let special = pg_sys::PageGetSpecialPointer(self.pg_page) as *mut BM25PageSpecialData;
            (*special).next_blockno
        }
    }
}

pub struct PageMut<'a> {
    buffer: &'a mut BufferMut,
    pg_page: pg_sys::Page,
}

impl PageMut<'_> {
    pub fn max_offset_number(&self) -> pg_sys::OffsetNumber {
        unsafe { pg_sys::PageGetMaxOffsetNumber(self.pg_page) }
    }

    pub fn mark_item_dead(&mut self, offno: pg_sys::OffsetNumber) {
        unsafe {
            let item_id = pg_sys::PageGetItemId(self.pg_page, offno);
            (*item_id).set_lp_flags(pg_sys::LP_DEAD);
            self.buffer.dirty = true;
        }
    }

    pub fn deserialize_item<T: From<PgItem>>(
        &self,
        offno: pg_sys::OffsetNumber,
    ) -> Option<(T, pg_sys::Size)> {
        unsafe { self.pg_page.deserialize_item(offno) }
    }

    pub fn find_item<T: From<PgItem>, F: Fn(T) -> bool>(
        &self,
        cmp: F,
    ) -> Option<pg_sys::OffsetNumber> {
        let max = self.max_offset_number();
        for offno in pg_sys::FirstOffsetNumber as pg_sys::OffsetNumber..=max {
            let (item, _) = self.deserialize_item::<T>(offno)?;
            if cmp(item) {
                return Some(offno);
            }
        }
        None
    }

    #[must_use]
    #[track_caller]
    pub fn append_item(
        &mut self,
        item: pg_sys::Item,
        size: pg_sys::Size,
        flags: i32,
    ) -> pg_sys::OffsetNumber {
        let offno = unsafe {
            pg_sys::PageAddItemExtended(
                self.pg_page,
                item,
                size,
                pg_sys::InvalidOffsetNumber,
                flags,
            )
        };
        if offno != pg_sys::InvalidOffsetNumber {
            self.buffer.dirty = true;
        }
        offno
    }

    #[must_use]
    pub fn replace_item(
        &mut self,
        offno: pg_sys::OffsetNumber,
        item: pg_sys::Item,
        size: pg_sys::Size,
    ) -> bool {
        assert!(offno != pg_sys::InvalidOffsetNumber);
        let did_replace =
            unsafe { pg_sys::PageIndexTupleOverwrite(self.pg_page, offno, item, size) };
        if did_replace {
            self.buffer.dirty = true;
        }
        did_replace
    }

    pub fn delete_items(&mut self, item_offsets: &mut [pg_sys::OffsetNumber]) {
        // assert the list of item offsets is sorted.  Thanks, stack overflow -- it's too late for me to come up with this on my own
        debug_assert!(item_offsets.windows(2).all(|w| w[0] <= w[1]));
        unsafe {
            pg_sys::PageIndexMultiDelete(
                self.pg_page,
                item_offsets.as_mut_ptr(),
                item_offsets.len() as i32,
            );
            self.buffer.dirty = true;
        }
    }

    pub fn delete_item(&mut self, offno: pg_sys::OffsetNumber) {
        unsafe {
            pg_sys::PageIndexTupleDelete(self.pg_page, offno);
        }
        self.buffer.dirty = true;
    }

    pub fn header(&self) -> &pg_sys::PageHeaderData {
        unsafe { &*(self.pg_page as *const pg_sys::PageHeaderData) }
    }

    pub fn header_mut(&mut self) -> &mut pg_sys::PageHeaderData {
        let header = unsafe { &mut *(self.pg_page as *mut pg_sys::PageHeaderData) };
        self.buffer.dirty = true;
        header
    }

    pub fn special_mut<T>(&mut self) -> &mut T {
        let special = unsafe { &mut *(pg_sys::PageGetSpecialPointer(self.pg_page) as *mut T) };
        self.buffer.dirty = true;
        special
    }

    pub fn can_fit(&mut self, len: usize) -> bool {
        let len: u16 = len.try_into().expect("bytes length too large for a page");
        let start = self.header().pd_lower;
        let end = self.header().pd_upper;
        if start + len > end {
            // bytes won't fit here
            return false;
        }
        true
    }

    pub fn free_space_slice_mut(&mut self, len: usize) -> Option<&mut [u8]> {
        let len: u16 = len.try_into().expect("bytes length too large for a page");
        let slice = unsafe {
            let start = self.header().pd_lower;
            let end = self.header().pd_upper;
            if start + len > end {
                // bytes won't fit here
                return None;
            }
            std::slice::from_raw_parts_mut(
                (self.pg_page as *mut u8).add(start as usize),
                len as usize,
            )
        };
        self.buffer.dirty = true;
        Some(slice)
    }

    pub fn append_bytes(&mut self, bytes: &[u8]) -> bool {
        let len: u16 = bytes
            .len()
            .try_into()
            .expect("bytes length too large for a page");
        let slice = unsafe {
            let start = self.header().pd_lower;
            let end = self.header().pd_upper;
            if start + len > end {
                // bytes won't fit here
                return false;
            }
            std::slice::from_raw_parts_mut(
                (self.pg_page as *mut u8).add(start as usize),
                len as usize,
            )
        };
        slice.copy_from_slice(bytes);
        self.header_mut().pd_lower += len;
        self.buffer.dirty = true;
        true
    }

    pub fn contents_mut<T>(&mut self) -> &mut T {
        let contents = unsafe {
            let contents = pg_sys::PageGetContents(self.pg_page) as *mut T;

            // adjust pd_lower at the same time
            let header = self.pg_page as *mut pg_sys::PageHeaderData;
            (*header).pd_lower = (contents.add(1) as usize - header as usize)
                .try_into()
                .expect("pd_lower overflowed");

            &mut *contents
        };
        self.buffer.dirty = true;
        contents
    }

    pub fn next_blockno(&self) -> pg_sys::BlockNumber {
        unsafe {
            let special = pg_sys::PageGetSpecialPointer(self.pg_page) as *mut BM25PageSpecialData;
            (*special).next_blockno
        }
    }
}

pub trait PageHeaderMethods {
    fn free_space(&self) -> usize;
}

impl PageHeaderMethods for pg_sys::PageHeaderData {
    fn free_space(&self) -> usize {
        self.pd_upper as usize - self.pd_lower as usize
    }
}

#[derive(Clone, Debug)]
pub struct BufferManager {
    bcache: BM25BufferCache,
    fsm_blockno: Option<pg_sys::BlockNumber>,
}

impl BufferManager {
    pub fn new(rel: &PgSearchRelation) -> Self {
        Self {
            bcache: BM25BufferCache::open(rel),
            fsm_blockno: None,
        }
    }

    pub fn fsm(&mut self) -> FreeSpaceManager {
        let fsm_blockno = *self
            .fsm_blockno
            .get_or_insert_with(|| MetaPage::open(self.bcache.rel()).fsm());
        FreeSpaceManager::open(fsm_blockno)
    }

    pub fn bm25cache(&self) -> &BM25BufferCache {
        &self.bcache
    }

    #[must_use]
    pub fn new_buffer(&mut self) -> BufferMut {
        let pg_buffer = self
            .fsm()
            .pop(self)
            .map(|blockno| unsafe {
                self.bcache
                    .get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE))
            })
            .unwrap_or_else(|| unsafe { self.bcache.new_buffer() });

        BufferMut {
            dirty: false,
            inner: Buffer { pg_buffer },
        }
    }

    /// Like [`new_buffer`], but returns an iterator of buffers instead.
    /// This is better than calling [`new_buffer`] multiple times because it avoids potentially
    /// locking the relation for every new buffer.
    pub fn new_buffers(&mut self, npages: usize) -> impl Iterator<Item = BufferMut> {
        let fsm_blocknos = self.fsm().pop_many(self, npages);
        let needed = npages - fsm_blocknos.len();
        let new_buffers = unsafe { self.bcache.new_buffers(needed) };

        let rel = self.bcache.rel().as_ptr();
        fsm_blocknos
            .into_iter()
            .map(move |blockno| unsafe {
                let pg_buffer = pg_sys::ReadBuffer(rel, blockno);
                pg_sys::LockBuffer(pg_buffer, pg_sys::BUFFER_LOCK_EXCLUSIVE as _);
                BufferMut {
                    dirty: false,
                    inner: Buffer { pg_buffer },
                }
            })
            .chain(new_buffers)
    }

    pub fn pinned_buffer(&self, blockno: pg_sys::BlockNumber) -> PinnedBuffer {
        unsafe { PinnedBuffer::new(self.bcache.get_buffer(blockno, None)) }
    }

    pub fn get_buffer(&self, blockno: pg_sys::BlockNumber) -> Buffer {
        unsafe {
            Buffer::new(
                self.bcache
                    .get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_SHARE)),
            )
        }
    }

    ///
    /// A convenience wrapper around `get_buffer` to handle acquiring a lock on the Buffer for the
    /// given blockno before releasing the lock on the given Buffer.
    ///
    /// Useful for hand-over-hand locking.
    ///
    pub fn get_buffer_exchange(&self, blockno: pg_sys::BlockNumber, old_buffer: Buffer) -> Buffer {
        let buffer = self.get_buffer(blockno);
        std::mem::drop(old_buffer);
        buffer
    }

    pub fn get_buffer_mut(&mut self, blockno: pg_sys::BlockNumber) -> BufferMut {
        unsafe {
            BufferMut {
                dirty: false,
                inner: Buffer::new(
                    self.bcache
                        .get_buffer(blockno, Some(pg_sys::BUFFER_LOCK_EXCLUSIVE)),
                ),
            }
        }
    }

    ///
    /// See `get_buffer_exchange`.
    ///
    pub fn get_buffer_exchange_mut(
        &mut self,
        blockno: pg_sys::BlockNumber,
        old_buffer: BufferMut,
    ) -> BufferMut {
        let buffer = self.get_buffer_mut(blockno);
        std::mem::drop(old_buffer);
        buffer
    }

    #[allow(dead_code)]
    pub fn get_buffer_conditional(&mut self, blockno: pg_sys::BlockNumber) -> Option<BufferMut> {
        unsafe {
            let pg_buffer = self.bcache.get_buffer(blockno, None);
            if pg_sys::ConditionalLockBuffer(pg_buffer) {
                Some(BufferMut {
                    dirty: false,
                    inner: Buffer::new(pg_buffer),
                })
            } else {
                pg_sys::ReleaseBuffer(pg_buffer);
                None
            }
        }
    }

    pub fn get_buffer_for_cleanup(&mut self, blockno: pg_sys::BlockNumber) -> BufferMut {
        unsafe {
            let buffer = self.bcache.get_buffer_with_strategy(
                blockno,
                pg_sys::ReadBufferMode::RBM_NORMAL as _,
                None,
            );
            pg_sys::LockBufferForCleanup(buffer);
            BufferMut {
                dirty: false,
                inner: Buffer::new(buffer),
            }
        }
    }

    pub fn get_buffer_for_cleanup_conditional(
        &mut self,
        blockno: pg_sys::BlockNumber,
    ) -> Option<BufferMut> {
        unsafe {
            let buffer = self.bcache.get_buffer(blockno, None);
            if pg_sys::ConditionalLockBufferForCleanup(buffer) {
                Some(BufferMut {
                    dirty: false,
                    inner: Buffer::new(buffer),
                })
            } else {
                pg_sys::ReleaseBuffer(buffer);
                None
            }
        }
    }

    pub fn page_is_empty(&self, blockno: pg_sys::BlockNumber) -> bool {
        self.get_buffer(blockno).page().is_empty()
    }
}

pub unsafe fn init_new_buffer(rel: pg_sys::Relation) -> BufferMut {
    unsafe {
        pg_sys::LockRelationForExtension(rel, pg_sys::AccessExclusiveLock as _);
        let pg_buffer = pg_sys::ReadBufferExtended(
            rel,
            pg_sys::ForkNumber::MAIN_FORKNUM,
            pg_sys::InvalidBlockNumber,
            pg_sys::ReadBufferMode::RBM_NORMAL,
            std::ptr::null_mut(),
        );
        pg_sys::UnlockRelationForExtension(rel, pg_sys::AccessExclusiveLock as _);
        pg_sys::LockBuffer(pg_buffer, pg_sys::BUFFER_LOCK_EXCLUSIVE as _);

        let mut buffer = BufferMut {
            dirty: false,
            inner: Buffer { pg_buffer },
        };
        let mut page = buffer.init_page();
        let special = page.special_mut::<BM25PageSpecialData>();
        special.next_blockno = pg_sys::InvalidBlockNumber;
        special.xmax = pg_sys::InvalidTransactionId;
        buffer
    }
}
