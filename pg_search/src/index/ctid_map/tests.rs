// Copyright (c) 2023-2026 ParadeDB, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

#[pgrx::pg_schema]
mod tests {
    use std::slice;

    use pgrx::pg_test;
    use tantivy::directory::{RamDirectory, TerminatingWrite};
    use tantivy::index::{IndexSortByField, Order};
    use tantivy::schema::{FAST, Schema};
    use tantivy::{Index, IndexSettings};

    use super::super::*;

    #[pg_test]
    fn nullable_ctid_map_boundaries() {
        let chunk = CHUNK_SIZE as BlockNumber;
        for blocks in [
            vec![10, 10, 10],
            vec![10, 10, 11, 11, 12],
            vec![10, 10, 11, 11, 11, 15, 15],
            vec![10, 10 + chunk - 1, 10 + chunk, 10 + chunk * 3 + 2],
            vec![10, 10 + chunk - 2],
            (10..10 + chunk + 4)
                .filter(|block| block % 101 != 0)
                .collect(),
            vec![InvalidBlockNumber - 2, InvalidBlockNumber - 1],
        ] {
            for descending in [false, true] {
                for legacy in [false, true] {
                    let mut schema = Schema::builder();
                    let field = schema.add_u64_field(CTID_FIELD_NAME, FAST);
                    let index = Index::create(
                        RamDirectory::create(),
                        schema.build(),
                        IndexSettings {
                            sort_by_field: Some(IndexSortByField {
                                field: CTID_FIELD_NAME.into(),
                                order: if descending { Order::Desc } else { Order::Asc },
                            }),
                            ..IndexSettings::default()
                        },
                    )
                    .unwrap();
                    let segment = index.new_segment();
                    let mut writer = ColumnarWriter::default();
                    writer.record_column_type(CTID_FIELD_NAME, ColumnType::U64, false);
                    for doc in 0..blocks.len() {
                        let block = blocks[if descending {
                            blocks.len() - doc - 1
                        } else {
                            doc
                        }];
                        writer.record_numerical(
                            doc as DocId,
                            CTID_FIELD_NAME,
                            (u64::from(block) << 16) | 1,
                        );
                    }
                    let mut fast = segment.open_write(SegmentComponent::FastFields).unwrap();
                    writer
                        .serialize(blocks.len() as DocId, None, CODECS, &mut fast)
                        .unwrap();
                    fast.terminate().unwrap();
                    let first_block = blocks[0];
                    let last_block = *blocks.last().unwrap();
                    let count = (last_block - first_block) as usize + 2;
                    let mut output =
                        CompositeWrite::wrap(segment.open_write(plugin::component()).unwrap());
                    if legacy {
                        for start in (0..count).step_by(CHUNK_SIZE) {
                            let len = (count - start).min(CHUNK_SIZE);
                            let mut writer = ColumnarWriter::default();
                            writer.record_column_type(BLOCK_BOUNDARIES, ColumnType::U64, false);
                            for row in 0..len {
                                let block = first_block + (start + row) as BlockNumber;
                                let boundary = blocks.partition_point(|&present| present < block);
                                writer.record_numerical(
                                    row as DocId,
                                    BLOCK_BOUNDARIES,
                                    boundary as u64,
                                );
                            }
                            writer
                                .serialize(
                                    len as DocId,
                                    None,
                                    CODECS,
                                    output.for_field_with_idx(field, start / CHUNK_SIZE),
                                )
                                .unwrap();
                        }
                    } else {
                        write(&segment, &mut output).unwrap();
                    }
                    output.close().unwrap();
                    let file =
                        CompositeFile::open(&segment.open_read(plugin::component()).unwrap())
                            .unwrap();
                    let mut map = BlockToDocIdMap {
                        first_block,
                        last_block,
                        num_docs: blocks.len() as DocId,
                        file,
                        field,
                        values: None,
                    };
                    let requested: Vec<_> = (first_block..=last_block + 1).collect();
                    let expected: Vec<_> = requested
                        .iter()
                        .map(|block| blocks.partition_point(|present| present < block) as DocId)
                        .collect();
                    assert_eq!(map.boundaries(&requested).unwrap(), expected);
                    // Exercise cache reuse and queries that start in a different chunk.
                    for &block in requested.iter().rev() {
                        let start = blocks.partition_point(|&present| present < block) as DocId;
                        assert_eq!(map.boundaries(&[block]).unwrap(), [start]);
                    }
                    assert_eq!(
                        map.doc_id_ranges_for_blocks(slice::from_ref(
                            &(first_block..last_block + 1)
                        ))
                        .unwrap()
                        .as_slice(),
                        slice::from_ref(&(0..blocks.len() as DocId))
                    );
                    if !legacy {
                        for chunk in 0..count.div_ceil(CHUNK_SIZE) {
                            let reader = ColumnarReader::open(
                                map.file.open_read_with_idx(field, chunk).unwrap(),
                            )
                            .unwrap();
                            let column = reader.read_columns(BLOCK_BOUNDARIES).unwrap()[0]
                                .open()
                                .unwrap();
                            let DynamicColumn::U64(column) = column else {
                                panic!("expected u64 boundaries")
                            };
                            for row in 0..column.num_docs() - 1 {
                                let block = first_block + (chunk * CHUNK_SIZE) as BlockNumber + row;
                                assert_eq!(
                                    column.index.has_value(row),
                                    blocks.binary_search(&block).is_ok()
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
