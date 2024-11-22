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

use crate::index::channel::{ChannelDirectory, ChannelRequest, ChannelRequestHandler};
use crate::index::directory::blocking::BlockingDirectory;
use crate::index::WriterResources;
use crate::postgres::options::SearchIndexCreateOptions;
use crate::{
    index::SearchIndex,
    postgres::types::TantivyValueError,
    schema::{
        SearchDocument, SearchFieldConfig, SearchFieldName, SearchFieldType, SearchIndexSchema,
    },
};
use anyhow::Result;
use std::time::Duration;
use tantivy::store::Compressor;
use tantivy::{Directory, Index};
use tantivy::{IndexSettings, IndexWriter};
use thiserror::Error;

/// The entity that interfaces with Tantivy indexes.
pub struct SearchIndexWriter {
    pub underlying_writer: IndexWriter,
    pub wants_merge: bool,
    handler: ChannelRequestHandler,
}

impl SearchIndexWriter {
    pub fn new(
        index: &Index,
        resources: WriterResources,
        index_options: &SearchIndexCreateOptions,
        mut handler: ChannelRequestHandler,
    ) -> Result<Self> {
        let underlying_writer = std::thread::scope(|scope| {
            let scope_handle = scope.spawn(|| {
                let (parallelism, memory_budget, _target_segment_count, _merge_on_insert) =
                    resources.resources(index_options);
                eprintln!(
                    "TODO: why do we have to divide the memory budget: {}",
                    memory_budget
                );
                index.writer_with_num_threads(1, memory_budget / parallelism)
            });

            while !scope_handle.is_finished() {
                match handler.try_recv() {
                    Ok(true) => break,
                    Ok(false) => continue,
                    Err(e) => {
                        if let Some(_) = e.downcast_ref::<crossbeam::channel::TryRecvError>() {
                            continue;
                        } else {
                            return Err(e);
                        }
                    }
                }
            }

            scope_handle
                .join()
                .expect("SearchIndexWriter::new():  scoped thread join should not fail")
                .map_err(|e| anyhow::Error::from(e))
        })?;

        pgrx::warning!("got writer");
        Ok(Self {
            underlying_writer,
            // TODO: Merge on insert
            wants_merge: false,
            handler: handler.clone(),
        })
    }

    pub fn insert(&mut self, document: SearchDocument) -> Result<()> {
        // Add the Tantivy document to the index.
        let tantivy_document: tantivy::TantivyDocument = document.into();
        let _opstamp = std::thread::scope(|scope| {
            let scope_handle =
                scope.spawn(|| self.underlying_writer.add_document(tantivy_document));

            while !scope_handle.is_finished() {
                match self.handler.try_recv() {
                    Ok(true) => break,
                    Ok(false) => continue,
                    Err(e) => {
                        if let Some(_) = e.downcast_ref::<crossbeam::channel::TryRecvError>() {
                            continue;
                        } else {
                            return Err(e);
                        }
                    }
                }
            }

            scope_handle
                .join()
                .expect("SearchIndexWriter::insert():  scoped thread join should not fail")
                .map_err(|e| anyhow::Error::from(e))
        })
        .map_err(|e| anyhow::Error::from(e))?;

        Ok(())
    }

    pub fn commit(mut self) -> Result<()> {
        pgrx::warning!("starting commit");
        let mut n = 0;
        while let Ok(false) = self.handler.recv_timeout(Duration::from_millis(100)) {
            n += 1
        }
        pgrx::warning!("received {n} messages");
        let _stats = std::thread::scope(|scope| {
            let scope_handle = scope.spawn(|| self.underlying_writer.commit());

            while !scope_handle.is_finished() {
                pgrx::warning!("waiting for commit to finish");
                match self.handler.try_recv() {
                    Ok(true) => break,
                    Ok(false) => continue,
                    Err(e) => {
                        if let Some(_) = e.downcast_ref::<crossbeam::channel::TryRecvError>() {
                            continue;
                        } else {
                            return Err(e);
                        }
                    }
                }
            }

            scope_handle
                .join()
                .expect("SearchIndexWriter::commit():  scoped thread join should not fail")
                .map_err(|e| anyhow::Error::from(e))
        })?;

        Ok(())

        // self.underlying_writer.commit()?;

        // self.current_opstamp += 1;
        // let max_doc = self.underlying_writer.max_doc();
        // self.underlying_writer.finalize()?;
        // let segment = self.segment.with_max_doc(max_doc);
        // let index = segment.index();
        //
        // let _lock = index.directory().acquire_lock(&Lock {
        //     filepath: META_LOCK.filepath.clone(),
        //     is_blocking: true,
        // });
        //
        // let committed_meta = index.load_metas()?;
        // let mut segments = committed_meta.segments.clone();
        // segments.push(segment.meta().clone());
        //
        // let new_meta = tantivy::IndexMeta {
        //     segments,
        //     opstamp: self.current_opstamp,
        //     index_settings: committed_meta.index_settings,
        //     schema: committed_meta.schema,
        //     payload: committed_meta.payload,
        // };
        //
        // index
        //     .directory()
        //     .atomic_write(*META_FILEPATH, &serde_json::to_vec(&new_meta)?)?;
        //
        // Ok(())
    }

    pub fn create_index(
        index_oid: pgrx::pg_sys::Oid,
        fields: Vec<(SearchFieldName, SearchFieldConfig, SearchFieldType)>,
        key_field_index: usize,
    ) -> Result<SearchIndex> {
        let (request_sender, request_receiver) = crossbeam::channel::unbounded();
        let (response_sender, response_receiver) = crossbeam::channel::unbounded();

        let schema = SearchIndexSchema::new(fields, key_field_index)?;
        let blocking_dir = BlockingDirectory::new(index_oid);
        let handler =
            ChannelRequestHandler::open(blocking_dir, index_oid, response_sender, request_receiver);
        let channel_dir = ChannelDirectory::new(request_sender, response_receiver);

        let settings = IndexSettings {
            docstore_compress_dedicated_thread: false,
            ..IndexSettings::default()
        };
        let mut underlying_index = Index::create(channel_dir, schema.schema.clone(), settings)?;

        SearchIndex::setup_tokenizers(&mut underlying_index, &schema);
        Ok(SearchIndex {
            index_oid,
            handler,
            underlying_index,
            schema,
        })
    }
}

#[derive(Error, Debug)]
pub enum IndexError {
    #[error(transparent)]
    TantivyError(#[from] tantivy::TantivyError),

    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    #[error(transparent)]
    TantivyValueError(#[from] TantivyValueError),

    #[error("key_field column '{0}' cannot be NULL")]
    KeyIdNull(String),
}
