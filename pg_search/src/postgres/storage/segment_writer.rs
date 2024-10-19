use pgrx::*;
use std::io::Write;
use std::path::Path;

#[derive(Clone, Copy, Debug)]
pub struct SegmentWriter {
    relation_oid: u32,
    start_blockno: pg_sys::BlockNumber,
}

pub(crate) struct SegmentSpecialData {
    next_blockno: pg_sys::BlockNumber,
}

impl SegmentWriter {
    pub unsafe fn new(relation_oid: u32, path: &Path) -> Self {
        // let base = BaseDirectory::new(relation_oid);
        // let segment_blockno = base
        //     .new_buffer(std::mem::size_of::<SegmentSpecialData>())
        //     .block_number();
        // let meta_buffer = base.get_buffer(SEARCH_META_BLOCKNO, pg_sys::BUFFER_LOCK_SHARE);
        // let page = meta_buffer.page();

        // // Add segment to the metadata map
        // match pg_sys::PageGetMaxOffsetNumber(page) == pg_sys::InvalidOffsetNumber {
        //     true => {
        //         let mut segments = HashMap::new();
        //         segments.insert(segment_blockno, PathBuf::from(path));
        //         pgrx::info!("segments is null {:?}", segments);
        //         let serialized = serde_json::to_vec(&segments).unwrap();
        //         let item = std::ffi::CString::new(serialized.clone()).unwrap().into_raw() as pg_sys::Item;

        //         pgrx::info!("serialized");
        //         base.add_item(
        //             &meta_buffer,
        //             pg_sys::InvalidOffsetNumber,
        //             item,
        //             serialized.len(),
        //             0,
        //         );
        //         pgrx::info!("added item");
        //     }
        //     false => {
        //         pgrx::info!("not null");
        //         let item =
        //             base.get_item(&meta_buffer, pg_sys::FirstOffsetNumber) as *mut SearchMetaMap;
        //         pgrx::info!("got item");
        //         let mut segments = (*item).segments.clone();
        //         segments.insert(segment_blockno, PathBuf::from(path));
        //         pgrx::info!("not null {:?}", segments);
        //         (*item).segments = segments;
        //     }
        // };

        // meta_buffer.mark_dirty();

        Self {
            relation_oid,
            start_blockno: 3,
        }
    }
}

impl Write for SegmentWriter {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        unsafe {
            pgrx::info!("Writing");
            // let base = BaseDirectory::new(self.relation_oid);
            // let data_size = data.len();
            // let mut buffer = base.get_buffer(self.start_blockno, pg_sys::BUFFER_LOCK_EXCLUSIVE);
            // let mut page = buffer.page();
            // let mut start_byte = 0;
            // let mut end_byte = min(
            //     data_size,
            //     pg_sys::PageGetFreeSpace(page) - std::mem::size_of::<pg_sys::ItemIdData>(),
            // );
            // let mut data_slice = &data[start_byte..end_byte];

            // while end_byte <= data_size {
            //     pgrx::info!("writing start_byte: {start_byte}, end_byte: {end_byte}");
            //     if start_byte != 0 {
            //         let new_buffer = base.new_buffer(std::mem::size_of::<SegmentSpecialData>());
            //         let special = pg_sys::PageGetSpecialPointer(page) as *mut SegmentSpecialData;
            //         (*special).next_blockno = new_buffer.block_number();
            //         buffer = new_buffer.clone();
            //         page = new_buffer.page();
            //         pgrx::info!("new buffer created");
            //     }

            //     base.add_item(
            //         &buffer,
            //         pg_sys::InvalidOffsetNumber,
            //         data_slice.as_ptr() as pg_sys::Item,
            //         data_slice.len(),
            //         pg_sys::PAI_OVERWRITE,
            //     );

            //     start_byte = end_byte;
            //     end_byte = min(
            //         data_size,
            //         end_byte + pg_sys::PageGetFreeSpace(page)
            //             - std::mem::size_of::<pg_sys::ItemIdData>(),
            //     );
            //     data_slice = &data[start_byte..end_byte];
            //     buffer.mark_dirty();
            // }

            Ok(data.len())
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        pgrx::info!("Flushing");
        Ok(())
    }
}
