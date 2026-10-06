// Copyright (c) 2023-2026 ParadeDB, Inc.
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

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::io;

use rand::{RngExt, SeedableRng, rngs::StdRng};
use serde::{Deserialize, Serialize};
use tantivy::aggregation::metric::CardinalityCollector;
use tantivy::columnar::{Column, DynamicColumn, MonotonicallyMappableToU64};
#[cfg(test)]
use tantivy::directory::CompositeFile;
use tantivy::directory::CompositeWrite;
use tantivy::schema::Field;

const VERSION: u8 = 2;
const MANIFEST_IDX: usize = 2;
const SAMPLE_DOCS: usize = 2048;
const MAX_SAMPLE_VALUES: usize = 16384;
const HISTOGRAM_SIZE: usize = 128;
const COMMON_VALUES: usize = 32;
const MAX_DICTIONARY_BYTES: usize = 1 << 20;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct DistributionManifest {
    version: u8,
    pub(crate) columns: Vec<(String, u8)>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ValueFrequency {
    pub(crate) value: u128,
    pub(crate) documents: u32,
}

/// Values use the column's order-preserving encoding; strings use segment-local ordinals.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Distribution {
    pub(crate) dictionary: Vec<(u128, Vec<u8>)>,
    pub(crate) present_docs: u32,
    pub(crate) distinct: f64,
    pub(crate) common: Vec<ValueFrequency>,
    pub(crate) histogram: Vec<u128>,
    pub(crate) sample: Vec<Vec<u128>>,
}

impl DistributionManifest {
    pub(crate) fn write<W: tantivy_common::TerminatingWrite>(
        columns: impl Iterator<Item = io::Result<(String, DynamicColumn)>>,
        write: &mut CompositeWrite<W>,
    ) -> io::Result<()> {
        use std::io::Write;
        let mut manifest = Self {
            version: VERSION,
            columns: Vec::new(),
        };
        for column in columns {
            let (name, column) = column?;
            let idx = MANIFEST_IDX + 1 + manifest.columns.len();
            manifest.columns.push((name, column.column_type() as u8));
            if let Some(stats) = Distribution::collect(&column) {
                write
                    .for_field_with_idx(Field::from_field_id(0), idx)
                    .write_all(&postcard::to_allocvec(&stats).map_err(io::Error::other)?)?;
            }
        }
        write
            .for_field_with_idx(Field::from_field_id(0), MANIFEST_IDX)
            .write_all(&postcard::to_allocvec(&manifest).map_err(io::Error::other)?)?;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn read(file: &CompositeFile) -> io::Result<Option<Self>> {
        let Some(slice) = file.open_read_with_idx(Field::from_field_id(0), MANIFEST_IDX) else {
            return Ok(None);
        };
        let bytes = slice.read_bytes()?;
        // Check the version before decoding the rest so future formats fail open.
        if bytes.first() != Some(&VERSION) {
            return Ok(None);
        }
        postcard::from_bytes(&bytes)
            .map(Some)
            .map_err(io::Error::other)
    }

    #[cfg(test)]
    pub(crate) fn distribution(
        &self,
        file: &CompositeFile,
        ordinal: usize,
    ) -> io::Result<Option<Distribution>> {
        if ordinal >= self.columns.len() {
            return Ok(None);
        }
        let Some(slice) =
            file.open_read_with_idx(Field::from_field_id(0), MANIFEST_IDX + 1 + ordinal)
        else {
            return Ok(None);
        };
        let bytes = slice.read_bytes()?;
        postcard::from_bytes(&bytes)
            .map(Some)
            .map_err(io::Error::other)
    }
}

impl Distribution {
    fn collect(column: &DynamicColumn) -> Option<Self> {
        match column {
            DynamicColumn::Bool(c) => Self::from_column(c, |v| u128::from(v.to_u64()), None),
            DynamicColumn::I64(c) => Self::from_column(c, |v| u128::from(v.to_u64()), None),
            DynamicColumn::U64(c) => Self::from_column(c, u128::from, None),
            DynamicColumn::F64(c) => Self::from_column(c, |v| u128::from(v.to_u64()), None),
            DynamicColumn::DateTime(c) => Self::from_column(c, |v| u128::from(v.to_u64()), None),
            DynamicColumn::IpAddr(c) => Self::from_column(c, u128::from, None),
            DynamicColumn::Str(c) => {
                Self::from_column(c.ords(), u128::from, Some(c.num_terms() as f64))?
                    .with_dictionary(|ord, bytes| c.ord_to_bytes(ord, bytes))
            }
            DynamicColumn::Bytes(c) => {
                Self::from_column(c.ords(), u128::from, Some(c.num_terms() as f64))?
                    .with_dictionary(|ord, bytes| c.ord_to_bytes(ord, bytes))
            }
        }
    }

    fn with_dictionary(
        mut self,
        resolve: impl Fn(u64, &mut Vec<u8>) -> io::Result<bool>,
    ) -> Option<Self> {
        let mut ords: Vec<_> = self.sample.iter().flatten().copied().collect();
        ords.sort_unstable();
        ords.dedup();
        let mut total_bytes = 0;
        for ord in ords {
            let mut bytes = Vec::new();
            if !resolve(ord as u64, &mut bytes).ok()? {
                return None;
            }
            total_bytes += bytes.len();
            if total_bytes > MAX_DICTIONARY_BYTES {
                return None;
            }
            self.dictionary.push((ord, bytes));
        }
        Some(self)
    }

    /// Scan only during segment serialization. Keep a uniform document sample and an HLL count.
    fn from_column<T: PartialOrd + Copy + Debug + Send + Sync + 'static>(
        column: &Column<T>,
        encode: impl Fn(T) -> u128,
        known_distinct: Option<f64>,
    ) -> Option<Self> {
        let mut rng = StdRng::seed_from_u64(0x57A75);
        let mut docs: Vec<u32> = (0..column.num_docs().min(SAMPLE_DOCS as u32)).collect();
        for doc in SAMPLE_DOCS as u32..column.num_docs() {
            let slot = rng.random_range(0..=doc) as usize;
            if slot < SAMPLE_DOCS {
                docs[slot] = doc;
            }
        }
        docs.sort_unstable();
        let mut distinct = CardinalityCollector::default();
        let mut present_docs = 0;
        let mut sample = Vec::with_capacity(docs.len());
        let mut sampled_values = 0;
        let mut selected = docs.into_iter().peekable();
        for doc in 0..column.num_docs() {
            let keep = selected.peek() == Some(&doc);
            if !keep && known_distinct.is_some() {
                present_docs += u32::from(column.first(doc).is_some());
                continue;
            }
            let mut row = Vec::new();
            let mut present = false;
            for value in column.values_for_doc(doc) {
                present = true;
                let value = encode(value);
                if known_distinct.is_none() {
                    distinct.insert_bytes(&value.to_le_bytes());
                }
                if keep {
                    sampled_values += 1;
                    if sampled_values > MAX_SAMPLE_VALUES {
                        return None;
                    }
                    row.push(value);
                }
            }
            present_docs += u32::from(present);
            if keep {
                row.sort_unstable();
                row.dedup();
                sample.push(row);
                selected.next();
            }
        }
        let mut counts = BTreeMap::<u128, u32>::new();
        for value in sample.iter().flatten() {
            *counts.entry(*value).or_default() += 1;
        }
        let mut common: Vec<_> = counts
            .iter()
            .filter(|(_, count)| **count > 1)
            .map(|(&value, &documents)| ValueFrequency { value, documents })
            .collect();
        common.sort_unstable_by_key(|entry| (std::cmp::Reverse(entry.documents), entry.value));
        common.truncate(COMMON_VALUES);
        let remainder: Vec<_> = sample
            .iter()
            .flatten()
            .copied()
            .filter(|value| !common.iter().any(|entry| entry.value == *value))
            .collect();
        let mut remainder = remainder;
        remainder.sort_unstable();
        let histogram = if remainder.is_empty() {
            Vec::new()
        } else {
            let size = HISTOGRAM_SIZE.min(remainder.len());
            (0..size)
                .map(|i| remainder[i * (remainder.len() - 1) / (size - 1).max(1)])
                .collect()
        };
        Some(Self {
            dictionary: Vec::new(),
            present_docs,
            distinct: known_distinct.unwrap_or_else(|| distinct.finalize().unwrap_or_default()),
            common,
            histogram,
            sample,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tantivy::columnar::{ColumnarReader, ColumnarWriter, DEFAULT_CODEC_TYPES};

    fn column(rows: &[Vec<u64>]) -> DynamicColumn {
        let mut writer = ColumnarWriter::default();
        writer.record_column_type("value", tantivy::columnar::ColumnType::U64, false);
        for (doc, values) in rows.iter().enumerate() {
            for &value in values {
                writer.record_numerical(doc as u32, "value", value);
            }
        }
        let mut bytes = Vec::new();
        writer
            .serialize(rows.len() as u32, None, &DEFAULT_CODEC_TYPES, &mut bytes)
            .unwrap();
        ColumnarReader::open(bytes)
            .unwrap()
            .read_columns("value")
            .unwrap()[0]
            .open()
            .unwrap()
    }

    #[test]
    fn distribution_round_trip() {
        let mut bytes = Vec::new();
        let mut write = CompositeWrite::wrap(&mut bytes);
        DistributionManifest::write(
            std::iter::once(Ok(("value".into(), column(&[vec![4], vec![]])))),
            &mut write,
        )
        .unwrap();
        write.close().unwrap();
        let stats =
            crate::index::stats::SegmentStats::open(tantivy::directory::FileSlice::from(bytes))
                .unwrap();
        let manifest = stats.distributions().unwrap().unwrap();
        assert_eq!(manifest.columns.len(), 1);
        assert_eq!(
            stats
                .distribution(&manifest, 0)
                .unwrap()
                .unwrap()
                .present_docs,
            1
        );
    }

    #[test]
    fn distribution_old_unknown_and_missing_entries() {
        use std::io::Write;
        for version in [None, Some(VERSION - 1), Some(VERSION + 1)] {
            let mut bytes = Vec::new();
            let mut write = CompositeWrite::wrap(&mut bytes);
            if let Some(version) = version {
                write
                    .for_field_with_idx(Field::from_field_id(0), MANIFEST_IDX)
                    .write_all(&[version])
                    .unwrap();
            }
            write.close().unwrap();
            let stats =
                crate::index::stats::SegmentStats::open(tantivy::directory::FileSlice::from(bytes))
                    .unwrap();
            assert!(stats.distributions().unwrap().is_none());
        }
        let mut bytes = Vec::new();
        let mut write = CompositeWrite::wrap(&mut bytes);
        DistributionManifest::write(
            std::iter::once(Ok((
                "huge".into(),
                column(&[vec![0; MAX_SAMPLE_VALUES + 1]]),
            ))),
            &mut write,
        )
        .unwrap();
        write.close().unwrap();
        let stats =
            crate::index::stats::SegmentStats::open(tantivy::directory::FileSlice::from(bytes))
                .unwrap();
        let manifest = stats.distributions().unwrap().unwrap();
        assert_eq!(manifest.columns[0].0, "huge");
        assert!(stats.distribution(&manifest, 0).unwrap().is_none());
    }

    #[test]
    fn distribution_merge_upgrades_old_segments_and_removes_deleted_values() {
        use tantivy::merge_policy::NoMergePolicy;
        use tantivy::schema::{FAST, INDEXED, Schema};
        use tantivy::{Index, IndexWriter, TantivyDocument, Term, doc};
        let mut schema = Schema::builder();
        let id = schema.add_u64_field("id", FAST | INDEXED);
        let schema = schema.build();
        let index = Index::builder()
            .schema(schema)
            .register_plugin(std::sync::Arc::new(super::super::plugin::StatsPlugin))
            .create_in_ram()
            .unwrap();
        {
            let mut writer: IndexWriter<TantivyDocument> = index.writer(15_000_000).unwrap();
            writer.set_merge_policy(Box::new(NoMergePolicy));
            writer.add_document(doc!(id => 1u64)).unwrap();
            writer.add_document(doc!(id => 999u64)).unwrap();
            writer.commit().unwrap();
        }
        let old_segment = index.searchable_segments().unwrap().remove(0);
        tantivy::directory::Directory::delete(
            index.directory(),
            &old_segment.relative_path(super::super::plugin::stats_component()),
        )
        .unwrap();
        CompositeWrite::wrap(
            old_segment
                .open_write(super::super::plugin::stats_component())
                .unwrap(),
        )
        .close()
        .unwrap();
        assert!(
            crate::index::stats::SegmentStats::of_segment(&old_segment)
                .unwrap()
                .unwrap()
                .distributions()
                .unwrap()
                .is_none()
        );
        {
            let mut writer: IndexWriter<TantivyDocument> = index.writer(15_000_000).unwrap();
            writer.set_merge_policy(Box::new(NoMergePolicy));
            writer.add_document(doc!(id => 2u64)).unwrap();
            writer.delete_term(Term::from_field_u64(id, 999));
            writer.commit().unwrap();
            writer
                .merge(&index.searchable_segment_ids().unwrap())
                .wait()
                .unwrap();
        }
        let segments = index.searchable_segments().unwrap();
        assert_eq!(segments.len(), 1);
        let stats = crate::index::stats::SegmentStats::of_segment(&segments[0])
            .unwrap()
            .unwrap();
        let manifest = stats.distributions().unwrap().unwrap();
        let summary = stats.distribution(&manifest, 0).unwrap().unwrap();
        assert_eq!(summary.present_docs, 2);
        assert_eq!(summary.distinct, 2.0);
        let mut values: Vec<_> = summary.sample.into_iter().flatten().collect();
        values.sort_unstable();
        assert_eq!(values, vec![1, 2]);
    }

    #[test]
    fn distribution_separates_json_paths_and_types() {
        let mut writer = ColumnarWriter::default();
        writer.record_numerical(0, "json\u{1}value", 42u64);
        writer.record_str(1, "json\u{1}value", "forty two");
        writer.record_str(2, "json\u{1}other", "separate");
        writer.record_bool(0, "flag", true);
        writer.record_ip_addr(0, "ip", "::1".parse().unwrap());
        writer.record_datetime(
            0,
            "date",
            tantivy_common::DateTime::from_timestamp_micros(123),
        );
        writer.record_numerical(0, "negative", -42i64);
        writer.record_numerical(0, "float", 1.5f64);
        writer.record_bytes(0, "bytes", b"raw");
        let mut fast = Vec::new();
        writer
            .serialize(3, None, &DEFAULT_CODEC_TYPES, &mut fast)
            .unwrap();
        let reader = ColumnarReader::open(fast).unwrap();
        let mut bytes = Vec::new();
        let mut output = CompositeWrite::wrap(&mut bytes);
        DistributionManifest::write(
            reader
                .iter_columns()
                .unwrap()
                .map(|(name, h)| h.open().map(|c| (name, c))),
            &mut output,
        )
        .unwrap();
        output.close().unwrap();
        let stats =
            crate::index::stats::SegmentStats::open(tantivy::directory::FileSlice::from(bytes))
                .unwrap();
        let manifest = stats.distributions().unwrap().unwrap();
        assert_eq!(
            manifest
                .columns
                .iter()
                .filter(|(name, _)| name == "json\u{1}value")
                .count(),
            2
        );
        for ordinal in 0..manifest.columns.len() {
            let summary = stats.distribution(&manifest, ordinal).unwrap().unwrap();
            assert_eq!(summary.present_docs, 1);
            assert_eq!(summary.distinct, 1.0);
            assert_eq!(summary.sample.len(), 3);
            if matches!(
                tantivy::columnar::ColumnType::try_from_code(manifest.columns[ordinal].1).unwrap(),
                tantivy::columnar::ColumnType::Str | tantivy::columnar::ColumnType::Bytes
            ) {
                assert_eq!(summary.dictionary.len(), 1);
                assert!(!summary.dictionary[0].1.is_empty());
            }
        }
    }

    #[test]
    fn distribution_string_dictionary_has_a_byte_budget() {
        let mut writer = ColumnarWriter::default();
        for doc in 0..32 {
            writer.record_str(doc, "text", &format!("{doc:02}{}", "x".repeat(40000)));
        }
        let mut bytes = Vec::new();
        writer
            .serialize(32, None, &DEFAULT_CODEC_TYPES, &mut bytes)
            .unwrap();
        let reader = ColumnarReader::open(bytes).unwrap();
        let column = reader.read_columns("text").unwrap()[0].open().unwrap();
        assert!(Distribution::collect(&column).is_none());
    }

    #[test]
    fn distribution_size_stays_bounded_as_data_grows() {
        for count in [10000, 200000] {
            let column = column(&(0..count).map(|i| vec![i]).collect::<Vec<_>>());
            let start = std::time::Instant::now();
            let summary = Distribution::collect(&column).unwrap();
            let bytes = postcard::to_allocvec(&summary).unwrap().len();
            eprintln!(
                "{count} values: {} ms, {bytes} summary bytes",
                start.elapsed().as_millis()
            );
            assert_eq!(summary.sample.len(), SAMPLE_DOCS);
            assert_eq!(summary.histogram.len(), HISTOGRAM_SIZE);
            assert!(bytes < 65536);
            assert!((summary.distinct / count as f64 - 1.0).abs() < 0.1);
        }
    }

    #[test]
    fn distribution_counts_documents_and_preserves_empty_rows() {
        let stats = Distribution::collect(&column(&[vec![1, 1, 2], vec![], vec![1]])).unwrap();
        assert_eq!(stats.present_docs, 2);
        assert_eq!(stats.distinct, 2.0);
        assert_eq!(stats.sample, vec![vec![1, 2], vec![], vec![1]]);
        assert_eq!(stats.common[0].value, 1);
        assert_eq!(stats.common[0].documents, 2);
        assert_eq!(stats.histogram, vec![2]);
    }

    #[test]
    fn distribution_tracks_skew_and_bounded_quantiles() {
        let rows: Vec<_> = (0..20000)
            .map(|i| vec![if i < 18000 { 7 } else { i }])
            .collect();
        let column = column(&rows);
        let stats = Distribution::collect(&column).unwrap();
        assert_eq!(stats.sample.len(), SAMPLE_DOCS);
        assert_eq!(stats.common[0].value, 7);
        let fraction = stats.common[0].documents as f64 / SAMPLE_DOCS as f64;
        assert!((fraction - 0.9).abs() < 0.03, "{fraction}");
        assert!((stats.distinct - 2001.0).abs() < 200.0);
        assert!(stats.histogram.len() <= HISTOGRAM_SIZE);
        assert!(stats.histogram.windows(2).all(|pair| pair[0] <= pair[1]));
        assert_eq!(
            postcard::to_allocvec(&stats).unwrap(),
            postcard::to_allocvec(&Distribution::collect(&column).unwrap()).unwrap()
        );
    }

    #[test]
    fn distribution_rejects_truncated_document_samples() {
        let rows = vec![vec![0; MAX_SAMPLE_VALUES + 1]];
        assert!(Distribution::collect(&column(&rows)).is_none());
    }
}
