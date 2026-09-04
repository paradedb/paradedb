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

//! The tantivy side of the component: what a segment write and a merge put in the file.
//!
//! One `CompositeFile` per segment, keyed by `(Field, idx)`. `idx = 0` holds the empirical
//! `min`/`max` of a fast field, `idx = 1` the box a partitioned build assigned to the segment's
//! partition, and `idx = 2` stays reserved for sketches. The footer maps each entry to a byte
//! range, so a reader touches only the entries it asks for.
//!
//! The two entries have different lifecycles. Empirical stats come from the segment's own
//! `.fast` file, so every immutable segment gets them, at write and at merge. Boxes come from
//! the build that routed the rows; a merge keeps them only when every source has one, widened
//! to the union box, which still holds every row.

use std::any::Any;
use std::io::Write;
use std::sync::Arc;

use serde::Serialize;
use tantivy::columnar::{Cardinality, ColumnarReader, DynamicColumn, DynamicColumnHandle};
use tantivy::directory::error::OpenReadError;
use tantivy::directory::CompositeWrite;
use tantivy::index::{Segment, SegmentComponent, SegmentReader};
use tantivy::indexer::DocIdMapping;
use tantivy::schema::{Field, FieldType, Schema};
use tantivy::{
    Index, PluginMergeContext, PluginWriter, PluginWriterContext, SegmentPlugin, TantivyError,
};

use super::{
    EmpiricalStats, EmpiricalWire, LogicalBounds, LogicalBoundsByField, LogicalWire, SegmentStats,
    EMPIRICAL_IDX, LOGICAL_IDX, STATS_EXT,
};
use crate::api::HashMap;
use crate::postgres::datetime::PostgresDateTime;
use crate::postgres::pdb_owned_value::PdbOwnedValue;

/// The plugin that writes and merges the `.stats` component.
pub(crate) struct StatsPlugin;

/// Attaches the plugin to an index. Every `Index` pg_search writes or merges through must call
/// this: tantivy restores only its built-in plugins, and the index metadata lists `stats` as
/// required. A read-only `Index` has no use for it.
pub(crate) fn register(index: &mut Index) {
    index.register_plugin(Arc::new(StatsPlugin));
}

impl SegmentPlugin for StatsPlugin {
    fn extensions(&self) -> &[&str] {
        &[STATS_EXT]
    }

    fn create_writer(&self, _ctx: &PluginWriterContext) -> tantivy::Result<Box<dyn PluginWriter>> {
        Ok(Box::new(StatsWriter { logical: None }))
    }

    fn merge(&self, ctx: PluginMergeContext) -> tantivy::Result<()> {
        let logical = merged_logical_bounds(ctx.readers, ctx.schema)?;
        write_stats(ctx.target_segment, ctx.schema, logical.as_ref())
    }
}

/// Per-segment writer. It records nothing per document: the statistics come off the finished
/// `.fast` file, which the built-in plugins serialize before this one runs.
pub(crate) struct StatsWriter {
    logical: Option<Arc<LogicalBoundsByField>>,
}

impl StatsWriter {
    pub(crate) fn set_logical_bounds(&mut self, bounds: Arc<LogicalBoundsByField>) {
        self.logical = Some(bounds);
    }
}

impl PluginWriter for StatsWriter {
    fn serialize(
        self: Box<Self>,
        segment: &Segment,
        _doc_id_map: Option<&DocIdMapping>,
    ) -> tantivy::Result<()> {
        write_stats(segment, &segment.schema(), self.logical.as_deref())
    }

    fn mem_usage(&self) -> usize {
        0
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub(super) fn stats_component() -> SegmentComponent {
    SegmentComponent::Custom(STATS_EXT.to_string())
}

fn encode<T: Serialize>(value: &T) -> tantivy::Result<Vec<u8>> {
    postcard::to_allocvec(value).map_err(|e| TantivyError::InternalError(e.to_string()))
}

fn write_stats(
    segment: &Segment,
    schema: &Schema,
    logical: Option<&LogicalBoundsByField>,
) -> tantivy::Result<()> {
    let empirical = empirical_stats(segment, schema)?;
    let mut write = CompositeWrite::wrap(segment.open_write(stats_component())?);
    for (field, stats) in empirical {
        let bytes = encode(&EmpiricalWire::from(stats))?;
        write
            .for_field_with_idx(field, EMPIRICAL_IDX)
            .write_all(&bytes)?;
    }
    for (name, bounds) in logical.into_iter().flatten() {
        let Ok(field) = schema.get_field(name) else {
            continue;
        };
        if !logical_bounds_hold(schema, field) {
            continue;
        }
        let bytes = encode(&LogicalWire::from(bounds))?;
        write
            .for_field_with_idx(field, LOGICAL_IDX)
            .write_all(&bytes)?;
    }
    write.close()?;
    Ok(())
}

/// The build routes on raw values, but a partition's range query runs on the fast column. A
/// text normalizer other than `raw` reorders that column, so a box in raw order could prune a
/// segment the query still needs.
pub(crate) fn logical_bounds_hold(schema: &Schema, field: Field) -> bool {
    match schema.get_field_entry(field).field_type() {
        FieldType::Str(options) => options.get_fast_field_tokenizer_name() == Some("raw"),
        _ => true,
    }
}

/// `min`/`max` of every fast field column of `segment`, read off its `.fast` file.
fn empirical_stats(
    segment: &Segment,
    schema: &Schema,
) -> tantivy::Result<Vec<(Field, EmpiricalStats)>> {
    let fast = match segment.open_read(SegmentComponent::FastFields) {
        Ok(fast) => fast,
        Err(OpenReadError::FileDoesNotExist(_)) => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    let columnar = ColumnarReader::open(fast)?;
    let mut by_field: HashMap<Field, Vec<DynamicColumnHandle>> = HashMap::default();
    for (name, handle) in columnar.iter_columns()? {
        // JSON subpaths carry no schema field of their own.
        let Ok(field) = schema.get_field(&name) else {
            continue;
        };
        by_field.entry(field).or_default().push(handle);
    }
    let mut stats = Vec::with_capacity(by_field.len());
    for (field, handles) in by_field {
        // A field spread over several typed columns has no single order to bound.
        let [handle] = handles.as_slice() else {
            continue;
        };
        if let Some(column_stats) = column_stats(handle)? {
            stats.push((field, column_stats));
        }
    }
    stats.sort_by_key(|(field, _)| field.field_id());
    Ok(stats)
}

fn column_stats(handle: &DynamicColumnHandle) -> tantivy::Result<Option<EmpiricalStats>> {
    let (min, max, cardinality) = match handle.open()? {
        DynamicColumn::I64(c) => (
            PdbOwnedValue::I64(c.min_value()),
            PdbOwnedValue::I64(c.max_value()),
            c.get_cardinality(),
        ),
        DynamicColumn::U64(c) => (
            PdbOwnedValue::U64(c.min_value()),
            PdbOwnedValue::U64(c.max_value()),
            c.get_cardinality(),
        ),
        DynamicColumn::F64(c) => (
            PdbOwnedValue::F64(c.min_value()),
            PdbOwnedValue::F64(c.max_value()),
            c.get_cardinality(),
        ),
        DynamicColumn::Bool(c) => (
            PdbOwnedValue::Bool(c.min_value()),
            PdbOwnedValue::Bool(c.max_value()),
            c.get_cardinality(),
        ),
        DynamicColumn::DateTime(c) => {
            // A value Postgres can't represent has no place in a segment, but a statistic is
            // not worth failing the write over.
            let (Ok(min), Ok(max)) = (
                PostgresDateTime::try_from(c.min_value()),
                PostgresDateTime::try_from(c.max_value()),
            ) else {
                return Ok(None);
            };
            (
                PdbOwnedValue::Date(min),
                PdbOwnedValue::Date(max),
                c.get_cardinality(),
            )
        }
        DynamicColumn::IpAddr(c) => (
            PdbOwnedValue::IpAddr(c.min_value()),
            PdbOwnedValue::IpAddr(c.max_value()),
            c.get_cardinality(),
        ),
        DynamicColumn::Bytes(c) => {
            let num_terms = c.num_terms();
            if num_terms == 0 {
                return Ok(None);
            }
            let (mut min, mut max) = (Vec::new(), Vec::new());
            c.ord_to_bytes(0, &mut min)?;
            c.ord_to_bytes(num_terms as u64 - 1, &mut max)?;
            (
                PdbOwnedValue::Bytes(min),
                PdbOwnedValue::Bytes(max),
                c.ords().get_cardinality(),
            )
        }
        DynamicColumn::Str(c) => {
            let num_terms = c.num_terms();
            if num_terms == 0 {
                return Ok(None);
            }
            let (mut min, mut max) = (String::new(), String::new());
            c.ord_to_str(0, &mut min)?;
            c.ord_to_str(num_terms as u64 - 1, &mut max)?;
            (
                PdbOwnedValue::Str(min),
                PdbOwnedValue::Str(max),
                c.ords().get_cardinality(),
            )
        }
    };
    Ok(Some(EmpiricalStats {
        min,
        max,
        nullable: cardinality != Cardinality::Full,
    }))
}

/// The union of the sources' logical bounds, per field, or `None` when any source has no
/// `.stats` at all. A field is kept only when every source bounds it.
fn merged_logical_bounds(
    readers: &[SegmentReader],
    schema: &Schema,
) -> tantivy::Result<Option<LogicalBoundsByField>> {
    let mut sources = Vec::with_capacity(readers.len());
    for reader in readers {
        match SegmentStats::of_reader(reader)? {
            Some(stats) => sources.push(stats),
            // A segment written before the component existed. The merge cannot claim a box it
            // cannot prove for it.
            None => return Ok(None),
        }
    }
    if sources.is_empty() {
        return Ok(None);
    }
    let mut merged = LogicalBoundsByField::new();
    'fields: for (field, entry) in schema.fields() {
        let mut union: Option<LogicalBounds> = None;
        for source in &sources {
            let Some(bounds) = source.logical(field)? else {
                continue 'fields;
            };
            union = Some(match union {
                None => bounds,
                Some(current) => current.union(&bounds),
            });
        }
        if let Some(union) = union {
            merged.insert(entry.name().to_string(), union);
        }
    }
    Ok(Some(merged))
}
