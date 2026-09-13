// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use std::io::{self, Read, Seek, SeekFrom, Write};
use std::ptr::NonNull;

use pgrx::{check_for_interrupts, pg_sys};
use superkmeans::TempStorage;

const SEGMENT_BYTES: u64 = 0x40000000;

pub struct PgTempStorage;

pub struct PgTempFile {
    file: NonNull<pg_sys::BufFile>,
    position: u64,
    len: u64,
}

impl TempStorage for PgTempStorage {
    type File = PgTempFile;

    fn create(&mut self) -> io::Result<Self::File> {
        check_for_interrupts!();
        let file = NonNull::new(unsafe { pg_sys::BufFileCreateTemp(false) })
            .ok_or_else(|| io::Error::other("could not create training temporary file"))?;
        Ok(PgTempFile {
            file,
            position: 0,
            len: 0,
        })
    }

    fn buffer_bytes_per_file(&self) -> usize {
        pg_sys::BLCKSZ as usize + 1024
    }
}

impl PgTempFile {
    fn seek_file(&mut self, position: u64) -> io::Result<()> {
        let segment = i32::try_from(position / SEGMENT_BYTES).map_err(|_| {
            io::Error::other("training temporary file exceeds BufFile segment limit")
        })?;
        let offset = (position % SEGMENT_BYTES) as pg_sys::off_t;
        if unsafe { pg_sys::BufFileSeek(self.file.as_ptr(), segment, offset, 0) } != 0 {
            return Err(io::Error::other("could not seek training temporary file"));
        }
        Ok(())
    }

    fn extend_segments(&mut self) -> io::Result<()> {
        // BufFile cannot seek across uncreated 1 GiB segments. Extend sparsely,
        // completing each segment before starting the next one.
        while self.position / SEGMENT_BYTES > self.len.saturating_sub(1) / SEGMENT_BYTES {
            check_for_interrupts!();
            let boundary = (self.len.saturating_sub(1) / SEGMENT_BYTES + 1) * SEGMENT_BYTES;
            let start = self.len.max(boundary - 1);
            self.seek_file(start)?;
            let mut zeros = [0_u8; 2];
            unsafe {
                pg_sys::BufFileWrite(
                    self.file.as_ptr(),
                    zeros.as_mut_ptr().cast(),
                    (boundary + 1 - start) as usize,
                );
            }
            self.len = boundary + 1;
        }
        Ok(())
    }
}

impl Read for PgTempFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        check_for_interrupts!();
        if self.position >= self.len || buf.is_empty() {
            return Ok(0);
        }
        self.seek_file(self.position)?;
        let size = (buf.len() as u64).min(self.len - self.position) as usize;
        let read =
            unsafe { pg_sys::BufFileRead(self.file.as_ptr(), buf.as_mut_ptr().cast(), size) };
        self.position += read as u64;
        Ok(read)
    }
}

impl Write for PgTempFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        check_for_interrupts!();
        if buf.is_empty() {
            return Ok(0);
        }
        let end = self
            .position
            .checked_add(buf.len() as u64)
            .ok_or_else(|| io::Error::other("training temporary file size overflow"))?;
        if self.position > self.len {
            self.extend_segments()?;
        }
        self.seek_file(self.position)?;
        unsafe {
            pg_sys::BufFileWrite(
                self.file.as_ptr(),
                buf.as_ptr().cast_mut().cast(),
                buf.len(),
            );
        }
        self.position = end;
        self.len = self.len.max(end);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Seek for PgTempFile {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let position = match position {
            SeekFrom::Start(offset) => Some(offset),
            SeekFrom::Current(offset) => self.position.checked_add_signed(offset),
            SeekFrom::End(offset) => self.len.checked_add_signed(offset),
        }
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "temporary file seek overflow")
        })?;
        self.position = position;
        Ok(position)
    }
}

impl Drop for PgTempFile {
    fn drop(&mut self) {
        unsafe {
            pg_sys::BufFileClose(self.file.as_ptr());
        }
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use pgrx::pg_test;

    #[pg_test]
    fn training_temp_file_replays_and_replaces_data() {
        let mut file = PgTempStorage.create().unwrap();
        file.write_all(b"abcdef").unwrap();
        file.seek(SeekFrom::Start(2)).unwrap();
        file.write_all(b"XY").unwrap();
        file.rewind().unwrap();
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"abXYef");
        assert_eq!(file.seek(SeekFrom::End(-2)).unwrap(), 4);
    }

    #[pg_test]
    fn training_temp_file_can_scatter_across_segment_boundaries() {
        let mut file = PgTempStorage.create().unwrap();
        file.write_all(b"root").unwrap();
        file.seek(SeekFrom::Start(2 * SEGMENT_BYTES + 8)).unwrap();
        file.write_all(b"child").unwrap();
        file.rewind().unwrap();
        let mut bytes = [0; 4];
        file.read_exact(&mut bytes).unwrap();
        assert_eq!(&bytes, b"root");
        file.seek(SeekFrom::Start(SEGMENT_BYTES - 1)).unwrap();
        file.read_exact(&mut bytes).unwrap();
        assert_eq!(bytes, [0; 4]);
        file.seek(SeekFrom::End(-5)).unwrap();
        let mut child = [0; 5];
        file.read_exact(&mut child).unwrap();
        assert_eq!(&child, b"child");
    }

    #[pg_test]
    fn training_temp_file_builds_vector_index_from_spilled_sample() {
        pgrx::Spi::run(
            "CREATE EXTENSION IF NOT EXISTS vector;
            SET paradedb.vector_min_training_rows = 1;
            SET maintenance_work_mem = '16MB';
            SET max_parallel_workers_per_gather = 0;
            CREATE TABLE spill_training_vectors (id integer, embedding vector(3));
            INSERT INTO spill_training_vectors
            SELECT i, ARRAY[(i % 17)::real, (i % 31)::real, 1.0::real]::vector
            FROM generate_series(1, 3000) i;
            CREATE INDEX spill_training_vectors_idx ON spill_training_vectors
            USING paradedb (id, embedding vector_cosine_ops)
            WITH (key_field = 'id', centroid_ratio = 0.02, training_sample_ratio = 0.5);",
        )
        .unwrap();
        let count = pgrx::Spi::get_one::<i64>(
            "SELECT count(*) FROM (
            SELECT id FROM spill_training_vectors WHERE id @@@ pdb.all()
            ORDER BY embedding <=> '[1,2,1]'::vector LIMIT 10
        ) nearest",
        )
        .unwrap();
        assert_eq!(count, Some(10));
    }
}
