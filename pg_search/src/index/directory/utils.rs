use crate::index::mvcc::{MvccSatisfies, PinCushion};
use crate::postgres::storage::block::{
    DeleteEntry, FileEntry, LinkedList, MVCCEntry, PgItem, SegmentFileDetails, SegmentMetaEntry,
    SCHEMA_START, SEGMENT_METAS_START, SETTINGS_START,
};
use crate::postgres::storage::buffer::BufferManager;
use crate::postgres::storage::{LinkedBytesList, LinkedItemList};
use anyhow::Result;
use pgrx::pg_sys;
use rustc_hash::{FxHashMap, FxHashSet};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tantivy::index::SegmentComponent;
use tantivy::{
    index::{DeleteMeta, IndexSettings, InnerSegmentMeta, SegmentId, SegmentMetaInventory},
    schema::Schema,
    IndexMeta,
};

pub fn save_schema(relation_oid: pg_sys::Oid, tantivy_schema: &Schema) -> Result<()> {
    let mut schema = LinkedBytesList::open(relation_oid, SCHEMA_START);
    if schema.is_empty() {
        let bytes = serde_json::to_vec(tantivy_schema)?;
        unsafe {
            let _ = schema.write(&bytes)?;
        }
    }
    Ok(())
}

pub fn save_settings(relation_oid: pg_sys::Oid, tantivy_settings: &IndexSettings) -> Result<()> {
    let mut settings = LinkedBytesList::open(relation_oid, SETTINGS_START);
    if settings.is_empty() {
        let bytes = serde_json::to_vec(tantivy_settings)?;
        unsafe {
            let _ = settings.write(&bytes)?;
        }
    }
    Ok(())
}

pub unsafe fn save_new_metas(
    relation_oid: pg_sys::Oid,
    new_meta: &IndexMeta,
    prev_meta: &IndexMeta,
    directory_entries: &mut FxHashMap<PathBuf, FileEntry>,
) -> Result<()> {
    // in order to ensure that all of our mutations to the list of segments appear atomically on
    // physical replicas, we atomically operate on a deep copy of the list.
    let mut linked_list =
        LinkedItemList::<SegmentMetaEntry>::open(relation_oid, SEGMENT_METAS_START).atomically();

    let incoming_segments = new_meta
        .segments
        .iter()
        .map(|s| (s.id(), s))
        .collect::<FxHashMap<_, _>>();
    let new_ids = new_meta
        .segments
        .iter()
        .map(|s| s.id())
        .collect::<FxHashSet<_>>();
    let previous_ids = prev_meta
        .segments
        .iter()
        .map(|s| s.id())
        .collect::<FxHashSet<_>>();

    // first, reorganize the directory_entries by segment id
    let mut new_files =
        FxHashMap::<SegmentId, FxHashMap<SegmentComponent, (FileEntry, PathBuf)>>::default();

    for (path, file_entry) in directory_entries.drain() {
        let segment_id = path.segment_id();
        let component_type = path.component_type();

        if let (Some(segment_id), Some(component_type)) = (segment_id, component_type) {
            new_files
                .entry(segment_id)
                .or_default()
                .insert(component_type, (file_entry, path));
        } else {
            panic!("malformed PathBuf: {}", path.display());
        }
    }

    let created_ids = new_ids.difference(&previous_ids).collect::<Vec<_>>();
    let mut modified_ids = previous_ids.intersection(&new_ids).collect::<Vec<_>>();
    let deleted_ids = previous_ids.difference(&new_ids).collect::<Vec<_>>();

    modified_ids.retain(|id| {
        if let Some(new_files) = new_files.get(id) {
            new_files.contains_key(&SegmentComponent::Delete)
        } else {
            false
        }
    });

    //
    // process the new segments
    //
    // these are added to the linked list as new items
    // xmin is set to the current transaction id
    // and xmax is set to InvalidTransactionId
    //
    let created_entries = created_ids
        .into_iter()
        .filter_map(|id| {
            let created_segment = incoming_segments.get(id).unwrap();
            let mut files = new_files.remove(id)?;

            let meta_entry = SegmentMetaEntry {
                segment_id: *id,
                max_doc: created_segment.max_doc(),
                xmin: if deleted_ids.is_empty() {
                    // this is a new segment created by a tantivy index .commit()
                    pg_sys::GetCurrentTransactionId()
                } else {
                    // this is a new segment created by a tantivy merge.  in this case it's imperative
                    // this segment, which contains tuples that have previously bee on disk, be
                    // written to a transaction that is known to be "not in progress anymore".  This
                    // is because the data in this segment came from a transaction that was not in
                    // progress and changing their "in progress" state will upset our visibility
                    // expectations
                    pg_sys::FrozenTransactionId
                },
                xmax: pg_sys::InvalidTransactionId,
                postings: files.remove(&SegmentComponent::Postings).map(|e| e.0),
                positions: files.remove(&SegmentComponent::Positions).map(|e| e.0),
                fast_fields: files.remove(&SegmentComponent::FastFields).map(|e| e.0),
                field_norms: files.remove(&SegmentComponent::FieldNorms).map(|e| e.0),
                terms: files.remove(&SegmentComponent::Terms).map(|e| e.0),
                store: files.remove(&SegmentComponent::Store).map(|e| e.0),
                temp_store: files.remove(&SegmentComponent::TempStore).map(|e| e.0),
                delete: files
                    .remove(&SegmentComponent::Delete)
                    .map(|(file_entry, _)| DeleteEntry {
                        file_entry,
                        num_deleted_docs: created_segment.num_deleted_docs(),
                    }),
            };

            Some(meta_entry)
        })
        .collect::<Vec<_>>();

    //
    // process the modified segments
    //
    // lookup existing version, update the `delete` entry
    // if it already has one also locate its LinkedBytesList and .mark_deleted()
    // xmin/xmax do not change
    //
    let mut orphaned_deletes_files = Vec::new();
    let modified_entries = modified_ids
        .into_iter()
        .filter_map(|id| {
            let mut files = new_files.remove(id)?;
            assert!(
                files.len() == 1 && files.contains_key(&SegmentComponent::Delete),
                "new files for segment_id `{id}` should be exactly one Delete component:  {files:#?}"
            );

            let existing_segment = incoming_segments.get(id).unwrap();
            let (mut meta_entry, blockno, _) = linked_list
                .lookup_ex(|entry| entry.segment_id == *id)
                .unwrap_or_else(|e| {
                    panic!("segment id `{id}` should be in the segment meta linked list:  {e}")
                });
            let (new_delete_entry, _path) = files
                .remove(&SegmentComponent::Delete)
                .unwrap_or_else(|| panic!("missing new delete file for segment_id `{id}`"));

            if meta_entry.delete.is_some() {
                // remember the old delete_entry for future action
                orphaned_deletes_files.push((meta_entry.xmax, meta_entry.delete.unwrap()));
            }

            // replace (or set new) the delete_entry
            meta_entry.delete = Some(DeleteEntry {
                file_entry: new_delete_entry,
                num_deleted_docs: existing_segment.num_deleted_docs(),
            });

            Some((meta_entry, blockno))
        })
        .collect::<Vec<_>>();

    //
    // process the deleted segments
    //
    // find the deleted segment entries and set their `xmax` to the `deleting_xid` calculated above
    // and for each file in each deleted segment, locate its LinkedBytesList and .mark_deleted()
    //
    let deleted_entries = deleted_ids
        .into_iter()
        .map(|id| {
            let (mut meta_entry, blockno, _) = linked_list
                .lookup_ex(|entry| entry.segment_id == *id)
                .unwrap_or_else(|e| {
                    panic!("segment id `{id}` should be in the segment meta linked list: {e}")
                });

            // we need to be in a transaction in order to delete segments
            // this means (auto)VACUUM can't do this, but that's okay because it doesn't
            // it only applies .delete files, which we consider as modifications
            assert!(pg_sys::IsTransactionState());

            assert!(
                meta_entry.xmax == pg_sys::InvalidTransactionId,
                "SegmentMetaEntry {} should not already be deleted",
                meta_entry.segment_id
            );

            // deleted segments belong to a transaction that is known to not be in progress
            // and so when we mark them as deleted, it'll be with a transaction that is known to
            // not be in progress, the FrozenTransactionId.
            //
            // NB:  I think we could instead set `meta_entry.xmax = meta_entry.xmin` because the xmin
            // is also know to not be in progress anymore, but using FrozenTransactionId is nice as
            // it makes it clear when inspecting the segment meta entries list how this segment
            // got marked deleted
            meta_entry.xmax = pg_sys::FrozenTransactionId;

            (meta_entry, blockno)
        })
        .collect::<Vec<_>>();

    //
    // recycle anything leftover in `new_files` to our input `directory_entries` as they belong to segment(s) we
    // are not dealing with in this call to save_new_metas()
    //

    if !new_files.is_empty() {
        directory_entries.extend(
            new_files
                .into_values()
                .flat_map(|segment| segment.into_values())
                .map(|(entry, path)| (path, entry)),
        )
    }

    //
    // now change things on disk
    //

    // delete old entries and their corresponding files -- happens only as the result of a merge
    for (entry, blockno) in &deleted_entries {
        assert!(entry.xmax == pg_sys::FrozenTransactionId);
        let mut buffer = linked_list.bman_mut().get_buffer_mut(*blockno);
        let mut page = buffer.page_mut();

        let Some(offno) =
            page.find_item::<SegmentMetaEntry, _>(|item| item.segment_id == entry.segment_id)
        else {
            panic!(
                "DELETE:  could not find SegmentMetaEntry for segment_id `{}` on block #{blockno}",
                entry.segment_id
            );
        };

        let PgItem(pg_item, size) = (*entry).into();

        let did_replace = page.replace_item(offno, pg_item, size);
        assert!(did_replace);

        drop(buffer);
    }

    // replace the modified entries -- only happens because of vacuum
    for (entry, blockno) in modified_entries {
        let mut buffer = linked_list.bman_mut().get_buffer_mut(blockno);
        let mut page = buffer.page_mut();
        let Some(offno) =
            page.find_item::<SegmentMetaEntry, _>(|item| item.segment_id == entry.segment_id)
        else {
            panic!(
                "MODIFY:  could not find SegmentMetaEntry for segment_id `{}` on block #{blockno}",
                entry.segment_id
            );
        };

        let PgItem(pg_item, size) = entry.into();
        let did_replace = page.replace_item(offno, pg_item, size);
        if !did_replace {
            // couldn't replace because it doesn't fit in that slot, so delete the item...
            page.delete_item(offno);

            // ... and add it to somewhere in the list, starting on this page
            linked_list.add_items(&[entry], Some(buffer))?;
        }
    }

    // add the new entries -- happens via an index commit or the result of a merge
    if !created_entries.is_empty() {
        linked_list.add_items(&created_entries, None)?;
    }

    if !orphaned_deletes_files.is_empty() {
        // if we have orphaned ".deletes" files, what we do with them is add a new, fake entry to
        // the metas `linked_ist` for each one, where the fake entry is configured such that it's
        // immediately considered recyclable.  This will allow for a future garbage collection to
        // properly delete the blocks associated with the orphaned file
        let fake_entries = orphaned_deletes_files
            .into_iter()
            .map(|(_, delete_entry)| SegmentMetaEntry {
                segment_id: SegmentId::from_bytes([0; 16]), // all zeros
                max_doc: delete_entry.num_deleted_docs,
                xmin: pg_sys::FrozenTransactionId,
                xmax: pg_sys::FrozenTransactionId, // immediately recyclable
                postings: None,
                positions: None,
                fast_fields: None,
                field_norms: None,
                terms: None,
                store: None,
                temp_store: None,
                delete: Some(delete_entry), // the file whose bytes we need to ensure get garbage collected in the future
            })
            .collect::<Vec<_>>();
        linked_list.add_items(&fake_entries, None)?;
    }

    // atomically replace the SegmentMetaEntry list, and then mark any orphaned files deleted.
    linked_list.commit();

    Ok(())
}

impl LinkedItemList<SegmentMetaEntry> {
    pub unsafe fn for_each_and_pin<
        Accept: Fn(&SegmentMetaEntry, &mut BufferManager) -> bool,
        ForEach: FnMut(SegmentMetaEntry),
    >(
        &mut self,
        accept: Accept,
        mut for_each: ForEach,
    ) -> PinCushion {
        let mut pin_cushion = PinCushion::default();

        let mut blockno = self.get_start_blockno();
        while blockno != pg_sys::InvalidBlockNumber {
            let buffer = self.bman().get_buffer(blockno);
            let page = buffer.page();
            let mut offsetno = pg_sys::FirstOffsetNumber;
            let max_offset = page.max_offset_number();
            while offsetno <= max_offset {
                if let Some((deserialized, _)) = page.deserialize_item::<SegmentMetaEntry>(offsetno)
                {
                    if accept(&deserialized, self.bman_mut()) {
                        pin_cushion.push(self.bman(), &deserialized);
                        for_each(deserialized);
                    }
                }
                offsetno += 1;
            }
            blockno = page.next_blockno();
        }

        pin_cushion
    }
}

#[allow(clippy::collapsible_if)] // come on clippy, let me write the code that's clear to me
pub unsafe fn load_metas(
    relation_oid: pg_sys::Oid,
    inventory: &SegmentMetaInventory,
    snapshot: Option<pg_sys::Snapshot>,
    solve_mvcc: &MvccSatisfies,
) -> tantivy::Result<(Vec<SegmentMetaEntry>, IndexMeta, PinCushion)> {
    let mut segment_metas =
        LinkedItemList::<SegmentMetaEntry>::open(relation_oid, SEGMENT_METAS_START);
    let mut alive_segments = vec![];
    let mut alive_entries = vec![];
    let mut opstamp = None;

    let pin_cushion = segment_metas.for_each_and_pin(|entry, bman| {
        // nobody sees recyclable segments
        !entry.recyclable(bman) && (
            // parallel workers only see a specific set of segments.  This relies on the leader having kept a pin on them
            matches!(solve_mvcc, MvccSatisfies::ParallelWorker(only_these) if only_these.contains(&entry.segment_id))

            // vacuum sees everything that hasn't been deleted by a merge
            || (matches!(solve_mvcc, MvccSatisfies::Vacuum) && entry.xmax == pg_sys::InvalidTransactionId)

            // a snapshot can see any that are visible in its snapshot
            || (matches!(solve_mvcc, MvccSatisfies::Snapshot) && entry.visible(snapshot.expect("snapshot must be provided for `MvccSatisfies::Snapshot`")))

            // mergeable can see any that are known to be mergeable
            || (matches!(solve_mvcc, MvccSatisfies::Mergeable) && entry.mergeable())
        )
    }, |entry| {
        let inner_segment_meta = InnerSegmentMeta {
            max_doc: entry.max_doc,
            segment_id: entry.segment_id,
            deletes: entry.delete.map(|delete_entry| DeleteMeta {
                num_deleted_docs: delete_entry.num_deleted_docs,
                opstamp: 0, // hardcode zero as the entry's opstamp as it's not used
            }),
            include_temp_doc_store: Arc::new(AtomicBool::new(false)),
        };
        alive_segments.push(inner_segment_meta.track(inventory));
        alive_entries.push(entry);

        opstamp = opstamp.max(Some(entry.opstamp()));
    });

    if let MvccSatisfies::ParallelWorker(only_these) = solve_mvcc {
        assert!(alive_entries.len() == only_these.len(), "load_metas: MvccSatisfies::ParallelWorker didn't load the correct segments.  desired={only_these:?}, actual={alive_entries:?}");
    }

    let schema = LinkedBytesList::open(relation_oid, SCHEMA_START);
    let settings = LinkedBytesList::open(relation_oid, SETTINGS_START);
    let deserialized_schema = serde_json::from_slice(&schema.read_all())?;
    let deserialized_settings = serde_json::from_slice(&settings.read_all())?;

    Ok((
        alive_entries,
        IndexMeta {
            segments: alive_segments,
            schema: deserialized_schema,
            index_settings: deserialized_settings,
            opstamp: opstamp.unwrap_or(0),
            payload: None,
        },
        pin_cushion,
    ))
}
