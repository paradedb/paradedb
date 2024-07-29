// Copyright (c) 2023-2024 Retake, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use crate::globals::WRITER_GLOBAL;

use super::{transfer::WriterTransferProducer, ServerRequest, WriterClient};
use anyhow::Result;
use serde::Serialize;
use std::{marker::PhantomData, net::SocketAddr, panic, path::Path};
use thiserror::Error;

// static void
// PgSearchWALDataInitMeta(PgSearchWALData *data)
// {
// 	if (RelationGetNumberOfBlocks(data->index) == 0)
// 	{
// 		data->meta.buffer =
// 			PgSearchWALReadLockedBuffer(data->index, P_NEW, BUFFER_LOCK_EXCLUSIVE);
// 		data->buffers[data->nBuffers++] = data->meta.buffer;
// 		data->meta.page = GenericXLogRegisterBuffer(
// 			data->state, data->meta.buffer, GENERIC_XLOG_FULL_IMAGE);
// 		PageInit(data->meta.page, BLCKSZ, sizeof(PgSearchWALMetaPageSpecial));
// 		data->meta.pageSpecial =
// 			(PgSearchWALMetaPageSpecial *) PageGetSpecialPointer(data->meta.page);
// 		data->meta.pageSpecial->next = PGSEARCH_WAL_META_PAGE_BLOCK_NUMBER + 1;
// 		data->meta.pageSpecial->max = data->meta.pageSpecial->next + 1;
// 		data->meta.pageSpecial->version = PGSEARCH_WAL_META_PAGE_SPECIAL_VERSION;
// 	}
// 	else
// 	{
// 		data->meta.buffer =
// 			PgSearchWALReadLockedBuffer(data->index,
// 									PGSEARCH_WAL_META_PAGE_BLOCK_NUMBER,
// 									BUFFER_LOCK_EXCLUSIVE);
// 		data->buffers[data->nBuffers++] = data->meta.buffer;
// 		data->meta.page =
// 			GenericXLogRegisterBuffer(data->state, data->meta.buffer, 0);
// 		data->meta.pageSpecial =
// 			(PgSearchWALMetaPageSpecial *) PageGetSpecialPointer(data->meta.page);
// 	}
// }

// static void
// PgSearchWALPageWriterEnsureCurrent(PgSearchWALData *data)
// {
// 	PgSearchWALMetaPageSpecial *meta;

// 	if (!BufferIsInvalid(data->current.buffer))
// 		return;

// 	if (data->nUsedPages == MAX_GENERIC_XLOG_PAGES)
// 	{
// 		PgSearchWALDataRestart(data);
// 	}

// 	meta = data->meta.pageSpecial;
// 	if (RelationGetNumberOfBlocks(data->index) <= meta->next)
// 	{
// 		data->current.buffer =
// 			PgSearchWALReadLockedBuffer(data->index, P_NEW, BUFFER_LOCK_EXCLUSIVE);
// 		data->buffers[data->nBuffers++] = data->current.buffer;
// 		meta->next = BufferGetBlockNumber(data->current.buffer);
// 		data->current.page = GenericXLogRegisterBuffer(
// 			data->state, data->current.buffer, GENERIC_XLOG_FULL_IMAGE);
// 		PageInit(data->current.page, BLCKSZ, 0);
// 	}
// 	else
// 	{
// 		data->current.buffer = PgSearchWALReadLockedBuffer(
// 			data->index, meta->next, BUFFER_LOCK_EXCLUSIVE);
// 		data->buffers[data->nBuffers++] = data->current.buffer;
// 		data->current.page =
// 			GenericXLogRegisterBuffer(data->state, data->current.buffer, 0);
// 		if (PgSearchWALPageGetFreeSize(data->current.page) == 0)
// 			PageInit(data->current.page, BLCKSZ, 0);
// 	}

// 	data->nUsedPages++;
// }

// static void
// PgSearchWALDataInitBuffers(PgSearchWALData *data)
// {
// 	size_t i;
// 	data->nBuffers = 0;
// 	for (i = 0; i < MAX_GENERIC_XLOG_PAGES; i++)
// 	{
// 		data->buffers[i] = InvalidBuffer;
// 	}
// }

// struct WalState<'a> {
//     num_buffers: u32,
//     buffers: [u32; pg_sys::MAX_GENERIC_XLOG_PAGES as usize],
//     current_buffer: Option<pg_sys::Buffer>,
//     used_pages: u32,
//     relation: &'a pgrx::PgRelation,
// }

// impl<'a> WalState<'a> {
//     fn new(relation: &'a pgrx::PgRelation) -> Self {
//         Self {
//             num_buffers: 0,
//             buffers: [pg_sys::InvalidBuffer; pg_sys::MAX_GENERIC_XLOG_PAGES as usize],
//             current_buffer: None,
//             // Initialized to 1 because the "meta page" is created automatically.
//             used_pages: 1,
//             relation
//         }
//     }
//     fn num_blocks(&self) -> u32 {
//         unsafe {
//             pg_sys::RelationGetNumberOfBlocksInFork(
//                 self.relation.as_ptr(),
//                 pg_sys::ForkNumber_MAIN_FORKNUM,
//             )
//         }
//     }

//     fn buffer_is_invalid(&self) -> bool {
//         self.current_buffer != (pg_sys::InvalidBuffer as pg_sys::Buffer)
//     }

//     fn too_many_pages(&self) -> bool {
//         self.used_pages >= pg_sys::MAX_GENERIC_XLOG_PAGES
//     }

//     fn new_buffer_page() ->

//     fn ensure_current(&self) -> Result<()> {
//         if self.buffer_is_invalid() {
//             return Ok(()); // Must be an InvalidBuffer.
//         };

//         if self.too_many_pages() {
//             // TODO: Run restart routine.
//         }

//         Ok(())
//     }
// }

// #[derive(Serialize, Deserialize)]
// enum WalRecord {
//     Xid {
//         oxid: u64,
//         transaction_id: u32,
//     },
//     Commit {
//         xmin: u64,
//     },
//     Rollback {
//         xmin: u64,
//     },
//     JointCommit {
//         xid: u32,
//         xmin: u64,
//     },
//     Relation {
//         ix_type: u8,
//         datoid: u32,
//         reloid: u32,
//         relnode: u32,
//     },
//     // Add other record types as needed
// }
// fn replay_container(data: &[u8], _single: bool, _xlog_rec_ptr: u64) -> Result<()> {
//     let mut ptr = data;
//     while !ptr.is_empty() {
//         // Assuming each record is prefixed with its length (u32) in bytes
//         let (length_bytes, rest) = ptr.split_at(std::mem::size_of::<u32>());
//         let length = u32::from_le_bytes(length_bytes.try_into().unwrap()) as usize;

//         let (record_bytes, rest) = rest.split_at(length);
//         ptr = rest;

//         let record: WalRecord = serde_json::from_slice(record_bytes)?;

//         match record {
//             WalRecord::Xid {
//                 oxid,
//                 transaction_id,
//             } => {
//                 println!(
//                     "Processing Xid: oxid={}, transaction_id={}",
//                     oxid, transaction_id
//                 );
//                 // Handle XID record
//             }
//             WalRecord::Commit { xmin } => {
//                 println!("Processing Commit: xmin={}", xmin);
//                 // Handle Commit record
//             }
//             WalRecord::Rollback { xmin } => {
//                 println!("Processing Rollback: xmin={}", xmin);
//                 // Handle Rollback record
//             }
//             WalRecord::JointCommit { xid, xmin } => {
//                 println!("Processing JointCommit: xid={}, xmin={}", xid, xmin);
//                 // Handle JointCommit record
//             }
//             WalRecord::Relation {
//                 ix_type,
//                 datoid,
//                 reloid,
//                 relnode,
//             } => {
//                 println!(
//                     "Processing Relation: ix_type={}, datoid={}, reloid={}, relnode={}",
//                     ix_type, datoid, reloid, relnode
//                 );
//                 // Handle Relation record
//             } // Add other record types as needed
//         }
//     }
//     Ok(())
// }

// trait ClientService {
//     fn call<T: Serialize>(&self, request: ServerRequest<T>) -> Result<ServerRequest<T>>;
// }

// struct WalService<'a> {
//     relation: &'a pgrx::PgRelation,
// }

// impl<'a> ClientService for WalService<'a> {
//     fn call<T: Serialize>(&self, request: ServerRequest<T>) -> Result<ServerRequest<T>> {
//         // PGroogna calls this first. Have to decide if its needed?
//         // PgSearchWALApply(index);

//         // We can use any block number for this. We just want an index
//         // level lock but we can't use LockRelation(index) because it
//         // conflicts with REINDEX INDEX CONCURRENTLY.

//         let blockno: pg_sys::BlockNumber = 0;
//         let lockmode = unsafe {
//             if pg_sys::RecoveryInProgress() {
//                 pg_sys::RowExclusiveLock as pg_sys::LOCKMODE
//             } else {
//                 pg_sys::ShareUpdateExclusiveLock as pg_sys::LOCKMODE
//             }
//         };
//         let relation = self.relation.as_ptr();
//         unsafe {
//             pg_sys::LockPage(relation, blockno, lockmode);
//         }

//         // call xlogstart
//         let state = unsafe { pg_sys::GenericXLogStart(relation) };

//         let num_blocks = unsafe {
//             pg_sys::RelationGetNumberOfBlocksInFork(relation, pg_sys::ForkNumber_MAIN_FORKNUM)
//         };

//         let page = if num_blocks == 0 {
//             unsafe {
//                 pg_sys::LockRelationForExtension(
//                     relation,
//                     pg_sys::ExclusiveLock as pg_sys::LOCKMODE,
//                 );

//                 // InvalidBlockNumer is aliased as P_NEW in Postgres codebase, its a special
//                 // code that means "create a new buffer".
//                 let buffer: pg_sys::Buffer =
//                     pg_sys::ReadBuffer(relation, pg_sys::InvalidBlockNumber);

//                 pg_sys::LockBuffer(buffer, pg_sys::BUFFER_LOCK_EXCLUSIVE as pg_sys::LOCKMODE);

//                 pg_sys::UnlockRelationForExtension(
//                     relation,
//                     pg_sys::ExclusiveLock as pg_sys::LOCKMODE,
//                 );

//                 // Flag as GENERIC_XLOG_FULL_IMAGE because this is a new buffer page.
//                 let page = pg_sys::GenericXLogRegisterBuffer(
//                     state,
//                     buffer,
//                     pg_sys::GENERIC_XLOG_FULL_IMAGE as i32,
//                 );
//                 pg_sys::PageInit(page, pg_sys::BLCKSZ as usize, 0);
//                 page
//             }
//         } else {
//             unsafe {
//                 // We can just use the very first block number ("0"), as we've created
//                 // our own buffer page for this.
//                 let buffer: pg_sys::Buffer = pg_sys::ReadBuffer(relation, 0);
//                 pg_sys::LockBuffer(buffer, pg_sys::BUFFER_LOCK_EXCLUSIVE as pg_sys::LOCKMODE);

//                 // If not using GENERIC_XLOG_FULL_IMAGE, you should pass "0" because there are
//                 // no other available flags.
//                 pg_sys::GenericXLogRegisterBuffer(state, buffer, 0)
//             }
//         };

//         unsafe {
//             pg_sys::XLogBeginInsert(); // Needs to be called before RegisterData/Insert.
//             pg_sys::XLogRegisterData();
//             // XLR_SPECIAL_REL_UPDATE needs to be set here because we are modifying
//             // additonal files to the usual relation files. External tools that read WAL
//             // need this to recognize these extra files.
//             pg_sys::XLogInsert(
//                 wal::RESOURCE_MANAGER_ID,
//                 pg_sys::PGSEARCH_WAL_RECORD_INSERT | pg_sys::XLR_SPECIAL_REL_UPDATE,
//             );
//         }

//         // data->state = GenericXLogStart(data->index);

//         // init buffers?
//         // PgSearchWALDataInitBuffers(PgSearchWALData *data)
//         // {
//         // 	size_t i;
//         // 	data->nBuffers = 0;
//         // 	for (i = 0; i < MAX_GENERIC_XLOG_PAGES; i++)
//         // 	{
//         // 		data->buffers[i] = InvalidBuffer;
//         // 	}
//         // }

//         // initializes "n used pages" counter

//         // initializes xlogregisterbuffer
//         // static void
//         // PgSearchWALDataInitMeta(PgSearchWALData *data)
//         // {
//         // 	if (RelationGetNumberOfBlocks(data->index) == 0)
//         // 	{
//         // 		data->meta.buffer =
//         // 			PgSearchWALReadLockedBuffer(data->index, P_NEW, BUFFER_LOCK_EXCLUSIVE);
//         // 		data->buffers[data->nBuffers++] = data->meta.buffer;
//         // 		data->meta.page = GenericXLogRegisterBuffer(
//         // 			data->state, data->meta.buffer, GENERIC_XLOG_FULL_IMAGE);
//         // 		PageInit(data->meta.page, BLCKSZ, sizeof(PgSearchWALMetaPageSpecial));
//         // 		data->meta.pageSpecial =
//         // 			(PgSearchWALMetaPageSpecial *) PageGetSpecialPointer(data->meta.page);
//         // 		data->meta.pageSpecial->next = PGSEARCH_WAL_META_PAGE_BLOCK_NUMBER + 1;
//         // 		data->meta.pageSpecial->max = data->meta.pageSpecial->next + 1;
//         // 		data->meta.pageSpecial->version = PGSEARCH_WAL_META_PAGE_SPECIAL_VERSION;
//         // 	}
//         // 	else
//         // 	{
//         // 		data->meta.buffer =
//         // 			PgSearchWALReadLockedBuffer(data->index,
//         // 									PGSEARCH_WAL_META_PAGE_BLOCK_NUMBER,
//         // 									BUFFER_LOCK_EXCLUSIVE);
//         // 		data->buffers[data->nBuffers++] = data->meta.buffer;
//         // 		data->meta.page =
//         // 			GenericXLogRegisterBuffer(data->state, data->meta.buffer, 0);
//         // 		data->meta.pageSpecial =
//         // 			(PgSearchWALMetaPageSpecial *) PageGetSpecialPointer(data->meta.page);
//         // 	}
//         // }

//         // initializes a "current buffer" to null or something

//         // writes the page data
//         // static int
//         // PgSearchWALPageWriter(void *userData, const char *buffer, size_t length)
//         // {
//         // 	PgSearchWALData *data = userData;
//         // 	int written = 0;
//         // 	size_t rest = length;

//         // 	while (written < length)
//         // 	{
//         // 		size_t freeSize;

//         // 		PgSearchWALPageWriterEnsureCurrent(data);

//         // 		freeSize = PgSearchWALPageGetFreeSize(data->current.page);
//         // 		if (rest <= freeSize)
//         // 		{
//         // 			PgSearchWALPageAppend(data->current.page, buffer, rest);
//         // 			written += rest;
//         // 		}
//         // 		else
//         // 		{
//         // 			PgSearchWALPageAppend(data->current.page, buffer, freeSize);
//         // 			written += freeSize;
//         // 			rest -= freeSize;
//         // 			buffer += freeSize;
//         // 		}

//         // 		if (PgSearchWALPageGetFreeSize(data->current.page) == 0)
//         // 		{
//         // 			PgSearchWALPageFilled(data);
//         // 			PgSearchWALPageWriterEnsureCurrent(data);
//         // 		}
//         // 	}

//         // 	return written;
//         // }

//         Ok(request)
//     }
// }

pub struct Client<T: Serialize> {
    addr: std::net::SocketAddr,
    http: reqwest::blocking::Client,
    producer: Option<WriterTransferProducer<T>>,
    marker: PhantomData<T>,
}

/// A generic client for communication with background server.
/// The client has two functions, "request" and "transfer".

/// "request" sends a synchronous request and waits for a response.

/// "transfer" sends a request, and then opens a data pipe to the backend.
/// This is useful for transfering large volumes of data, where "request"
/// has too much overhead to be called over and over.

/// A transfer requires exclusive access to the background server, so
/// during a transfer, other connections will block and wait for the
/// background server to become available again.
impl<T: Serialize> Client<T> {
    pub fn new(addr: SocketAddr) -> Self {
        // Some server processes, like creating a index, can take a long time.
        // Because the server is blocking/single-threaded, clients should wait
        // as long as they need to for their turn to use the server.
        let http = reqwest::blocking::ClientBuilder::new()
            .timeout(None)
            .build()
            .expect("error building http client");

        Self {
            addr,
            http,
            producer: None,
            marker: PhantomData,
        }
    }

    pub fn from_global() -> Self {
        let lock = panic::catch_unwind(|| WRITER_GLOBAL.share());

        let addr = match lock {
            Ok(lock) => lock.addr(),
            Err(_) => {
                panic!("Could not get lock on writer. Have you added the extension to the shared preload library list?");
            }
        };

        Self::new(addr)
    }

    fn url(&self) -> String {
        format!("http://{}", self.addr)
    }

    fn send_request(&mut self, request: ServerRequest<T>) -> Result<(), ClientError> {
        // If there is an open pending transfer, stop it so that we can continue
        // with more requests.
        self.stop_transfer();
        let bytes = bincode::serialize(&request).unwrap();
        let response = self.http.post(self.url()).body::<Vec<u8>>(bytes).send()?;

        let result = match response.status() {
            reqwest::StatusCode::OK => Ok(()),
            _ => {
                let err = response.text().map_err(ClientError::ResponseParse)?;
                Err(ClientError::ServerError(err))
            }
        };

        // let json = serde_json::to_string(&request).expect("blew up serializing to json");
        // unsafe {
        //     // Convert the Rust String to a CString
        //     let c_json = CString::new(json).expect("CString::new failed");

        //     // Get the raw pointer from the CString and cast to *mut c_char
        //     let json_ptr = c_json.as_ptr() as *mut c_char;

        //     // Pass the raw pointer to the XLogRegisterData function
        //     pg_sys::XLogBeginInsert();

        //     // Pass the raw pointer to the XLogRegisterData function
        //     pg_sys::XLogRegisterData(json_ptr, c_json.to_bytes().len() as u32);

        //     pg_sys::XLogInsert(wal::RESOURCE_MANAGER_ID, wal::XLOG_RESOURCE_MANAGER_MESSAGE);
        // }

        result
    }

    fn send_transfer<P: AsRef<Path>>(
        &mut self,
        pipe_path: P,
        request: T,
    ) -> Result<(), ClientError> {
        if self.producer.is_none() {
            // Send a request to open a transfer to the server.
            self.send_request(ServerRequest::Transfer(
                pipe_path.as_ref().display().to_string(),
            ))?;
            // Store a new transfer producer in the client state.
            self.producer
                .replace(WriterTransferProducer::new(pipe_path)?);
        }

        // There is an existing producer in client state, use it to send the request.
        self.producer.as_mut().unwrap().write_message(&request)?;
        Ok(())
    }

    /// Stop a data pipe transfer. Must be called when the transfer is done, or
    /// the client + server will both hang forever.
    ///
    /// With insert transactions, it's tricky to know when the transfer is
    /// completely done. Best practice is to call this both during the end of
    /// transaction callback, as well as before every send_request.
    fn stop_transfer(&mut self) {
        // Dropping the producer closes the named pipe file.
        self.producer.take();
    }

    /// Should only be called by shutdown background worker.
    pub fn stop_server(&mut self) -> Result<(), ClientError> {
        self.send_request(ServerRequest::Shutdown)?;
        Ok(())
    }
}

impl<T: Serialize> WriterClient<T> for Client<T> {
    fn request(&mut self, request: T) -> Result<(), ClientError> {
        self.send_request(ServerRequest::Request(request))
    }

    fn transfer<P: AsRef<Path>>(&mut self, pipe_path: P, request: T) -> Result<(), ClientError> {
        self.send_transfer(pipe_path, request)
    }
}

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("could not parse response from writer server: {0}")]
    ResponseParse(reqwest::Error),

    #[error("writer server responded with an error: {0}")]
    ServerError(String),

    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),

    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use crate::fixtures::*;
    use crate::writer::{Client, Server, WriterClient, WriterRequest};
    use rstest::*;
    use std::thread;

    #[rstest]
    #[case::insert_request(WriterRequest::Insert {
        directory: mock_dir().writer_dir,
        document: simple_doc(simple_schema(default_fields())),
    })]
    #[case::commit_request(WriterRequest::Commit { directory: mock_dir().writer_dir })]
    #[case::abort_request(WriterRequest::Abort {directory: mock_dir().writer_dir})]
    #[case::vacuum_request(WriterRequest::Vacuum { directory: mock_dir().writer_dir })]
    #[case::drop_index_request(WriterRequest::DropIndex { directory: mock_dir().writer_dir })]
    /// Test request serialization and transfer between client and server.
    fn test_client_request(#[case] request: WriterRequest) {
        // Create a handler that will test that the received request is the same as sent.
        let request_clone = request.clone();
        let handler = TestHandler::new(move |req: WriterRequest| assert_eq!(&req, &request_clone));
        let mut server = Server::new(handler).unwrap();
        let addr = server.addr();

        // Start the server in a new thread, as it blocks once started.
        thread::spawn(move || {
            server.start().unwrap();
        });

        let mut client: Client<WriterRequest> = Client::new(addr);
        client.request(request.clone()).unwrap();

        // The server must be stopped, or this test will not finish.
        client.stop_server().unwrap();
    }
}
