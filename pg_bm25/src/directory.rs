use pgrx::IntoDatum;
use pgrx::Spi;
use std::fmt::Debug;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{fmt, io};
use tantivy::directory::error::{DeleteError, OpenReadError, OpenWriteError};
use tantivy::directory::{
    Directory, FileHandle, OwnedBytes, TerminatingWrite, WatchCallback, WatchHandle, WritePtr,
};
use tantivy::HasLen;

use crate::sql_writer::SQLWriter;

struct SQLDirectoryFileHandle {
    sql_directory: SQLDirectory,
    path: PathBuf,
}

impl HasLen for SQLDirectoryFileHandle {
    fn len(&self) -> usize {
        let data = self
            .sql_directory
            .atomic_read(&self.path)
            .expect("failed to read length");
        data.len()
    }
}

impl fmt::Debug for SQLDirectoryFileHandle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "SQLDirectoryFileHandle({:?}, dir={:?})",
            &self.path, self.sql_directory
        )
    }
}

impl FileHandle for SQLDirectoryFileHandle {
    fn read_bytes(&self, _byte_range: Range<usize>) -> io::Result<OwnedBytes> {
        let data = self
            .sql_directory
            .atomic_read(&self.path.to_path_buf())
            .expect("failed to read bytes");

        let start = _byte_range.start;
        let end = _byte_range.end;

        if end > data.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Byte range exceeds the length of the data",
            ));
        }

        let bytes = data[start..end].to_vec();

        let owned_bytes = OwnedBytes::new(bytes);

        Ok(owned_bytes)
    }
}

#[derive(Clone)]
pub struct SQLDirectory {
    name: String,
}

impl Debug for SQLDirectory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SQLDirectory")
    }
}

impl SQLDirectory {
    pub fn new(name: String) -> SQLDirectory {
        let create_if_exists_q = format!(
            "CREATE TABLE IF NOT EXISTS {} (
                path TEXT PRIMARY KEY,
                content BYTEA
            );",
            name
        );

        Spi::run(&create_if_exists_q).expect("failed to create index table");

        SQLDirectory { name }
    }
}

impl Directory for SQLDirectory {
    fn get_file_handle(&self, path: &Path) -> Result<Arc<dyn FileHandle>, OpenReadError> {
        let file_handle = SQLDirectoryFileHandle {
            sql_directory: self.clone(),
            path: path.to_path_buf(),
        };
        Ok(Arc::new(file_handle))
    }

    fn exists(&self, path: &Path) -> Result<bool, OpenReadError> {
        let query: String = format!(
            "SELECT EXISTS (SELECT 1 FROM {} WHERE path = '{}')",
            self.name,
            path.display()
        );
        Spi::connect(|client| {
            let mut result: bool = false;
            let res = client.select(&query, None, None).unwrap();
            for row in res {
                let exists = row["exists"].value::<bool>().expect("no content").unwrap();
                result = exists
            }
            Ok(result)
        })
    }

    fn delete(&self, path: &Path) -> Result<(), DeleteError> {
        let query = format!(
            "DELETE FROM {} WHERE path = '{}'",
            self.name,
            path.display()
        );
        Spi::run(&query).expect("failed to delete file");
        Ok(())
    }

    fn atomic_read(&self, path: &Path) -> Result<Vec<u8>, OpenReadError> {
        let query: String = format!(
            "SELECT content FROM {} WHERE path = '{}'",
            self.name,
            path.display()
        );

        Spi::connect(|client| {
            let mut result: Vec<u8> = Vec::new();
            let res = client.select(&query, None, None);

            match res {
                Ok(res) => {
                    for row in res {
                        let content = row["content"]
                            .value::<Vec<u8>>()
                            .expect("no content")
                            .unwrap();
                        result = content
                    }

                    if result.is_empty() {
                        Err(OpenReadError::FileDoesNotExist(path.to_path_buf()))
                    } else {
                        Ok(result)
                    }
                }
                Err(_) => Err(OpenReadError::FileDoesNotExist(path.to_path_buf())),
            }
        })
    }

    fn atomic_write(&self, path: &Path, data: &[u8]) -> Result<(), std::io::Error> {
        let query = format!(
            "INSERT INTO {} (path, content) VALUES ('{}', $1) ON CONFLICT (path) DO UPDATE SET content = $1",
            self.name,
            path.display()
        );
        let args = vec![(pgrx::PgBuiltInOids::BYTEAOID.oid(), data.into_datum())];

        Spi::run_with_args(&query, Some(args)).expect("failed to write file");
        Ok(())
    }

    fn open_write(&self, path: &Path) -> Result<WritePtr, OpenWriteError> {
        let sql_writer = SQLWriter::new(self.name.clone(), path.to_path_buf());
        let boxed_terminating_write: Box<dyn TerminatingWrite + 'static> = Box::new(sql_writer);
        Ok(WritePtr::new(boxed_terminating_write))
    }

    fn sync_directory(&self) -> std::io::Result<()> {
        Ok(())
    }

    fn watch(&self, _watch_callback: WatchCallback) -> tantivy::Result<WatchHandle> {
        // Reload Policy for index reader must be set to Manual, since this method
        // is unimplemented
        // https://docs.rs/tantivy/latest/tantivy/enum.ReloadPolicy.html#variant.Manual
        todo!()
    }
}
