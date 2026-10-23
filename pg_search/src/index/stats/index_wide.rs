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

//! a VACUUM-time summary of one complete segment view. the segment stamp lets a planner
//! decline this summary after a write or background merge changes the view.

use std::cmp::Ordering;
use std::io;

use serde::{Deserialize, Serialize};
use tantivy::index::SegmentReader;

use super::{EmpiricalStats, EmpiricalWire, SegmentStats};
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::metadata::MetaPage;

const MAGIC: &[u8; 8] = b"PDBISTAT";
const FORMAT: u8 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SegmentStamp {
    id: String,
    max_doc: u32,
    num_docs: u64,
}

impl SegmentStamp {
    fn of(index: &SearchIndexReader, reader: &SegmentReader) -> Self {
        Self {
            id: reader.segment_id().to_string(),
            max_doc: reader.max_doc(),
            num_docs: index.segment_num_docs(reader),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct FieldSummary {
    pub(crate) name: String,
    /// a type or field-options change invalidates the old summary even if the name is reused.
    pub(crate) field_type: String,
    /// unknown if any nonempty segment cannot provide bounds for this field.
    bounds: Option<EmpiricalWire>,
}

impl FieldSummary {
    // the estimator starts consuming this in the follow-up PR.
    #[allow(dead_code)]
    pub(crate) fn bounds(&self) -> Option<EmpiricalStats> {
        self.bounds.as_ref().map(|wire| EmpiricalStats {
            min: wire.min.0.clone(),
            max: wire.max.0.clone(),
            nullable: wire.nullable,
        })
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct IndexStats {
    pub(crate) total_docs: u64,
    segments: Vec<SegmentStamp>,
    pub(crate) fields: Vec<FieldSummary>,
}

impl IndexStats {
    fn encode(&self) -> io::Result<Vec<u8>> {
        let mut bytes = Vec::from(MAGIC.as_slice());
        bytes.push(FORMAT);
        bytes.extend(
            postcard::to_allocvec(self)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?,
        );
        Ok(bytes)
    }

    fn decode(bytes: &[u8]) -> io::Result<Self> {
        let Some(payload) = bytes.strip_prefix(MAGIC) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "index statistics marker is absent",
            ));
        };
        let Some(payload) = payload.strip_prefix(&[FORMAT]) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unsupported index statistics format",
            ));
        };
        postcard::from_bytes(payload)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))
    }

    /// the file belongs to a complete view, not a particular segment that happened to be large.
    #[allow(dead_code)]
    pub(crate) fn covers(&self, reader: &SearchIndexReader) -> bool {
        let mut current: Vec<_> = reader
            .segment_readers()
            .iter()
            .map(|segment| SegmentStamp::of(reader, segment))
            .collect();
        current.sort_by(|a, b| a.id.cmp(&b.id));
        self.segments == current
            && self.total_docs == current.iter().map(|segment| segment.num_docs).sum::<u64>()
            && self.fields.len() == reader.schema().fields().count()
            && self.fields.iter().all(|summary| {
                reader
                    .schema()
                    .search_field(&summary.name)
                    .is_some_and(|field| {
                        format!("{:?}", field.field_entry().field_type()) == summary.field_type
                    })
            })
    }
}

/// build from the maintenance view after VACUUM has removed dead segment entries. a missing
/// per-segment component only loses bounds for that field; it does not lose the document count.
pub(crate) fn refresh(indexrel: &PgSearchRelation) -> anyhow::Result<()> {
    let reader = SearchIndexReader::empty(indexrel, MvccSatisfies::Vacuum)?;
    let mut segments: Vec<_> = reader
        .segment_readers()
        .iter()
        .map(|segment| SegmentStamp::of(&reader, segment))
        .collect();
    segments.sort_by(|a, b| a.id.cmp(&b.id));
    let total_docs = segments.iter().map(|segment| segment.num_docs).sum();
    let segment_stats: Vec<_> = reader
        .segment_readers()
        .iter()
        .map(SegmentStats::of_reader)
        .collect::<io::Result<_>>()?;

    let mut fields = Vec::new();
    for (_, entry) in reader.schema().fields() {
        let name = entry.name().to_string();
        let field_type = format!("{:?}", entry.field_type());
        let field = reader
            .schema()
            .search_field(&name)
            .expect("schema field exists");
        let mut union: Option<EmpiricalStats> = None;
        let mut complete = true;
        for (segment, stats) in reader
            .segment_readers()
            .iter()
            .zip(&segment_stats)
            .filter(|(segment, _)| reader.segment_num_docs(segment) > 0)
        {
            let persisted = stats
                .as_ref()
                .map(|stats| stats.empirical_for(&field))
                .transpose()?
                .flatten();
            let observed = match persisted {
                Some(stats) => Some(stats),
                None => EmpiricalStats::from_fast_field(segment, &field)?,
            };
            let Some(observed) = observed else {
                complete = false;
                break;
            };
            if union.as_ref().is_some_and(|previous| {
                std::mem::discriminant(&previous.min) != std::mem::discriminant(&observed.min)
                    || std::mem::discriminant(&previous.max)
                        != std::mem::discriminant(&observed.max)
            }) {
                complete = false;
                break;
            }
            union = Some(match union {
                Some(previous) => EmpiricalStats {
                    min: if previous.min.total_cmp(&observed.min) == Ordering::Less {
                        previous.min
                    } else {
                        observed.min
                    },
                    max: if previous.max.total_cmp(&observed.max) == Ordering::Greater {
                        previous.max
                    } else {
                        observed.max
                    },
                    nullable: previous.nullable || observed.nullable,
                },
                None => observed,
            });
        }
        fields.push(FieldSummary {
            name,
            field_type,
            bounds: complete.then(|| union.map(EmpiricalWire::from)).flatten(),
        });
    }
    let bytes = IndexStats {
        total_docs,
        segments,
        fields,
    }
    .encode()?;
    drop(reader);
    MetaPage::replace_index_stats(indexrel, &bytes)?;
    Ok(())
}

/// older indexes have no file until their first VACUUM. a changed segment view is also a miss.
#[allow(dead_code)]
pub(crate) fn load_for(
    indexrel: &PgSearchRelation,
    reader: &SearchIndexReader,
) -> io::Result<Option<IndexStats>> {
    let Some(bytes) = MetaPage::index_stats_bytes(indexrel) else {
        return Ok(None);
    };
    let stats = IndexStats::decode(&bytes)?;
    Ok(stats.covers(reader).then_some(stats))
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use super::*;
    use crate::postgres::pdb_owned_value::PdbOwnedValue;
    use pgrx::prelude::*;

    #[test]
    fn format_marker_and_roundtrip() {
        let stats = IndexStats {
            total_docs: 3,
            segments: Vec::new(),
            fields: Vec::new(),
        };
        let bytes = stats.encode().unwrap();
        assert_eq!(IndexStats::decode(&bytes).unwrap().total_docs, 3);
        assert_eq!(
            IndexStats::decode(&bytes[9..]).err().unwrap().kind(),
            io::ErrorKind::InvalidData
        );
        let mut newer = bytes;
        newer[8] += 1;
        assert_eq!(
            IndexStats::decode(&newer).err().unwrap().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[pg_test]
    fn summary_refreshes_and_rejects_changed_segment_view() {
        Spi::run("CREATE TABLE index_stats_source (id BIGINT, score BIGINT);").unwrap();
        Spi::run("INSERT INTO index_stats_source VALUES (1, 10), (2, NULL), (3, 30);").unwrap();
        Spi::run(
            "CREATE INDEX index_stats_source_idx ON index_stats_source USING paradedb (id, score);",
        )
        .unwrap();
        let oid = Spi::get_one::<pg_sys::Oid>("SELECT 'index_stats_source_idx'::regclass::oid")
            .unwrap()
            .unwrap();
        let indexrel = PgSearchRelation::open(oid);

        // the zero metapage field on a pre-summary index is a legitimate old format.
        assert!(MetaPage::index_stats_bytes(&indexrel).is_none());
        assert!(
            load_for(
                &indexrel,
                &SearchIndexReader::empty(&indexrel, MvccSatisfies::Snapshot).unwrap()
            )
            .unwrap()
            .is_none()
        );

        refresh(&indexrel).unwrap();
        let reader = SearchIndexReader::empty(&indexrel, MvccSatisfies::Snapshot).unwrap();
        let summary = load_for(&indexrel, &reader).unwrap().unwrap();
        assert_eq!(summary.total_docs, 3);
        let id = summary
            .fields
            .iter()
            .find(|field| field.name == "id")
            .unwrap()
            .bounds()
            .unwrap();
        assert_eq!(
            (id.min, id.max),
            (PdbOwnedValue::I64(1), PdbOwnedValue::I64(3))
        );
        assert!(
            summary
                .fields
                .iter()
                .find(|field| field.name == "score")
                .unwrap()
                .bounds()
                .unwrap()
                .nullable
        );
        drop(reader);

        Spi::run("INSERT INTO index_stats_source VALUES (4, 40);").unwrap();
        let changed = SearchIndexReader::empty(&indexrel, MvccSatisfies::Snapshot).unwrap();
        assert!(load_for(&indexrel, &changed).unwrap().is_none());
        drop(changed);

        refresh(&indexrel).unwrap();
        let updated = SearchIndexReader::empty(&indexrel, MvccSatisfies::Snapshot).unwrap();
        assert_eq!(
            load_for(&indexrel, &updated).unwrap().unwrap().total_docs,
            4
        );
    }

    #[pg_test]
    fn vacuum_refreshes_index_wide_summary() {
        Spi::run("CREATE TABLE index_stats_vacuum_source (id BIGINT);").unwrap();
        Spi::run("INSERT INTO index_stats_vacuum_source VALUES (1), (2), (3);").unwrap();
        Spi::run(
            "CREATE INDEX index_stats_vacuum_source_idx ON index_stats_vacuum_source USING paradedb (id);",
        )
        .unwrap();
        let oid =
            Spi::get_one::<pg_sys::Oid>("SELECT 'index_stats_vacuum_source_idx'::regclass::oid")
                .unwrap()
                .unwrap();
        let indexrel = PgSearchRelation::open(oid);
        assert!(MetaPage::index_stats_bytes(&indexrel).is_none());

        let mut info: pg_sys::IndexVacuumInfo = unsafe { std::mem::zeroed() };
        info.index = indexrel.as_ptr();
        unsafe {
            crate::postgres::vacuum::amvacuumcleanup(&mut info, std::ptr::null_mut());
        }

        let reader = SearchIndexReader::empty(&indexrel, MvccSatisfies::Snapshot).unwrap();
        assert_eq!(load_for(&indexrel, &reader).unwrap().unwrap().total_docs, 3);
    }
}
