use std::{
    io::{self, Error, Write},
    path::PathBuf,
};

use pgrx::{IntoDatum, Spi};
use tantivy::directory::{AntiCallToken, TerminatingWrite};

pub struct SQLWriter {
    name: String,
    path: PathBuf,
    buf: Vec<u8>,
}

impl SQLWriter {
    pub fn new(name: String, path: PathBuf) -> SQLWriter {
        SQLWriter {
            name,
            path,
            buf: Vec::new(),
        }
    }
}

impl TerminatingWrite for SQLWriter {
    fn terminate_ref(&mut self, _: AntiCallToken) -> Result<(), Error> {
        self.flush().expect("failed to flush writer");
        Ok(())
    }
}

impl Write for SQLWriter {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(buf);
        Ok(buf.len())
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        let query = format!(
            "INSERT INTO {} (path, content) VALUES ('{}', $1) ON CONFLICT (path) DO UPDATE SET content = $1",
            self.name,
            self.path.display()
        );
        let args = vec![(
            pgrx::PgBuiltInOids::BYTEAOID.oid(),
            self.buf.as_slice().into_datum(),
        )];

        Spi::run_with_args(&query, Some(args)).expect("failed to write file");
        Ok(())
    }
}
