use crate::api::{HashMap, HashSet};
use crate::index::mvcc::{MvccSatisfies, PinCushion};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::{
    DeleteEntry, FileEntry, LinkedList, MVCCEntry, PgItem, SegmentFileDetails, SegmentMetaEntry,
    SCHEMA_START, SEGMENT_METAS_START, SETTINGS_START,
};
use crate::postgres::storage::metadata::MetaPage;
use crate::postgres::storage::{LinkedBytesList, LinkedItemList};
use anyhow::Result;
use pgrx::pg_sys;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tantivy::index::SegmentComponent;
use tantivy::{
    index::{DeleteMeta, IndexSettings, InnerSegmentMeta, SegmentId, SegmentMetaInventory},
    schema::Schema,
    IndexMeta,
};

pub fn save_schema(indexrel: &PgSearchRelation, tantivy_schema: &Schema) -> Result<()> {
    let schema = LinkedBytesList::open(indexrel, SCHEMA_START);
    if schema.is_empty() {
        let bytes = serde_json::to_vec(tantivy_schema)?;
        unsafe {
            schema.writer().write(&bytes)?;
        }
    }
    Ok(())
}

pub fn save_settings(indexrel: &PgSearchRelation, tantivy_settings: &IndexSettings) -> Result<()> {
    let settings = LinkedBytesList::open(indexrel, SETTINGS_START);
    if settings.is_empty() {
        let bytes = serde_json::to_vec(tantivy_settings)?;
        unsafe {
            settings.writer().write(&bytes)?;
        }
    }
    Ok(())
}

pub unsafe fn save_new_metas(
    indexrel: &PgSearchRelation,
    new_meta: &IndexMeta,
    prev_meta: &IndexMeta,
    directory_entries: &mut HashMap<PathBuf, FileEntry>,
) -> Result<()> {
    // in order to ensure that all of our mutations to the list of segments appear atomically on
    // physical replicas, we atomically operate on a deep copy of the list.
    let mut segment_metas_linked_list =
        LinkedItemList::<SegmentMetaEntry>::open(indexrel, SEGMENT_METAS_START);
    let mut linked_list = segment_metas_linked_list.atomically();

    let incoming_segments = new_meta
        .segments
        .iter()
        .map(|s| (s.id(), s))
        .collect::<HashMap<_, _>>();
    let new_ids = new_meta
        .segments
        .iter()
        .map(|s| s.id())
        .collect::<HashSet<_>>();
    let previous_ids = prev_meta
        .segments
        .iter()
        .map(|s| s.id())
        .collect::<HashSet<_>>();

    // first, reorganize the directory_entries by segment id
    let mut new_files =
        HashMap::<SegmentId, HashMap<SegmentComponent, (FileEntry, PathBuf)>>::default();

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
                _unused: pg_sys::InvalidTransactionId,
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
            linked_list.add_items(&[entry], Some(buffer));
        }
    }

    // add the new entries -- happens via an index commit or the result of a merge
    if !created_entries.is_empty() {
        linked_list.add_items(&created_entries, None);
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
                xmax: pg_sys::FrozenTransactionId, // immediately recyclable
                delete: Some(delete_entry), // the file whose bytes we need to ensure get garbage collected in the future
                ..Default::default()
            })
            .collect::<Vec<_>>();
        linked_list.add_items(&fake_entries, None);
    }

    // atomically replace the SegmentMetaEntry list, and then mark any orphaned files deleted.
    linked_list.commit();

    Ok(())
}

pub struct LoadedMetas {
    pub entries: Vec<SegmentMetaEntry>,
    pub meta: IndexMeta,
    pub pin_cushion: PinCushion,
    pub total_segments: usize,
}

pub unsafe fn load_metas(
    indexrel: &PgSearchRelation,
    inventory: &SegmentMetaInventory,
    solve_mvcc: &MvccSatisfies,
) -> tantivy::Result<LoadedMetas> {
    let mut total_segments = 0;
    let mut alive_segments = vec![];
    let mut alive_entries = vec![];
    let mut opstamp = None;
    let mut pin_cushion = PinCushion::default();

    // Collect segments from each relevant list.
    let mut segment_metas = LinkedItemList::<SegmentMetaEntry>::open(indexrel, SEGMENT_METAS_START);
    let mut exhausted_metas_lists = false;

    let is_largest_only = &MvccSatisfies::LargestSegment == solve_mvcc;
    let mut largest_doc_count = 0;
    loop {
        // Find all relevant segments in this list.
        segment_metas.for_each(|bman, entry| {
            // nobody sees recyclable segments
            let accept = !entry.recyclable(bman) && (
                // parallel workers only see a specific set of segments.  This relies on the leader having kept a pin on them
                matches!(solve_mvcc, MvccSatisfies::ParallelWorker(only_these) if only_these.contains(&entry.segment_id))

                    // vacuum sees everything that hasn't been deleted by a merge
                    || (matches!(solve_mvcc, MvccSatisfies::Vacuum) && entry.xmax == pg_sys::InvalidTransactionId)

                    // a snapshot or ::LargestSegment can see any that are visible in its snapshot
                    || (matches!(solve_mvcc, MvccSatisfies::Snapshot | MvccSatisfies::LargestSegment) && entry.visible())

                    // mergeable can see any that are known to be mergeable
                    || (matches!(solve_mvcc, MvccSatisfies::Mergeable) && entry.mergeable())
            );
            if !accept {
                return;
            };

            total_segments += 1;

            let mut need_entry = true;
            if is_largest_only {
                if entry.num_docs() > largest_doc_count {
                    largest_doc_count = entry.num_docs();

                    // the entry we're processing right now is known to be the largest so far
                    // and it's the only one we want
                    alive_segments.clear();
                    alive_entries.clear();
                    pin_cushion.clear();
                } else {
                    // we already have the largest so we don't need this entry
                    need_entry = false;
                }
            }

            if need_entry {
                pin_cushion.push(bman, &entry);
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
            }
        });

        match solve_mvcc {
            MvccSatisfies::ParallelWorker(only_these)
                if alive_entries.len() != only_these.len() =>
            {
                // If we haven't tried the `segment_metas_garbage` list, try that next.
                if !exhausted_metas_lists {
                    if let Some(garbage) = MetaPage::open(indexrel).segment_metas_garbage() {
                        segment_metas = garbage;
                        exhausted_metas_lists = true;
                        continue;
                    }
                }

                let missing = only_these
                    .difference(&alive_entries.iter().map(|s| s.segment_id).collect())
                    .cloned()
                    .collect::<HashSet<SegmentId>>();
                let found = only_these.difference(&missing).collect::<HashSet<_>>();

                panic!(
                    "load_metas: MvccSatisfies::ParallelWorker didn't load the correct segments. \
                    found={found:?}, missing={missing:?}",
                );
            }
            #[cfg(debug_assertions)]
            MvccSatisfies::ParallelWorker(only_these) => {
                // In debug mode only, actually do a set comparison to determine that we got the
                // exact expected segments.
                let actual = alive_entries
                    .iter()
                    .map(|s| s.segment_id)
                    .collect::<HashSet<_>>();
                assert_eq!(
                    &actual, only_these,
                    "Got the wrong segments in parallel worker: \
                     actual: {actual:?}, expected: {only_these:?}"
                );
                break;
            }
            _ => {
                // We've successfully collected all of the relevant entries.
                break;
            }
        }
    }

    let schema = LinkedBytesList::open(indexrel, SCHEMA_START);
    let settings = LinkedBytesList::open(indexrel, SETTINGS_START);
    let deserialized_schema = serde_json::from_slice(&schema.read_all())?;
    let deserialized_settings = serde_json::from_slice(&settings.read_all())?;

    Ok(LoadedMetas {
        entries: alive_entries,
        meta: IndexMeta {
            segments: alive_segments,
            schema: deserialized_schema,
            index_settings: deserialized_settings,
            opstamp: opstamp.unwrap_or(0),
            payload: None,
        },
        pin_cushion,
        total_segments,
    })
}

pub fn load_index_schema(indexrel: &PgSearchRelation) -> tantivy::Result<Option<Schema>> {
    let schema_bytes = unsafe { LinkedBytesList::open(indexrel, SCHEMA_START).read_all() };
    if schema_bytes.is_empty() {
        return Ok(None);
    }
    Ok(serde_json::from_slice(&schema_bytes)?)
}
