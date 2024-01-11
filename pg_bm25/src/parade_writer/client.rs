use super::transfer::WriterTransferProducer;
use super::ParadeWriterServer;
use crate::parade_index::index::ParadeIndex;
use crate::WriterInitError;
use crate::{
    json::builder::JsonBuilder,
    parade_writer::{ParadeWriterRequest, ParadeWriterResponse},
};
use once_cell::sync::Lazy;
use pgrx::{log, PGRXSharedMemory};
use std::{error::Error, net::SocketAddr};
use tantivy::schema::Field;

/// A cache for the reqwest HTTP client so that it can be used over the lifespan of this connection.
static mut HTTP_CLIENT: Option<reqwest::blocking::Client> = None;

/// A cache to use for the transfer producer being used in the current transaction.
static mut WRITER_TRANSFER_PRODUCER: Option<WriterTransferProducer<JsonBuilder>> = None;

static mut SERVER: Lazy<ParadeWriterServer> = Lazy::new(|| ParadeWriterServer::new());

#[derive(Copy, Clone, Default)]
pub struct WriterStatus {
    pub addr: Option<SocketAddr>,
    pub error: Option<WriterInitError>,
}

impl WriterStatus {
    pub fn addr(&self) -> SocketAddr {
        self.addr
            .expect("could not access writer status, writer server may not have started.")
    }
    pub fn set_addr(&mut self, addr: SocketAddr) {
        self.addr = Some(addr);
    }

    pub fn set_error(&mut self, err: WriterInitError) {
        self.error = Some(err);
    }

    fn send_request(
        &self,
        request: ParadeWriterRequest,
    ) -> Result<ParadeWriterResponse, Box<dyn Error>> {
        let addr = match self.addr {
            // If there's no addr, the server hasn't started yet.
            // We won't send the shutdown request, but it is up to the insert worker
            // to handle this case by checking for SIGTERM right before starting its server.
            None => match request {
                ParadeWriterRequest::Shutdown => {
                    log!("pg_bm25 shutdown worker skipped sending signal to insert worker");
                    return Ok(ParadeWriterResponse::Ok);
                }
                // If it wasn't a shutdown request, then we have a problem if the server has not
                // been started. Return an error.
                req => {
                    return Err(format!(
                        "pg_bm25 writer not yet initialized, but received request: {req:?}"
                    )
                    .into())
                }
            },
            Some(addr) => addr,
        };

        let bytes: Vec<u8> = request.into();
        let client = unsafe { HTTP_CLIENT.get_or_insert_with(|| reqwest::blocking::Client::new()) };
        let response = client.post(format!("http://{addr}")).body(bytes).send()?;
        let response_body = response.bytes()?;
        ParadeWriterResponse::try_from(response_body.to_vec().as_slice()).map_err(|e| e.into())
    }

    fn get_index_directory(name: &str) -> String {
        crate::env::paradedb_data_dir_path()
            .join(name)
            .display()
            .to_string()
    }

    fn initialize_insert(&self, index_name: &str) {
        unsafe {
            // Nothing to do if the named pipe is already initialized.
            // This will be the case if this is not the first row being inserted
            // for the current transaction.
            if WRITER_TRANSFER_PRODUCER.is_some() {
                return;
            }

            // This will take the File out of the static variable, and drop it.
            // When the File is dropped the fifo stream will close.
            pgrx::register_xact_callback(pgrx::PgXactCallbackEvent::Commit, || {
                pgrx::log!("WE'RE DONE HERE CLEANING UP THE PIPE ON COMMIT");
                WRITER_TRANSFER_PRODUCER.take(); // Make sure the file ref is dropped.
            });

            pgrx::register_xact_callback(pgrx::PgXactCallbackEvent::Abort, || {
                pgrx::log!("WE'RE DONE HERE CLEANING UP THE PIPE ON ABORT");
                WRITER_TRANSFER_PRODUCER.take(); // Make sure the file ref is dropped.
            });

            // Let the writer server know that we're trying to insert. If it's ready,
            // it will respond with the path to a unix named pipe that this client can
            // write its data to.
            let response = self
                .send_request(ParadeWriterRequest::Insert(Self::get_index_directory(
                    index_name,
                )))
                .expect("error while sending insert request");

            match response {
                ParadeWriterResponse::Ok => {
                    pgrx::log!("got insert ok response from writer, creating producer...");
                    // It's this client's turn to insert its data. We create a unix named
                    // pipe to send a file through. This will have the same interface as
                    // writing to a normal File.
                    let producer = WriterTransferProducer::new()
                        .expect("could not create writer transfer producer");

                    WRITER_TRANSFER_PRODUCER.replace(producer);
                }
                error => {
                    panic!("unexpected error while initializing insert into index {index_name}: {error:?}")
                }
            };
        }
    }

    fn insert_impl(json_builder: JsonBuilder) -> Result<(), Box<dyn Error>> {
        // Serialize the json_builder and writer the bytes to the FIFO.
        let json_bytes = serde_json::to_vec(&json_builder)
            .unwrap_or_else(|err| panic!("could not serialize json_builder into bytes: {err:?}"));

        let producer = unsafe {
            WRITER_TRANSFER_PRODUCER
                .as_mut()
                .expect("expected named pipe to be initialized")
        };

        producer.write_message(&json_builder)?;

        Ok(())
    }

    pub fn insert(&self, index_name: &str, json_builder: JsonBuilder) {
        // We run the initialize_insert every time, because we don't know whether this
        // is the first insert in the transaction. It's a no-op on subsequent inserts.
        self.initialize_insert(index_name);

        // We wrap the json_builder in a Some here, because the server expects to receive
        // an Option<json_builder>, allowing it to close the connection when it receives None.
        Self::insert_impl(json_builder).unwrap_or_else(|err| {
            panic!("could not write to named pipe for index {index_name}: {err:?}")
        });
    }

    pub fn delete(&self, index_name: &str, ctid_field: Field, ctid_values: Vec<u64>) {
        let data_directory = Self::get_index_directory(index_name);
        let response = self
            .send_request(ParadeWriterRequest::Delete(
                data_directory.clone(),
                ctid_field,
                ctid_values,
            ))
            .expect("error while sending delete request");

        match response {
            ParadeWriterResponse::Ok => {}
            error => {
                panic!("unexpected error while deleting from index at {data_directory}: {error:?}")
            }
        };
    }

    pub fn commit(&self, index_name: &str) {
        // let data_directory = Self::get_data_directory(index_name);
        // let response = self
        //     .send_request(ParadeWriterRequest::Commit(data_directory.clone()))
        //     .expect("error while sending commit request");

        // match response {
        //     ParadeWriterResponse::Ok => {}
        //     error => {
        //         panic!("unexpected error while committing to index at {data_directory}: {error:?}")
        //     }
        // };
    }

    pub fn vacuum(&self, index_name: &str) {
        let data_directory = Self::get_index_directory(index_name);
        let response = self
            .send_request(ParadeWriterRequest::Vacuum(data_directory.clone()))
            .expect("error while sending vacuum request}");

        match response {
            ParadeWriterResponse::Ok => {}
            error => {
                panic!("unexpected error while vacuuming index at {data_directory}: {error:?}")
            }
        };
    }

    pub fn drop_index(&self, index_name: &str) {
        // The background worker will delete any file path we give it as part of its cleanup.
        // Here we define the paths we need gone.

        let mut paths_to_delete = Vec::new();
        let data_directory = Self::get_index_directory(index_name);
        let field_configs_file = ParadeIndex::get_field_configs_path(&data_directory);
        let tantivy_writer_lock = format!("{data_directory}/.tantivy-writer.lock");
        let tantivy_meta_lock = format!("{data_directory}/.tantivy-meta.lock");

        // The background worker will correctly order paths for safe deletion, so order
        // here doesn't matter.
        paths_to_delete.push(tantivy_writer_lock);
        paths_to_delete.push(tantivy_meta_lock);
        paths_to_delete.push(field_configs_file);
        paths_to_delete.push(data_directory.clone());

        let response = self
            .send_request(ParadeWriterRequest::DropIndex(
                data_directory.clone(),
                paths_to_delete,
            ))
            .expect("error while sending drop index request");

        match response {
            ParadeWriterResponse::Ok => {}
            error => {
                panic!("unexpected error while dropping index at {data_directory}: {error:?}")
            }
        };
    }

    pub fn shutdown(&self) -> Result<(), Box<dyn Error>> {
        self.send_request(ParadeWriterRequest::Shutdown)?;
        Ok(())
    }
}

unsafe impl PGRXSharedMemory for WriterStatus {}
