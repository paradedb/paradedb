#![allow(unused)]
// use pgrx::pg_sys::{self, AsPgCStr};
// use std::ffi::CString;
// use std::ptr::null_mut;

// // #[link(name = "groonga")]
// // extern "C" {
// //     fn grn_init() -> i32;
// //     fn grn_fin();
// //     fn grn_ctx_init(ctx: *mut grn_ctx, flags: i32) -> i32;
// //     fn grn_ctx_fin(ctx: *mut grn_ctx);
// //     fn grn_ctx_set_wal_role(ctx: *mut grn_ctx, role: i32);
// //     fn grn_set_segv_handler();
// //     fn grn_set_abrt_handler();
// //     fn grn_default_logger_set_flags(flags: i32);
// //     fn grn_default_logger_get_flags() -> i32;
// //     fn grn_default_logger_set_max_level(level: i32);
// //     fn grn_default_logger_set_path(path: *const i8);
// //     fn grn_ct_db(ctx: *mut grn_ctx) -> *mut grn_obj;
// //     fn grn_obj_close(ctx: *mut grn_ctx, obj: *mut grn_obj);
// //     fn grn_db_open(ctx: *mut grn_ctx, path: *const i8) -> *mut grn_obj;
// //     fn grn_db_create(ctx: *mut grn_ctx, path: *const i8, options: *const i8) -> *mut grn_obj;
// // }

// // const PGSEARCH_WAL_RECORD_CREATE_TABLE: u8 = 0x10;
// // const PGSEARCH_WAL_RECORD_CREATE_COLUMN: u8 = 0x20;
// // const PGSEARCH_WAL_RECORD_SET_SOURCES: u8 = 0x30;
// // const PGSEARCH_WAL_RECORD_RENAME_TABLE: u8 = 0x40;
// // const PGSEARCH_WAL_RECORD_INSERT: u8 = 0x50;
// // const PGSEARCH_WAL_RECORD_DELETE: u8 = 0x60;
// // const PGSEARCH_WAL_RECORD_REMOVE_OBJECT: u8 = 0x70;
// // const PGSEARCH_WAL_RECORD_REGISTER_PLUGIN: u8 = 0x80;

// // static mut PGSEARCH_WRM_CONTEXT: grn_ctx = grn_ctx {
// //     impl_: null_mut(),
// //     flags: 0,
// //     ..Default::default()
// // };

// // static mut PGSEARCH_WRM_LOG_PATH: Option<CString> = None;
// // static mut PGSEARCH_WRM_LOG_LEVEL: i32 = 4; // Default log level: Notice

// // const PGSEARCH_TAG: &str = "pgroonga: wal-resource-manager";
// // const PGSEARCH_VERSION: &str = "1.0.0"; // Example version
// // const PGSEARCH_WAL_ROLE_PRIMARY: i32 = 1; // Example constant

pub unsafe extern "C" fn pgsearch_startup() {
    if !pg_sys::StandbyMode {
        return;
    }

    // grn_thread_set_get_limit_func(pgrnwrm_get_thread_limit, std::ptr::null_mut());
    // grn_default_logger_set_flags(grn_default_logger_get_flags() | grn_log_flags_t::GRN_LOG_PID);
    // grn_default_logger_set_max_level(PGSEARCH_WRM_LOG_LEVEL);
    // if let Some(ref log_path) = PGSEARCH_WRM_LOG_PATH {
    //     grn_default_logger_set_path(log_path.as_ptr());
    // }

    // if grn_init() != grn_rc::GRN_SUCCESS {
    //     pgrx::ereport!(pg_sys::ERROR, "pgroonga: failed to initialize Groonga");
    // }

    // grn_set_segv_handler();
    // grn_set_abrt_handler();

    // let rc = grn_ctx_init(&mut PGSEARCH_WRM_CONTEXT, 0);
    // if rc != grn_rc::GRN_SUCCESS {
    //     pgrx::ereport!(
    //         pg_sys::ERROR,
    //         "pgroonga: failed to initialize Groonga context",
    //     );
    // }
    // grn_ctx_set_wal_role(&mut PGSEARCH_WRM_CONTEXT, PGSEARCH_WAL_ROLE_PRIMARY);

    // pgrx::log!(pg_sys::LOG, "{}: startup: <{}>", PGSEARCH_TAG, PGSEARCH_VERSION);
}

pub unsafe extern "C" fn pgsearch_cleanup() {
    // if !pg_sys::StandbyMode {
    //     return;
    // }

    // pgrx::log!(pg_sys::LOG, "{}: cleanup", PGSEARCH_TAG);

    // let db = grn_ctx_db(&mut PGSEARCH_WRM_CONTEXT);
    // if !db.is_null() {
    //     grn_obj_close(&mut PGSEARCH_WRM_CONTEXT, db);
    // }
    // grn_ctx_fin(&mut PGSEARCH_WRM_CONTEXT);
    // grn_fin();
}

pub unsafe extern "C" fn pgsearch_wrm_redo(record: *mut pg_sys::XLogReaderState) {
    if !pg_sys::StandbyMode {
        return;
    }

    // let info = (*(*record).record).header.xl_info & pg_sys::XLR_RMGR_INFO_MASK;

    // match info {
    //     PGSEARCH_WAL_RECORD_CREATE_TABLE => pg_search_wrm_redo_create_table(record),
    //     PGSEARCH_WAL_RECORD_CREATE_COLUMN => pg_search_wrm_redo_create_column(record),
    //     PGSEARCH_WAL_RECORD_SET_SOURCES => pg_search_wrm_redo_set_sources(record),
    //     PGSEARCH_WAL_RECORD_RENAME_TABLE => pg_search_wrm_redo_rename_table(record),
    //     PGSEARCH_WAL_RECORD_INSERT => pg_search_wrm_redo_insert(record),
    //     PGSEARCH_WAL_RECORD_DELETE => pg_search_wrm_redo_delete(record),
    //     PGSEARCH_WAL_RECORD_REMOVE_OBJECT => pg_search_wrm_redo_remove_object(record),
    //     PGSEARCH_WAL_RECORD_REGISTER_PLUGIN => pg_search_wrm_redo_register_plugin(record),
    //     _ => pgrx::ereport!(pg_sys::ERROR, "pgroonga: [redo] unknown info {}", info),
    // }
}

pub unsafe extern "C" fn pgsearch_wrm_desc(
    buffer: *mut pg_sys::StringInfoData,
    record: *mut pg_sys::XLogReaderState,
) {
    return;
}

pub unsafe extern "C" fn pgsearch_wrm_identify(info: u8) -> *const std::os::raw::c_char {
    pg_search_wrm_info_to_string(info)
}

// unsafe extern "C" fn pg_search_wrm_get_thread_limit(_: *mut std::ffi::c_void) -> u32 {
//     1
// }

// // pub static RESOURCE_MANAGER_DATA: pg_sys::RmgrData = pg_sys::RmgrData {
// //     rm_name: "pg_search".as_pg_cstr(),
// //     rm_redo: Some(pg_search_wrm_redo),
// //     rm_desc: Some(pg_search_wrm_desc),
// //     rm_identify: Some(pg_search_wrm_identify),
// //     rm_startup: Some(pg_search_wrm_startup),
// //     rm_cleanup: Some(pg_search_wrm_cleanup),
// //     rm_mask: None,
// //     rm_decode: None,
// // };

// unsafe fn pg_search_wrm_redo_create_table(record: *mut pg_sys::XLogReaderState) {
//     // Implement the redo logic for create table WAL record
//     // similar to the C implementation
// }

// unsafe fn pg_search_wrm_redo_create_column(record: *mut pg_sys::XLogReaderState) {
//     // Implement the redo logic for create column WAL record
//     // similar to the C implementation
// }

// unsafe fn pg_search_wrm_redo_set_sources(record: *mut pg_sys::XLogReaderState) {
//     // Implement the redo logic for set sources WAL record
//     // similar to the C implementation
// }

// unsafe fn pg_search_wrm_redo_rename_table(record: *mut pg_sys::XLogReaderState) {
//     // Implement the redo logic for rename table WAL record
//     // similar to the C implementation
// }

// unsafe fn pg_search_wrm_redo_insert(record: *mut pg_sys::XLogReaderState) {
//     // Implement the redo logic for insert WAL record
//     // similar to the C implementation
// }

// unsafe fn pg_search_wrm_redo_delete(record: *mut pg_sys::XLogReaderState) {
//     // Implement the redo logic for delete WAL record
//     // similar to the C implementation
// }

// unsafe fn pg_search_wrm_redo_remove_object(record: *mut pg_sys::XLogReaderState) {
//     // Implement the redo logic for remove object WAL record
//     // similar to the C implementation
// }

// unsafe fn pg_search_wrm_redo_register_plugin(record: *mut pg_sys::XLogReaderState) {
//     // Implement the redo logic for register plugin WAL record
//     // similar to the C implementation
// }

unsafe fn pg_search_wrm_info_to_string(info: u8) -> *const std::os::raw::c_char {
    // Match on the different writer request types.
    CString::new("WRITER_REQUEST_HERE").unwrap().into_raw()
}

use pgrx::pg_sys::{self, GenericXLogState};
use std::{ffi::CString, ptr::copy_nonoverlapping};

// The ParadeDB reserved resource manager ID, registered at:
// https://wiki.postgresql.org/wiki/CustomWALResourceManagers
pub static RESOURCE_MANAGER_ID: u8 = 137;
pub const XLOG_RESOURCE_MANAGER_MESSAGE: u8 = 0x00;

const MAX_WAL_SIZE: usize = usize::MAX;
const P_NEW: u32 = pg_sys::InvalidBlockNumber;
const PGSEARCH_WAL_META_PAGE_BLOCK_NUMBER: usize = 1;
const PGSEARCH_WAL_META_PAGE_SPECIAL_VERSION: u32 = 1;
const MAX_GENERIC_XLOG_PAGES: usize = 64;

struct MetaPageSpecial {
    next: pg_sys::BlockNumber,
    max: pg_sys::BlockNumber,
    version: u32,
}

struct PgSearchWALPageWriteData {
    state: *mut pg_sys::GenericXLogState,
    meta_page_special: *mut MetaPageSpecial,
    buffer: pg_sys::Buffer,
    page: pg_sys::Page,
}

pub struct PgSearchWALData {
    current: CurrentPage,
    meta: MetaPage,
    n_used_pages: usize,
    num_buffers: usize,
    buffers: Vec<pg_sys::Buffer>,
    state: pg_sys::GenericXLogState,
    index: pg_sys::Relation,
}

struct CurrentPage {
    buffer: pg_sys::Buffer,
    page: pg_sys::Page,
}

struct MetaPage {
    buffer: pg_sys::Buffer,
    page: pg_sys::Page,
    page_special: *mut MetaPageSpecial,
}

impl MetaPage {
    unsafe fn new(state: &mut GenericXLogState, index: *mut pg_sys::RelationData) -> Self {
        if pg_sys::RelationGetNumberOfBlocksInFork(index, pg_sys::ForkNumber_MAIN_FORKNUM) == 0 {
            let buffer = PgSearchWALData::wal_read_locked_buffer(
                index,
                P_NEW,
                pg_sys::BUFFER_LOCK_EXCLUSIVE as pg_sys::LOCKMODE,
            );
            // self.buffers.push(self.meta.buffer);
            let page = pg_sys::GenericXLogRegisterBuffer(
                state,
                buffer,
                pg_sys::GENERIC_XLOG_FULL_IMAGE as i32,
            );
            pg_sys::PageInit(
                page,
                pg_sys::BLCKSZ as usize,
                std::mem::size_of::<MetaPageSpecial>(),
            );
            let page_special = page_get_special_pointer(page) as *mut MetaPageSpecial;
            unsafe {
                (*page_special).next =
                    (PGSEARCH_WAL_META_PAGE_BLOCK_NUMBER + 1) as pg_sys::BlockNumber;
                (*page_special).max = ((*page_special).next + 1) as pg_sys::BlockNumber;
                (*page_special).version = PGSEARCH_WAL_META_PAGE_SPECIAL_VERSION;
            }

            Self {
                buffer,
                page,
                page_special,
            }
        } else {
            let buffer = PgSearchWALData::wal_read_locked_buffer(
                index,
                PGSEARCH_WAL_META_PAGE_BLOCK_NUMBER as u32,
                pg_sys::BUFFER_LOCK_EXCLUSIVE as pg_sys::LOCKMODE,
            );
            // self.buffers.push(self.meta.buffer);
            let page = pg_sys::GenericXLogRegisterBuffer(state, buffer, 0);
            let page_special = page_get_special_pointer(page) as *mut MetaPageSpecial;

            Self {
                buffer,
                page,
                page_special,
            }
        }
    }
}

// impl Drop for PgSearchWALData {
//     fn drop(&mut self) {
//         // Lock the buffer page.
//         // We can use any block number for this. We just want an index
//         // level lock but we can't use LockRelation(index) because it
//         // conflicts with REINDEX INDEX CONCURRENTLY.
//         let block_number = 0;
//         unsafe {
//             let lock_mode = Self::wal_lock_mode();
//             pg_sys::UnlockPage(self.index, block_number, lock_mode);
//         }
//     }
// }

impl PgSearchWALData {
    unsafe fn new_with_lock(index: pg_sys::Relation) -> Option<Self> {
        if index.is_null() {
            return None;
        }

        Self::wal_lock(index);

        let mut state = *pg_sys::GenericXLogStart(index);
        let meta = MetaPage::new(&mut state, index);

        Some(Self {
            num_buffers: 0,
            buffers: vec![pg_sys::InvalidBuffer as i32; MAX_GENERIC_XLOG_PAGES],
            n_used_pages: 1, // meta page
            current: CurrentPage {
                buffer: pg_sys::InvalidBuffer as pg_sys::Buffer,
                page: std::ptr::null_mut(),
            },
            state,
            meta,
            index,
        })
    }

    unsafe fn wal_page_writer(&mut self, buffer: &[u8]) -> isize {
        let mut written: usize = 0;
        let mut rest = buffer.len();

        while written < buffer.len() {
            let free_size = self.wal_page_get_free_size(self.current.page) as usize;

            if rest <= free_size {
                self.wal_page_append(self.current.page, &buffer[written..]);
                written += rest;
            } else {
                self.wal_page_append(self.current.page, &buffer[written..written + free_size]);
                written += free_size;
                rest -= free_size;
            }

            if self.wal_page_get_free_size(self.current.page) == 0 {
                self.wal_page_filled();
                self.wal_page_writer_ensure_current();
            }
        }

        written as isize
    }

    unsafe fn wal_page_filled(&mut self) {
        self.current.page = std::ptr::null_mut();
        self.current.buffer = pg_sys::InvalidBuffer as pg_sys::LOCKMODE;

        if MAX_WAL_SIZE == 0 {
            (*self.meta.page_special).next += 1;
            if (*self.meta.page_special).next >= (*self.meta.page_special).max {
                (*self.meta.page_special).max = (*self.meta.page_special).next + 1;
            }
        } else {
            let current_size = ((1 + (*self.meta.page_special).next) * pg_sys::BLCKSZ) as usize;
            let mut max_size = MAX_WAL_SIZE as usize;
            let min_max_size = ((1 + 2) * pg_sys::BLCKSZ) as usize;

            if max_size < min_max_size {
                max_size = min_max_size;
            }

            if current_size < max_size {
                (*self.meta.page_special).next += 1;
                if (*self.meta.page_special).next >= (*self.meta.page_special).max {
                    (*self.meta.page_special).max = (*self.meta.page_special).next + 1;
                }
            } else {
                (*self.meta.page_special).max = (*self.meta.page_special).next + 1;
                (*self.meta.page_special).next = (PGSEARCH_WAL_META_PAGE_BLOCK_NUMBER + 1) as u32;
            }
        }
    }

    unsafe fn wal_page_get_free_size(&self, page: pg_sys::Page) -> u16 {
        let page_header = unsafe { *(page as *const pg_sys::PageHeader) };
        (*page_header).pd_upper - (*page_header).pd_lower
    }

    unsafe fn wal_page_append(&self, page: pg_sys::Page, data: &[u8]) {
        let page_header = unsafe { *(page as *mut pg_sys::PageHeader) };
        let page_data = self.wal_page_get_data(page);
        unsafe {
            copy_nonoverlapping(
                data.as_ptr(),
                page_data.add(self.wal_page_get_last_offset(page)),
                data.len(),
            );
        }
        (*page_header).pd_lower += (data.len()) as u16;
    }

    unsafe fn wal_page_writer_ensure_current(&mut self) {
        if self.current.buffer as u32 != pg_sys::InvalidBuffer {
            return;
        }

        if self.n_used_pages == MAX_GENERIC_XLOG_PAGES {
            // Need to call restart here.
            // self.wal_data_restart();
        }

        let meta = unsafe { &mut *self.meta.page_special };
        if pg_sys::RelationGetNumberOfBlocksInFork(self.index, pg_sys::ForkNumber_MAIN_FORKNUM)
            <= meta.next
        {
            self.current.buffer = Self::wal_read_locked_buffer(
                self.index,
                P_NEW,
                pg_sys::BUFFER_LOCK_EXCLUSIVE as pg_sys::LOCKMODE,
            );
            self.buffers.push(self.current.buffer);
            meta.next = pg_sys::BufferGetBlockNumber(self.current.buffer);
            self.current.page = pg_sys::GenericXLogRegisterBuffer(
                &mut self.state,
                self.current.buffer,
                pg_sys::GENERIC_XLOG_FULL_IMAGE as pg_sys::LOCKMODE,
            );
            pg_sys::PageInit(self.current.page, pg_sys::BLCKSZ as usize, 0);
        } else {
            self.current.buffer = Self::wal_read_locked_buffer(
                self.index,
                meta.next,
                pg_sys::BUFFER_LOCK_EXCLUSIVE as pg_sys::LOCKMODE,
            );
            self.buffers.push(self.current.buffer);
            self.current.page =
                pg_sys::GenericXLogRegisterBuffer(&mut self.state, self.current.buffer, 0);
            if self.wal_page_get_free_size(self.current.page) == 0 {
                pg_sys::PageInit(self.current.page, pg_sys::BLCKSZ as usize, 0);
            }
        }

        self.n_used_pages += 1;
    }

    unsafe fn wal_data_restart(mut self) {
        self.wal_finish();
        self.wal_data_release_buffers();
        self.state = *pg_sys::GenericXLogStart(self.index);
        self.wal_data_init_n_used_pages();
        self.wal_data_init_meta();
        self.wal_data_init_current();
    }

    unsafe fn wal_start(&mut self) {
        if self.index.is_null() {
            return;
        }

        Self::wal_lock(self.index);

        self.state = *pg_sys::GenericXLogStart(self.index);
        self.wal_data_init_buffers();
        self.wal_data_init_n_used_pages();
        self.wal_data_init_meta();
        self.wal_data_init_current();
    }

    unsafe fn wal_insert_finish(&mut self) {
        let json = serde_json::to_string("").expect("blew up serializing to json");
        // Convert the Rust String to a CString
        let c_json = CString::new(json).expect("CString::new failed");
        // Get the raw pointer from the CString and cast to *mut c_char
        let json_ptr = c_json.as_ptr() as *mut std::ffi::c_char;

        // Pass the raw pointer to the XLogRegisterData function

        pg_sys::XLogBeginInsert(); // Needs to be called before RegisterData/Insert.
        pg_sys::XLogRegisterData(json_ptr, c_json.to_bytes().len() as u32);
        // XLR_SPECIAL_REL_UPDATE needs to be set here because we are modifying
        // additonal files to the usual relation files. External tools that read WAL
        // need this to recognize these extra files.
        pg_sys::XLogInsert(
            RESOURCE_MANAGER_ID,
            XLOG_RESOURCE_MANAGER_MESSAGE | pg_sys::XLR_SPECIAL_REL_UPDATE as u8,
        );
    }

    unsafe fn wal_lock(index: *mut pg_sys::RelationData) {
        /* We can use any block number for this. We just want an index
         * level lock but we can't use LockRelation(index) because it
         * conflicts with REINDEX INDEX CONCURRENTLY. */
        let block_number = 0;
        let lock_mode = Self::wal_lock_mode();
        pg_sys::LockPage(index, block_number, lock_mode);
    }

    unsafe fn wal_unlock(&mut self) {
        /* We can use any block number for this. We just want an index
         * level lock but we can't use LockRelation(index) because it
         * conflicts with REINDEX INDEX CONCURRENTLY. */
        let block_number = 0;
        let lock_mode = Self::wal_lock_mode();
        pg_sys::UnlockPage(self.index, block_number, lock_mode);
    }

    unsafe fn wal_lock_mode() -> pg_sys::LOCKMODE {
        if pg_sys::RecoveryInProgress() {
            pg_sys::RowExclusiveLock as pg_sys::LOCKMODE
        } else {
            pg_sys::ShareUpdateExclusiveLock as pg_sys::LOCKMODE
        }
    }

    unsafe fn wal_read_locked_buffer(
        index: pg_sys::Relation,
        block_number: pg_sys::BlockNumber,
        buffer_lock_mode: i32,
    ) -> pg_sys::Buffer {
        let lock_mode = pg_sys::ExclusiveLock;
        let mut buffer;

        if block_number == P_NEW {
            pg_sys::LockRelationForExtension(index, lock_mode as pg_sys::LOCKMODE);
        }
        buffer = pg_sys::ReadBuffer(index, block_number);
        pg_sys::LockBuffer(buffer, buffer_lock_mode);
        if block_number == P_NEW {
            pg_sys::UnlockRelationForExtension(index, lock_mode as pg_sys::LOCKMODE);
        }

        buffer
    }

    fn wal_page_get_data(&self, page: pg_sys::Page) -> *mut u8 {
        assert!(!page.is_null());
        let size_of_page_header_data = std::mem::size_of::<pg_sys::PageHeaderData>();
        // MAXALIGN calculation assuming 8-byte alignment
        let maxalign_size = (size_of_page_header_data + 7) & !7;
        unsafe { (page as *mut u8).add(maxalign_size) }
    }

    unsafe fn wal_finish(&mut self) {
        let (block, offset) = if !self.current.page.is_null() {
            (
                pg_sys::BufferGetBlockNumber(self.current.buffer),
                self.wal_page_get_last_offset(self.current.page),
            )
        } else {
            ((*self.meta.page_special).next, 0)
        };

        pg_sys::GenericXLogFinish(&mut self.state);
        pg_search_index_status_set_wal_applied_position(self.index, block, offset as u16);
    }

    unsafe fn wal_data_release_buffers(&mut self) {
        for buffer in &self.buffers {
            pg_sys::UnlockReleaseBuffer(*buffer);
        }
        self.buffers.clear();
        self.num_buffers = 0;
    }

    fn wal_data_init_buffers(&mut self) {
        self.num_buffers = 0;
        self.buffers = vec![pg_sys::InvalidBuffer as i32; MAX_GENERIC_XLOG_PAGES];
    }

    fn wal_data_init_n_used_pages(&mut self) {
        self.n_used_pages = 1; // meta page
    }

    unsafe fn wal_data_init_meta(&mut self) {
        if pg_sys::RelationGetNumberOfBlocksInFork(self.index, pg_sys::ForkNumber_MAIN_FORKNUM) == 0
        {
            self.meta.buffer = Self::wal_read_locked_buffer(
                self.index,
                P_NEW,
                pg_sys::BUFFER_LOCK_EXCLUSIVE as pg_sys::LOCKMODE,
            );
            self.buffers.push(self.meta.buffer);
            self.meta.page = pg_sys::GenericXLogRegisterBuffer(
                &mut self.state,
                self.meta.buffer,
                pg_sys::GENERIC_XLOG_FULL_IMAGE as i32,
            );
            pg_sys::PageInit(
                self.meta.page,
                pg_sys::BLCKSZ as usize,
                std::mem::size_of::<MetaPageSpecial>(),
            );
            self.meta.page_special =
                page_get_special_pointer(self.meta.page) as *mut MetaPageSpecial;
            unsafe {
                (*self.meta.page_special).next =
                    (PGSEARCH_WAL_META_PAGE_BLOCK_NUMBER + 1) as pg_sys::BlockNumber;
                (*self.meta.page_special).max =
                    ((*self.meta.page_special).next + 1) as pg_sys::BlockNumber;
                (*self.meta.page_special).version = PGSEARCH_WAL_META_PAGE_SPECIAL_VERSION;
            }
        } else {
            self.meta.buffer = Self::wal_read_locked_buffer(
                self.index,
                PGSEARCH_WAL_META_PAGE_BLOCK_NUMBER as u32,
                pg_sys::BUFFER_LOCK_EXCLUSIVE as pg_sys::LOCKMODE,
            );
            self.buffers.push(self.meta.buffer);
            self.meta.page =
                pg_sys::GenericXLogRegisterBuffer(&mut self.state, self.meta.buffer, 0);
            self.meta.page_special =
                page_get_special_pointer(self.meta.page) as *mut MetaPageSpecial;
        }
    }

    fn wal_data_init_current(&mut self) {
        self.current.buffer = pg_sys::InvalidBuffer as pg_sys::Buffer;
        self.current.page = std::ptr::null_mut();
    }

    unsafe fn wal_page_get_last_offset(&self, page: pg_sys::Page) -> usize {
        let size_of_page_header_data = std::mem::size_of::<pg_sys::PageHeaderData>();
        let page_header = unsafe { *(page as *const pg_sys::PageHeader) };
        (*page_header).pd_lower as usize - size_of_page_header_data
    }
}

// extern "C" {
//     fn PgSearchIndexStatusGetRecordID(index: Relation) -> grn_id;
//     fn PgSearchLookup(name: *const libc::c_char, error_level: libc::c_int) -> *mut grn_obj;
//     fn grn_ctx_get(
//         ctx: *mut grn_ctx,
//         name: *const libc::c_char,
//         name_size: libc::size_t,
//     ) -> *mut grn_obj;
//     fn grn_table_add(
//         ctx: *mut grn_ctx,
//         table: *mut grn_obj,
//         key: *const libc::c_void,
//         key_size: libc::size_t,
//         added: *mut grn_id,
//     ) -> grn_id;
//     fn GRN_UINT64_SET(ctx: *mut grn_ctx, obj: *mut grn_obj, val: u64);
//     fn grn_obj_set_value(
//         ctx: *mut grn_ctx,
//         obj: *mut grn_obj,
//         id: grn_id,
//         value: *mut grn_obj,
//         flags: libc::c_int,
//     ) -> grn_rc;
//     fn grn_db_touch(ctx: *mut grn_ctx, obj: *mut grn_obj);
// }

unsafe fn page_get_special_pointer(page: pg_sys::Page) -> *mut u8 {
    assert!(!page.is_null());
    let page_header = unsafe { *(page as *const pg_sys::PageHeader) };
    assert!((*page_header).pd_special <= pg_sys::BLCKSZ as u16);
    assert!((*page_header).pd_special >= std::mem::size_of::<pg_sys::PageHeaderData>() as u16);

    unsafe { (page as *mut u8).add((*page_header).pd_special as usize) }
}

unsafe fn pg_search_index_status_set_wal_applied_position(
    index: pg_sys::Relation,
    block: pg_sys::BlockNumber,
    offset: pg_sys::LocationIndex,
) {
    // let id = PgSearchIndexStatusGetRecordID(index);
    // let column_name = CString::new("TABLE_NAME.WAL_APPLIED_POSITION_COLUMN_NAME").unwrap();
    // let column = PgSearchLookup(column_name.as_ptr(), ERROR);
    // let position_raw = ((block as u64) << 32) + (offset as u64);

    // let mut position: grn_obj = std::mem::zeroed();
    // GRN_UINT64_SET(ptr::null_mut(), &mut position, position_raw);

    // grn_obj_set_value(ptr::null_mut(), column, id, &mut position, GRN_OBJ_SET);
    // grn_db_touch(ptr::null_mut(), column);
}

unsafe fn pg_search_index_status_get_record_id_with_wal(
    index: pg_sys::Relation,
    wal_data: *mut *mut PgSearchWALData,
    n_columns: usize,
) -> u32 {
    // let table_name = CString::new("TABLE_NAME").unwrap();
    // let table = PgSearchLookupWithSize(table_name.as_ptr(), table_name.as_bytes().len(), ERROR);
    // let key = &PGSEARCH_RELATION_GET_LOCATOR_NUMBER(index) as *const u32;
    // let key_size = std::mem::size_of::<u32>();

    // let id = grn_table_add(
    //     ptr::null_mut(),
    //     table,
    //     key as *const libc::c_void,
    //     key_size,
    //     ptr::null_mut(),
    // );
    // if id != GRN_ID_NIL && !wal_data.is_null() {
    //     *wal_data = PgSearchWALStart(index);
    //     PgSearchWALInsertStart(*wal_data, table, n_columns);
    //     PgSearchWALInsertKeyRaw(*wal_data, key as *const libc::c_void, key_size);
    // }

    // id
    0
}

// unsafe fn pg_search__lookup(name: &str, error_level: libc::c_int) -> *mut grn_obj {
//     let name_cstr = CString::new(name).unwrap();
//     pg_search__lookup_with_size(name_cstr.as_ptr(), name.len(), error_level)
// }

// unsafe fn pg_search__lookup_with_size(
//     name: *const libc::c_char,
//     name_size: usize,
//     error_level: libc::c_int,
// ) -> *mut grn_obj {
//     let object = grn_ctx_get(ptr::null_mut(), name, name_size);
//     if object.is_null() && error_level != PGSEARCH_ERROR_LEVEL_IGNORE {
//         grn_plugin_error(GRN_INVALID_ARGUMENT, name, name_size);
//         pg_search__check("PgSearchLookupWithSize");
//     }
//     object
// }

// unsafe fn pg_search__check(format: &str, args: fmt::Arguments) -> bool {
//     if ctx.rc == GRN_SUCCESS {
//         return true;
//     }

//     #[cfg(feature = "PGSEARCH_MODULE_PGROONGA")]
//     if PgSearchIsRLSEnabled {
//         pg_re_throw();
//     }

//     let message = format!("{}", args);
//     ereport(
//         ERROR,
//         errcode(pg_search__grn_rc_to_pg_error_code(ctx.rc)),
//         errmsg!("{}: {}: {}", PGSEARCH_TAG, message, ctx.errbuf),
//     );
//     false
// }

// fn grn_uint64_set(ctx: *mut grn_ctx, obj: *mut grn_obj, val: u64) {
//     unsafe {
//         grn_bulk_write_from(
//             ctx,
//             obj,
//             &val as *const u64 as *const libc::c_char,
//             0,
//             std::mem::size_of::<u64>(),
//         );
//     }
// }

// unsafe fn grn_db_touch(ctx: *mut grn_ctx, obj: *mut grn_obj) {
//     grn_obj_touch(ctx, obj, ptr::null_mut());
// }

// unsafe fn grn_obj_touch(ctx: *mut grn_ctx, obj: *mut grn_obj, tv: *mut grn_timeval) {
//     let mut tv_ = grn_timeval {
//         tv_sec: 0,
//         tv_nsec: 0,
//     };

//     if tv.is_null() {
//         grn_timeval_now(ctx, &mut tv_);
//         tv = &mut tv_;
//     }

//     match obj.header.type_ {
//         GRN_DB => grn_obj_touch_db(ctx, obj, tv),
//         GRN_TABLE_HASH_KEY | GRN_TABLE_PAT_KEY | GRN_TABLE_DAT_KEY | GRN_TABLE_NO_KEY
//         | GRN_COLUMN_VAR_SIZE | GRN_COLUMN_FIX_SIZE | GRN_COLUMN_INDEX => {
//             if !is_temp(obj) {
//                 let io = grn_obj_get_io(ctx, obj);
//                 if !io.is_null() {
//                     (*io).header.last_modified = tv.tv_sec as u32;
//                 }
//                 grn_obj_touch(ctx, grn_db_of(obj), tv);
//             }
//         }
//         _ => {}
//     }
// }

// unsafe fn grn_timeval_now(ctx: *mut grn_ctx, tv: *mut grn_timeval) -> grn_rc {
//     #[cfg(windows)]
//     {
//         let mut tb: libc::timeb;
//         _ftime_s(&mut tb);
//         tv.tv_sec = tb.time;
//         tv.tv_nsec = tb.millitm * (GRN_TIME_NSEC_PER_SEC / 1000);
//         GRN_SUCCESS
//     }

//     #[cfg(not(windows))]
//     {
//         #[cfg(have_clock_gettime)]
//         {
//             let mut t: libc::timespec;
//             if libc::clock_gettime(libc::CLOCK_REALTIME, &mut t) == 0 {
//                 tv.tv_sec = t.tv_sec;
//                 tv.tv_nsec = t.tv_nsec as i32;
//                 ctx.rc
//             } else {
//                 grn_set_error(ctx, "clock_gettime failed");
//                 ctx.rc
//             }
//         }

//         #[cfg(not(have_clock_gettime))]
//         {
//             let mut t: libc::timeval;
//             if libc::gettimeofday(&mut t, ptr::null_mut()) == 0 {
//                 tv.tv_sec = t.tv_sec;
//                 tv.tv_nsec = t.tv_usec * GRN_TIME_USEC_TO_NSEC as i32;
//                 ctx.rc
//             } else {
//                 grn_set_error(ctx, "gettimeofday failed");
//                 ctx.rc
//             }
//         }
//     }
// }

// unsafe fn grn_obj_touch_db(ctx: *mut grn_ctx, obj: *mut grn_obj, tv: *mut grn_timeval) {
//     let io = grn_obj_get_io(ctx, obj);
//     if !io.is_null() {
//         (*io).header.last_modified = tv.tv_sec as u32;
//     }
//     grn_db_dirty(ctx, obj);
// }

// unsafe fn grn_obj_get_io(ctx: *mut grn_ctx, obj: *mut grn_obj) -> *mut grn_io {
//     if obj.is_null() {
//         return ptr::null_mut();
//     }

//     let obj = if obj.header.type_ == GRN_DB {
//         (*obj.cast::<grn_db>()).keys
//     } else {
//         obj
//     };

//     match obj.header.type_ {
//         GRN_TABLE_PAT_KEY => (*obj.cast::<grn_pat>()).io,
//         GRN_TABLE_DAT_KEY => (*obj.cast::<grn_dat>()).io,
//         GRN_TABLE_HASH_KEY => (*obj.cast::<grn_hash>()).io,
//         GRN_TABLE_NO_KEY => (*obj.cast::<grn_array>()).io,
//         GRN_COLUMN_VAR_SIZE => (*obj.cast::<grn_ja>()).io,
//         GRN_COLUMN_FIX_SIZE => (*obj.cast::<grn_ra>()).io,
//         GRN_COLUMN_INDEX => (*obj.cast::<grn_ii>()).seg,
//         _ => ptr::null_mut(),
//     }
// }

// #[pg_guard]
// pub extern "C" fn PgSearchWALPageWriter(
//     user_data: *mut std::ffi::c_void,
//     buffer: *const u8,
//     length: usize,
// ) -> i32 {
//     let data = unsafe { &mut *(user_data as *mut PgSearchWALData) };
//     let buffer_slice = unsafe { std::slice::from_raw_parts(buffer, length) };
//     data.wal_page_writer(buffer_slice) as i32
// }

// #[pg_guard]
// pub extern "C" fn PgSearchWALPageFilled(data: *mut PgSearchWALData) {
//     unsafe { &mut *data }.wal_page_filled()
// }

// #[pg_guard]
// pub extern "C" fn PgSearchWALDataRestart(data: *mut PgSearchWALData) {
//     unsafe { &mut *data }.wal_data_restart()
// }

// #[pg_guard]
// pub extern "C" fn PgSearchWALReadLockedBuffer(
//     index: Relation,
//     block_number: BlockNumber,
//     buffer_lock_mode: i32,
// ) -> Buffer {
//     PgSearchWALData::wal_read_locked_buffer(index, block_number, buffer_lock_mode)
// }

// #[pg_guard]
// pub extern "C" fn PgSearchWALDataFinish(data: *mut PgSearchWALData) {
//     unsafe { &mut *data }.wal_data_finish()
// }

// #[pg_guard]
// pub extern "C" fn PgSearchWALDataReleaseBuffers(data: *mut PgSearchWALData) {
//     unsafe { &mut *data }.wal_data_release_buffers()
// }

// #[pg_guard]
// pub extern "C" fn PgSearchWALDataInitNUsedPages(data: *mut PgSearchWALData) {
//     unsafe { &mut *data }.wal_data_init_n_used_pages()
// }

// #[pg_guard]
// pub extern "C" fn PgSearchWALDataInitMeta(data: *mut PgSearchWALData) {
//     unsafe { &mut *data }.wal_data_init_meta()
// }

// #[pg_guard]
// pub extern "C" fn PgSearchWALDataInitCurrent(data: *mut PgSearchWALData) {
//     unsafe { &mut *data }.wal_data_init_current()
// }
