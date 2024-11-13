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

use anyhow::Result;
use pgrx::prelude::*;
use pgrx::PgRelation;

use crate::index::{SearchFs, SearchIndex, WriterDirectory};
use crate::postgres::index::{open_search_index, relfilenode_from_pg_relation};

#[pg_extern(sql = "
CREATE OR REPLACE PROCEDURE paradedb.delete_bm25_index_by_oid(
    index_oid oid
)
LANGUAGE c AS 'MODULE_PATHNAME', '@FUNCTION_NAME@';
")]
unsafe fn delete_bm25_index_by_oid(index_oid: pg_sys::Oid) -> Result<()> {
    let database_oid = crate::MyDatabaseId();
    let relfile_paths = WriterDirectory::relfile_paths(database_oid, index_oid.as_u32())
        .expect("could not look up pg_search relfilenode directory");

    for directory in relfile_paths {
        // Drop the Tantivy data directory.
        // It's expected that this will be queued to actually perform the delete upon
        // transaction commit.
        match SearchIndex::from_disk(&directory) {
            Ok(mut search_index) => {
                search_index.drop_index().unwrap_or_else(|err| {
                    panic!("error dropping index with OID {index_oid:?}: {err:?}")
                });
            }
            Err(e) => {
                pgrx::warning!(
                    "error dropping index with OID {index_oid:?} at path {}: {e:?}",
                    directory.search_index_dir_path(false).unwrap().0.display()
                );
            }
        }
    }
    Ok(())
}

#[pg_extern]
fn index_size(index: PgRelation) -> Result<i64> {
    // # Safety
    //
    // Lock the index relation until the end of this function so it is not dropped or
    // altered while we are reading it.
    //
    // Because we accept a PgRelation above, we have confidence that Postgres has already
    // validated the existence of the relation. We are safe calling the function below as
    // long we do not pass pg_sys::NoLock without any other locking mechanism of our own.
    let index = unsafe { PgRelation::with_lock(index.oid(), pg_sys::AccessShareLock as _) };
    let index_oid = index.oid();

    let database_oid = crate::MyDatabaseId();
    let relfilenode = relfilenode_from_pg_relation(&index);

    // Create a WriterDirectory with the obtained index_oid
    let writer_directory =
        WriterDirectory::from_oids(database_oid, index_oid.as_u32(), relfilenode.as_u32());

    // Call the total_size method to get the size in bytes
    let total_size = writer_directory.total_size()?;

    Ok(total_size as i64)
}

#[pg_extern]
fn index_info(
    index: PgRelation,
) -> anyhow::Result<
    TableIterator<
        'static,
        (
            name!(segno, String),
            name!(byte_size, i64),
            name!(num_docs, i64),
            name!(num_deleted, i64),
        ),
    >,
> {
    // # Safety
    //
    // Lock the index relation until the end of this function so it is not dropped or
    // altered while we are reading it.
    //
    // Because we accept a PgRelation above, we have confidence that Postgres has already
    // validated the existence of the relation. We are safe calling the function below as
    // long we do not pass pg_sys::NoLock without any other locking mechanism of our own.
    let index = unsafe { PgRelation::with_lock(index.oid(), pg_sys::AccessShareLock as _) };

    // open the specified index
    let index = open_search_index(&index).expect("should be able to open search index");
    let directory = index.directory.clone();
    let data = index
        .underlying_index
        .searchable_segment_metas()?
        .into_iter()
        .map(|meta| {
            let segno = meta.id().short_uuid_string();
            let byte_size = meta
                .list_files()
                .into_iter()
                .map(|file| {
                    let mut full_path = directory.tantivy_dir_path(false).unwrap().0;
                    full_path.push(file);

                    if full_path.exists() {
                        full_path
                            .metadata()
                            .map(|metadata| metadata.len())
                            .unwrap_or(0)
                    } else {
                        0
                    }
                })
                .sum::<u64>() as i64;
            let num_docs = meta.num_docs() as i64;
            let num_deleted = meta.num_deleted_docs() as i64;

            (segno, byte_size, num_docs, num_deleted)
        })
        .collect::<Vec<_>>();

    Ok(TableIterator::new(data))
}

extension_sql!(
    r#"
    CREATE OR REPLACE FUNCTION paradedb.drop_bm25_event_trigger()
    RETURNS event_trigger AS $$
    DECLARE
        obj RECORD;
    BEGIN
        FOR obj IN SELECT * FROM pg_event_trigger_dropped_objects() LOOP
            IF obj.object_type = 'index' THEN
                CALL paradedb.delete_bm25_index_by_oid(obj.objid);
            END IF;
        END LOOP;
    END;
    $$ LANGUAGE plpgsql;
    
    CREATE EVENT TRIGGER trigger_on_sql_index_drop
    ON sql_drop
    EXECUTE FUNCTION paradedb.drop_bm25_event_trigger();
    "#
    name = "create_drop_bm25_event_trigger",
    requires = [ delete_bm25_index_by_oid ]
);
