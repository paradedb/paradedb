use pgrx::*;
use std::io::Write;
use std::path::Path;

#[derive(Clone, Copy, Debug)]
pub struct SegmentWriter {
    relation_oid: u32,
    blockno: Option<pg_sys::BlockNumber>,
    offsetno: Option<pg_sys::OffsetNumber>,
}

pub(crate) struct SegmentSpecialData {
    next_blockno: pg_sys::BlockNumber,
}

impl SegmentWriter {
    pub unsafe fn new(relation_oid: u32, path: &Path) -> Self {
        if path.to_str().unwrap().ends_with(".lock") {
            return Self {
                relation_oid,
                blockno: None,
                offsetno: None,
            };
        }

        return Self {
            relation_oid,
            blockno: None,
            offsetno: None,
        };
    }
}

impl Write for SegmentWriter {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        unsafe {
            pgrx::info!("Writing data");
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

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        pgrx::info!("Writing all");
        Ok(())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        pgrx::info!("Flushing");
        Ok(())
    }
}
