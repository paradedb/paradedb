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

//! Per-segment statistics: the `.stats` component.
//!
//! One `CompositeFile` per segment, keyed by `(Field, idx)`. `idx = 0` holds the empirical
//! `min`/`max` of a fast field, `idx = 1` the logical box a partitioned build assigned to the
//! segment's cell, and `idx = 2` stays reserved for sketches. The footer maps each entry to a
//! byte range, so a reader touches only the entries it asks for. A missing file or entry means
//! "unknown", and a consumer must keep the segment.
//!
//! The two entries have different lifecycles. Empirical stats come from the segment's own
//! `.fast` file, so every immutable segment gets them, at write and at merge. Logical bounds come
//! from the build that routed the rows; a merge keeps them only when every source has them,
//! widened to the union box, which still holds every row.

use std::any::Any;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::io::{self, Write};
use std::ops::Bound;
use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tantivy::columnar::{Cardinality, ColumnarReader, DynamicColumn, DynamicColumnHandle};
use tantivy::directory::error::OpenReadError;
use tantivy::directory::footer::Footer;
use tantivy::directory::{CompositeFile, CompositeWrite, FileSlice};
use tantivy::index::{Segment, SegmentComponent, SegmentId, SegmentReader};
use tantivy::indexer::DocIdMapping;
use tantivy::schema::{Field, FieldType, Schema};
use tantivy::{
    Index, PluginMergeContext, PluginWriter, PluginWriterContext, SegmentPlugin, TantivyError,
};

use crate::api::HashMap;
use crate::index::mvcc::MvccSatisfies;
use crate::index::reader::index::SearchIndexReader;
use crate::index::reader::segment_component::SegmentComponentReader;
use crate::postgres::datetime::PostgresDateTime;
use crate::postgres::pdb_owned_value::{PdbOwnedValue, exact_scalar_wire};
use crate::postgres::rel::PgSearchRelation;
use crate::postgres::storage::block::{STATS_EXT, SegmentMetaEntry};
use crate::postgres::types::is_datetime_type;
use crate::scan::range_partitioning::RangePartitioning;
use crate::schema::SearchFieldType;

const EMPIRICAL_IDX: usize = 0;
const LOGICAL_IDX: usize = 1;

/// Empirical `min`/`max` of one fast field within one segment. `nullable` is false only when
/// every document holds exactly one value, so a segment without it may hide NULLs.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EmpiricalStats {
    pub(crate) min: PdbOwnedValue,
    pub(crate) max: PdbOwnedValue,
    pub(crate) nullable: bool,
}

/// The half-open box a partitioned build assigned to a segment's cell on one field.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LogicalBounds {
    pub(crate) lower: Bound<PdbOwnedValue>,
    pub(crate) upper: Bound<PdbOwnedValue>,
}

/// Logical bounds keyed by field name, in the order the file lays them out.
pub(crate) type LogicalBoundsByField = BTreeMap<String, LogicalBounds>;

/// The default `PdbOwnedValue` serde is lossy (a non-negative `I64` comes back as `U64`), so the
/// entries carry the exact scalar wire form the kd-tree also ships over the DSM.
#[derive(Serialize, Deserialize)]
struct WireScalar(#[serde(with = "exact_scalar_wire")] PdbOwnedValue);

#[derive(Serialize, Deserialize)]
struct EmpiricalWire {
    nullable: bool,
    min: WireScalar,
    max: WireScalar,
}

#[derive(Serialize, Deserialize)]
struct LogicalWire {
    lower: Bound<WireScalar>,
    upper: Bound<WireScalar>,
}

impl From<EmpiricalStats> for EmpiricalWire {
    fn from(stats: EmpiricalStats) -> Self {
        Self {
            nullable: stats.nullable,
            min: WireScalar(stats.min),
            max: WireScalar(stats.max),
        }
    }
}

impl From<EmpiricalWire> for EmpiricalStats {
    fn from(wire: EmpiricalWire) -> Self {
        Self {
            min: wire.min.0,
            max: wire.max.0,
            nullable: wire.nullable,
        }
    }
}

impl From<&LogicalBounds> for LogicalWire {
    fn from(bounds: &LogicalBounds) -> Self {
        Self {
            lower: bounds.lower.as_ref().map(|v| WireScalar(v.clone())),
            upper: bounds.upper.as_ref().map(|v| WireScalar(v.clone())),
        }
    }
}

impl From<LogicalWire> for LogicalBounds {
    fn from(wire: LogicalWire) -> Self {
        Self {
            lower: wire.lower.map(|v| v.0),
            upper: wire.upper.map(|v| v.0),
        }
    }
}

/// True when `hi` ends before `lo` starts, so two ranges with these ends cannot share a value.
fn ends_before(hi: Bound<&PdbOwnedValue>, lo: Bound<&PdbOwnedValue>) -> bool {
    match (hi, lo) {
        (Bound::Unbounded, _) | (_, Bound::Unbounded) => false,
        (Bound::Included(h), Bound::Included(l)) => h.total_cmp(l) == Ordering::Less,
        (Bound::Included(h), Bound::Excluded(l))
        | (Bound::Excluded(h), Bound::Included(l))
        | (Bound::Excluded(h), Bound::Excluded(l)) => h.total_cmp(l) != Ordering::Greater,
    }
}

fn ranges_intersect(
    a_lo: Bound<&PdbOwnedValue>,
    a_hi: Bound<&PdbOwnedValue>,
    b_lo: Bound<&PdbOwnedValue>,
    b_hi: Bound<&PdbOwnedValue>,
) -> bool {
    !ends_before(a_hi, b_lo) && !ends_before(b_hi, a_lo)
}

/// The looser of two lower bounds. At an equal value the inclusive one is looser.
fn min_lower(a: &Bound<PdbOwnedValue>, b: &Bound<PdbOwnedValue>) -> Bound<PdbOwnedValue> {
    match (a, b) {
        (Bound::Unbounded, _) | (_, Bound::Unbounded) => Bound::Unbounded,
        (Bound::Included(x), Bound::Included(y)) | (Bound::Excluded(x), Bound::Excluded(y)) => {
            if x.total_cmp(y) == Ordering::Greater {
                b.clone()
            } else {
                a.clone()
            }
        }
        (Bound::Included(x), Bound::Excluded(y)) => {
            if x.total_cmp(y) == Ordering::Greater {
                b.clone()
            } else {
                a.clone()
            }
        }
        (Bound::Excluded(x), Bound::Included(y)) => {
            if y.total_cmp(x) == Ordering::Greater {
                a.clone()
            } else {
                b.clone()
            }
        }
    }
}

/// The looser of two upper bounds. At an equal value the inclusive one is looser.
fn max_upper(a: &Bound<PdbOwnedValue>, b: &Bound<PdbOwnedValue>) -> Bound<PdbOwnedValue> {
    match (a, b) {
        (Bound::Unbounded, _) | (_, Bound::Unbounded) => Bound::Unbounded,
        (Bound::Included(x), Bound::Included(y)) | (Bound::Excluded(x), Bound::Excluded(y)) => {
            if x.total_cmp(y) == Ordering::Less {
                b.clone()
            } else {
                a.clone()
            }
        }
        (Bound::Included(x), Bound::Excluded(y)) => {
            if x.total_cmp(y) == Ordering::Less {
                b.clone()
            } else {
                a.clone()
            }
        }
        (Bound::Excluded(x), Bound::Included(y)) => {
            if y.total_cmp(x) == Ordering::Less {
                a.clone()
            } else {
                b.clone()
            }
        }
    }
}

/// Whether `total_cmp` ranks these two values by value. The derived order it falls back to
/// ranks by variant, so a bound of one kind against a statistic of another says nothing.
fn comparable(a: &PdbOwnedValue, b: &PdbOwnedValue) -> bool {
    use PdbOwnedValue::*;
    matches!(
        (a, b),
        (I64(_) | U64(_), I64(_) | U64(_))
            | (F64(_), F64(_))
            | (Str(_), Str(_))
            | (Bytes(_), Bytes(_))
            | (Bool(_), Bool(_))
            | (Date(_), Date(_))
            | (IpAddr(_), IpAddr(_))
    )
}

fn bound_comparable(bound: &Bound<PdbOwnedValue>, value: &PdbOwnedValue) -> bool {
    match bound {
        Bound::Unbounded => true,
        Bound::Included(b) | Bound::Excluded(b) => comparable(b, value),
    }
}

impl LogicalBounds {
    /// The smallest box that holds both.
    pub(crate) fn union(&self, other: &Self) -> Self {
        Self {
            lower: min_lower(&self.lower, &other.lower),
            upper: max_upper(&self.upper, &other.upper),
        }
    }

    /// Whether a value in this box can fall in `[lower, upper)`. Bounds of a kind this box
    /// cannot be ranked against count as intersecting.
    pub(crate) fn intersects(
        &self,
        lower: &Bound<PdbOwnedValue>,
        upper: &Bound<PdbOwnedValue>,
    ) -> bool {
        for own in [&self.lower, &self.upper] {
            if let Bound::Included(v) | Bound::Excluded(v) = own
                && !(bound_comparable(lower, v) && bound_comparable(upper, v))
            {
                return true;
            }
        }
        ranges_intersect(
            self.lower.as_ref(),
            self.upper.as_ref(),
            lower.as_ref(),
            upper.as_ref(),
        )
    }

    /// A build routes NULLs below every split, so only a box open at the bottom can hold them.
    pub(crate) fn may_hold_nulls(&self) -> bool {
        matches!(self.lower, Bound::Unbounded)
    }
}

impl EmpiricalStats {
    /// Whether a value in `[min, max]` can fall in `[lower, upper)`. Bounds of a kind these
    /// statistics cannot be ranked against count as intersecting.
    pub(crate) fn intersects(
        &self,
        lower: &Bound<PdbOwnedValue>,
        upper: &Bound<PdbOwnedValue>,
    ) -> bool {
        if !(bound_comparable(lower, &self.min) && bound_comparable(upper, &self.max)) {
            return true;
        }
        ranges_intersect(
            Bound::Included(&self.min),
            Bound::Included(&self.max),
            lower.as_ref(),
            upper.as_ref(),
        )
    }

    /// Datetimes of a recent index sit in an `I64` column; a query bound on that field is a
    /// `Date`, so lift the statistics the way `FFType` lifts the column's values.
    fn into_dates(self) -> Option<Self> {
        let lift = |v: PdbOwnedValue| match v {
            PdbOwnedValue::I64(raw) => PostgresDateTime::try_from_raw(raw)
                .ok()
                .map(PdbOwnedValue::Date),
            other => Some(other),
        };
        Some(Self {
            min: lift(self.min)?,
            max: lift(self.max)?,
            nullable: self.nullable,
        })
    }
}

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

fn stats_component() -> SegmentComponent {
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
fn logical_bounds_hold(schema: &Schema, field: Field) -> bool {
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
        match reader.open_read(stats_component()) {
            Ok(slice) => sources.push(SegmentStats::open(slice)?),
            // A segment written before the component existed. The merge cannot claim a box it
            // cannot prove for it.
            Err(OpenReadError::FileDoesNotExist(_)) => return Ok(None),
            Err(e) => return Err(e.into()),
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

/// A segment's `.stats` file, opened on its footer. Entries are decoded on request.
pub(crate) struct SegmentStats {
    file: CompositeFile,
}

impl SegmentStats {
    pub(crate) fn open(slice: FileSlice) -> io::Result<Self> {
        Ok(Self {
            file: CompositeFile::open(&slice)?,
        })
    }

    /// The `.stats` of a persisted segment, read straight off the index blocks. `None` when the
    /// segment was written without the component.
    pub(crate) fn open_persisted(
        indexrel: &PgSearchRelation,
        entry: &SegmentMetaEntry,
    ) -> io::Result<Option<Self>> {
        let Some(file_entry) = entry.stats() else {
            return Ok(None);
        };
        let handle =
            unsafe { SegmentComponentReader::new(indexrel, file_entry, Some(stats_component())) };
        // A direct block read still carries tantivy's file footer, which the managed directory
        // strips for the segment readers.
        let (_, body) = Footer::extract_footer(FileSlice::new(Arc::new(handle)))?;
        Self::open(body).map(Some)
    }

    pub(crate) fn empirical(&self, field: Field) -> io::Result<Option<EmpiricalStats>> {
        Ok(self
            .read::<EmpiricalWire>(field, EMPIRICAL_IDX)?
            .map(EmpiricalStats::from))
    }

    pub(crate) fn logical(&self, field: Field) -> io::Result<Option<LogicalBounds>> {
        Ok(self
            .read::<LogicalWire>(field, LOGICAL_IDX)?
            .map(LogicalBounds::from))
    }

    fn read<T: DeserializeOwned>(&self, field: Field, idx: usize) -> io::Result<Option<T>> {
        let Some(slice) = self.file.open_read_with_idx(field, idx) else {
            return Ok(None);
        };
        let bytes = slice.read_bytes()?;
        postcard::from_bytes(&bytes)
            .map(Some)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

/// The split grid a partitioned build stamped on the visible segments for `partition_by`,
/// sorted and deduplicated, or `None` once any visible segment lacks a box on it. An insert or a
/// merge with an unboxed segment breaks the grid, and what is left of it would give a coarse,
/// skewed layout where a value sample still divides the space evenly.
pub(crate) fn persisted_split_points(
    indexrel: &PgSearchRelation,
    partition_by: &str,
) -> anyhow::Result<Option<Vec<PdbOwnedValue>>> {
    if indexrel.options().partition_by().is_empty() {
        return Ok(None);
    }
    let directory = MvccSatisfies::Snapshot.directory(indexrel);
    let index = Index::open(directory.clone())?;
    let Ok(field) = index.schema().get_field(partition_by) else {
        return Ok(None);
    };
    let mut points = Vec::new();
    for entry in directory.all_entries().values() {
        let Some(stats) = SegmentStats::open_persisted(indexrel, entry)? else {
            return Ok(None);
        };
        let Some(bounds) = stats.logical(field)? else {
            return Ok(None);
        };
        for bound in [bounds.lower, bounds.upper] {
            if let Bound::Included(v) | Bound::Excluded(v) = bound {
                points.push(v);
            }
        }
    }
    points.sort_unstable_by(PdbOwnedValue::total_cmp);
    points.dedup_by(|a, b| a.total_cmp(b) == Ordering::Equal);
    Ok((!points.is_empty()).then_some(points))
}

/// The segments of `reader` that can hold a row of `partition`. A segment without statistics
/// it can be ranked against is kept, so the query the caller still applies stays the source of
/// truth.
pub(crate) fn segments_for_partition(
    reader: &SearchIndexReader,
    boundaries: &RangePartitioning,
    partition: usize,
) -> Vec<SegmentId> {
    let all = reader.segment_ids();
    let Some((lower, upper)) = boundaries.partition_range(partition) else {
        return all;
    };
    let field_name = boundaries.partition_by.as_ref();
    let Ok(field) = reader.schema().tantivy_schema().get_field(field_name) else {
        return all;
    };
    // A recent index keeps datetimes in an `I64` column, so its statistics need the same lift
    // as its values; a legacy index stored them as `Date` already.
    let is_date = match reader
        .schema()
        .search_field(field_name)
        .map(|f| f.field_type())
    {
        Some(SearchFieldType::Date(_)) => true,
        Some(SearchFieldType::I64(oid)) => is_datetime_type(oid),
        _ => false,
    };
    // NULLs route to partition 0, so it keeps every segment that may hold one.
    let catches_nulls = partition == 0;

    all.into_iter()
        .filter(|segment_id| {
            let Some(entry) = reader.segment_meta_entry(segment_id) else {
                return true;
            };
            let Ok(Some(stats)) = SegmentStats::open_persisted(reader.index_rel(), &entry) else {
                return true;
            };
            match stats.logical(field) {
                Ok(Some(bounds)) => {
                    return (catches_nulls && bounds.may_hold_nulls())
                        || bounds.intersects(&lower, &upper);
                }
                Ok(None) => {}
                Err(_) => return true,
            }
            let Ok(Some(empirical)) = stats.empirical(field) else {
                return true;
            };
            let empirical = if is_date {
                match empirical.into_dates() {
                    Some(empirical) => empirical,
                    None => return true,
                }
            } else {
                empirical
            };
            (catches_nulls && empirical.nullable) || empirical.intersects(&lower, &upper)
        })
        .collect()
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use std::ops::Bound;

    use pgrx::prelude::*;
    use tantivy::Index;

    use super::*;
    use crate::api::FieldName;
    use crate::query::SearchQueryInput;

    fn open_index(index: &str) -> PgSearchRelation {
        let oid = Spi::get_one::<pg_sys::Oid>(&format!("SELECT '{index}'::regclass::oid"))
            .unwrap()
            .unwrap();
        PgSearchRelation::open(oid)
    }

    /// Every persisted segment's `.stats`, with the tantivy field for `field_name`.
    fn segment_stats(indexrel: &PgSearchRelation, field_name: &str) -> (Field, Vec<SegmentStats>) {
        let directory = MvccSatisfies::Snapshot.directory(indexrel);
        let index = Index::open(directory.clone()).unwrap();
        let field = index.schema().get_field(field_name).unwrap();
        let stats = directory
            .all_entries()
            .values()
            .map(|entry| {
                SegmentStats::open_persisted(indexrel, entry)
                    .unwrap()
                    .expect("every segment of a fresh index carries a .stats component")
            })
            .collect();
        (field, stats)
    }

    fn below(a: &PdbOwnedValue, b: &PdbOwnedValue) -> bool {
        a.total_cmp(b) == Ordering::Less
    }

    #[pg_test]
    fn empirical_stats_match_the_table() {
        Spi::run(
            r#"
            CREATE TABLE stats_src (
                id BIGSERIAL PRIMARY KEY, tenant_id BIGINT, name TEXT, seen TIMESTAMP,
                score FLOAT8, flag BOOLEAN
            );
            INSERT INTO stats_src (tenant_id, name, seen, score, flag)
            SELECT CASE WHEN i % 10 = 0 THEN NULL ELSE (i * 7919) % 100 END,
                   'name' || lpad(i::text, 5, '0'),
                   TIMESTAMP '2024-01-01' + (i || ' minutes')::interval,
                   (i % 1000)::float8 / 10,
                   i % 3 = 0
            FROM generate_series(1, 5000) i;
            SET max_parallel_maintenance_workers = 0;
            CREATE INDEX stats_src_idx ON stats_src
                USING paradedb (id, tenant_id, name, seen, score, flag)
                WITH (key_field = 'id', target_segment_count = 1,
                      text_fields = '{"name": {"tokenizer": {"type": "keyword"}, "fast": true}}');
            "#,
        )
        .unwrap();
        let indexrel = open_index("stats_src_idx");
        let index = Index::open(MvccSatisfies::Snapshot.directory(&indexrel)).unwrap();
        let schema = index.schema();
        let (_, stats) = segment_stats(&indexrel, "id");
        let [stats] = stats.as_slice() else {
            panic!("expected one segment, got {}", stats.len());
        };
        let empirical = |name: &str| {
            stats
                .empirical(schema.get_field(name).unwrap())
                .unwrap()
                .unwrap_or_else(|| panic!("no empirical stats for {name}"))
        };

        let id = empirical("id");
        assert_eq!(
            (id.min, id.max, id.nullable),
            (PdbOwnedValue::I64(1), PdbOwnedValue::I64(5000), false)
        );

        // The NULL tenants are exactly the multiples of ten, so the extremes survive them.
        let tenant = empirical("tenant_id");
        assert_eq!(
            (tenant.min, tenant.max, tenant.nullable),
            (PdbOwnedValue::I64(1), PdbOwnedValue::I64(99), true)
        );

        let name = empirical("name");
        assert_eq!(name.min, PdbOwnedValue::Str("name00001".into()));
        assert_eq!(name.max, PdbOwnedValue::Str("name05000".into()));

        // Datetimes live in an I64 column of raw Postgres microseconds.
        let raw = |agg: &str| {
            Spi::get_one::<i64>(&format!(
                "SELECT ((EXTRACT(EPOCH FROM {agg}(seen)) - 946684800) * 1000000)::bigint FROM stats_src"
            ))
            .unwrap()
            .unwrap()
        };
        let seen = empirical("seen");
        assert_eq!(
            (seen.min, seen.max),
            (
                PdbOwnedValue::I64(raw("min")),
                PdbOwnedValue::I64(raw("max"))
            )
        );

        let score = empirical("score");
        assert_eq!(
            (score.min, score.max),
            (PdbOwnedValue::F64(0.0), PdbOwnedValue::F64(99.9))
        );

        let flag = empirical("flag");
        assert_eq!(
            (flag.min, flag.max),
            (PdbOwnedValue::Bool(false), PdbOwnedValue::Bool(true))
        );

        // No logical bounds without a partitioned build.
        assert!(
            stats
                .logical(schema.get_field("id").unwrap())
                .unwrap()
                .is_none()
        );
    }

    /// A regular build whose rows outgrow the writer budget flushes several segments and merges
    /// them at commit, so the surviving segment's statistics come from the merge path.
    #[pg_test]
    fn merge_recomputes_empirical_stats() {
        Spi::run(
            r#"
            CREATE TABLE stats_merge (id BIGSERIAL PRIMARY KEY, tenant_id BIGINT, name TEXT);
            INSERT INTO stats_merge (tenant_id, name)
            SELECT (i * 7919) % 4,
                   (SELECT string_agg(md5((i * 32 + j)::text), ' ') FROM generate_series(1, 32) j)
            FROM generate_series(1, 24000) i;
            SET max_parallel_maintenance_workers = 0;
            SET maintenance_work_mem = '16MB';
            CREATE INDEX stats_merge_idx ON stats_merge USING paradedb (id, tenant_id, name)
                WITH (key_field = 'id', target_segment_count = 1);
            "#,
        )
        .unwrap();
        let indexrel = open_index("stats_merge_idx");
        let (field, stats) = segment_stats(&indexrel, "id");
        assert_eq!(stats.len(), 1, "the build merges down to one segment");
        let id = stats[0].empirical(field).unwrap().unwrap();
        assert_eq!(
            (id.min, id.max),
            (PdbOwnedValue::I64(1), PdbOwnedValue::I64(24000))
        );
    }

    /// A partitioned build stamps each cell's box on its segment. The persisted split points
    /// then replace sampling, and each partition of a range scan maps to exactly its cell.
    #[pg_test]
    fn partitioned_build_stamps_bounds_and_prunes() {
        Spi::run(
            r#"
            CREATE TABLE stats_part (id BIGSERIAL PRIMARY KEY, tenant_id BIGINT, name TEXT);
            INSERT INTO stats_part (tenant_id, name)
            SELECT CASE WHEN i % 10 = 0 THEN NULL ELSE (i * 7919) % 100 END,
                   'lorem ipsum ' || i || ' ' || repeat('padding word here ', 50)
            FROM generate_series(1, 20000) i;
            SET max_parallel_maintenance_workers = 0;
            CREATE INDEX stats_part_idx ON stats_part USING paradedb (id, tenant_id, name)
                WITH (key_field = 'id', partition_by = 'tenant_id', target_segment_count = 8);
            "#,
        )
        .unwrap();
        let indexrel = open_index("stats_part_idx");
        let (field, stats) = segment_stats(&indexrel, "tenant_id");
        assert_eq!(stats.len(), 8, "one segment per cell");

        let mut open_below = 0;
        let mut open_above = 0;
        for segment in &stats {
            let bounds = segment
                .logical(field)
                .unwrap()
                .expect("every cell has a box");
            open_below += usize::from(matches!(bounds.lower, Bound::Unbounded));
            open_above += usize::from(matches!(bounds.upper, Bound::Unbounded));
            // The rows inside the box: what the build routed there. NULLs route below every
            // split, so only the bottom cell may hold them.
            let empirical = segment.empirical(field).unwrap().unwrap();
            assert_eq!(empirical.nullable, bounds.may_hold_nulls(), "{bounds:?}");
            if let Bound::Included(lower) = &bounds.lower {
                assert!(!below(&empirical.min, lower), "{empirical:?} vs {bounds:?}");
            }
            if let Bound::Excluded(upper) = &bounds.upper {
                assert!(below(&empirical.max, upper), "{empirical:?} vs {bounds:?}");
            }
        }
        assert_eq!((open_below, open_above), (1, 1));

        let split_points = persisted_split_points(&indexrel, "tenant_id")
            .unwrap()
            .expect("every cell segment carries its box");
        assert_eq!(split_points.len(), 7, "{split_points:?}");
        assert!(split_points.windows(2).all(|w| below(&w[0], &w[1])));

        let reader = SearchIndexReader::open(
            &indexrel,
            SearchQueryInput::All,
            false,
            MvccSatisfies::Snapshot,
        )
        .unwrap();
        let boundaries = RangePartitioning {
            partition_by: FieldName::from("tenant_id"),
            split_points,
        };
        let mut chosen = Vec::new();
        for partition in 0..8 {
            let segments = segments_for_partition(&reader, &boundaries, partition);
            assert_eq!(segments.len(), 1, "partition {partition}: {segments:?}");
            chosen.extend(segments);
        }
        chosen.sort();
        chosen.dedup();
        assert_eq!(chosen.len(), 8, "each partition maps to its own cell");

        // A coarser layout keeps every cell its range reaches, and nothing else.
        let coarse = RangePartitioning {
            partition_by: FieldName::from("tenant_id"),
            split_points: vec![boundaries.split_points[3].clone()],
        };
        let low = segments_for_partition(&reader, &coarse, 0);
        let high = segments_for_partition(&reader, &coarse, 1);
        assert_eq!((low.len(), high.len()), (4, 4));
    }

    /// Without a partitioned build, the empirical `min`/`max` still prunes: a serial build over
    /// a heap in key order gives every segment a disjoint key range.
    #[pg_test]
    fn empirical_stats_prune_unpartitioned_segments() {
        Spi::run(
            r#"
            CREATE TABLE stats_plain (id BIGSERIAL PRIMARY KEY, name TEXT);
            INSERT INTO stats_plain (name)
            SELECT 'row ' || i || ' ' || repeat('padding word here ', 50) FROM generate_series(1, 20000) i;
            ANALYZE stats_plain;
            SET max_parallel_maintenance_workers = 0;
            CREATE INDEX stats_plain_idx ON stats_plain USING paradedb (id, name)
                WITH (key_field = 'id', target_segment_count = 4);
            "#,
        )
        .unwrap();
        let indexrel = open_index("stats_plain_idx");
        let (field, stats) = segment_stats(&indexrel, "id");
        assert!(stats.len() > 1, "need several segments to prune between");
        let ranges: Vec<EmpiricalStats> = stats
            .iter()
            .map(|s| s.empirical(field).unwrap().unwrap())
            .collect();

        let reader = SearchIndexReader::open(
            &indexrel,
            SearchQueryInput::All,
            false,
            MvccSatisfies::Snapshot,
        )
        .unwrap();
        let boundaries = RangePartitioning {
            partition_by: FieldName::from("id"),
            split_points: vec![PdbOwnedValue::I64(5001), PdbOwnedValue::I64(15001)],
        };
        let mut pruned_somewhere = false;
        for partition in 0..3 {
            let (lower, upper) = boundaries.partition_range(partition).unwrap();
            let expected = ranges
                .iter()
                .filter(|r| r.intersects(&lower, &upper))
                .count();
            let chosen = segments_for_partition(&reader, &boundaries, partition);
            assert_eq!(chosen.len(), expected, "partition {partition}");
            pruned_somewhere |= chosen.len() < stats.len();
        }
        assert!(pruned_somewhere);
    }

    /// A cell that outgrows the writer budget flushes several segments, and `finish_cell`
    /// merges them: the merged segment must keep the cell's box.
    #[pg_test]
    fn cell_merge_keeps_logical_bounds() {
        Spi::run(
            r#"
            CREATE TABLE stats_cell_merge (id BIGSERIAL PRIMARY KEY, tenant_id BIGINT, name TEXT);
            INSERT INTO stats_cell_merge (tenant_id, name)
            SELECT (i * 7919) % 4,
                   (SELECT string_agg(md5((i * 32 + j)::text), ' ') FROM generate_series(1, 32) j)
            FROM generate_series(1, 24000) i;
            SET max_parallel_maintenance_workers = 0;
            SET maintenance_work_mem = '16MB';
            CREATE INDEX stats_cell_merge_idx ON stats_cell_merge USING paradedb (id, tenant_id, name)
                WITH (key_field = 'id', partition_by = 'tenant_id', target_segment_count = 2);
            "#,
        )
        .unwrap();
        let indexrel = open_index("stats_cell_merge_idx");
        let (field, stats) = segment_stats(&indexrel, "tenant_id");
        assert_eq!(stats.len(), 2, "each cell merges down to one segment");
        let mut lowers = Vec::new();
        for segment in &stats {
            let bounds = segment
                .logical(field)
                .unwrap()
                .expect("the merge keeps the box");
            let empirical = segment.empirical(field).unwrap().unwrap();
            if let Bound::Included(lower) = &bounds.lower {
                assert!(!below(&empirical.min, lower));
            }
            if let Bound::Excluded(upper) = &bounds.upper {
                assert!(below(&empirical.max, upper));
            }
            lowers.push(bounds.lower);
        }
        assert!(lowers.contains(&Bound::Unbounded));
        assert_eq!(
            persisted_split_points(&indexrel, "tenant_id")
                .unwrap()
                .map(|p| p.len()),
            Some(1)
        );
    }

    /// An insert after the build lands in a mutable segment, which carries no statistics at all.
    /// The grid is gone, so the planner samples again, and every partition keeps the new segment.
    #[pg_test]
    fn insert_after_build_drops_the_persisted_grid() {
        Spi::run(
            r#"
            CREATE TABLE stats_grow (id BIGSERIAL PRIMARY KEY, tenant_id BIGINT, name TEXT);
            INSERT INTO stats_grow (tenant_id, name)
            SELECT (i * 7919) % 100, 'lorem ipsum ' || i || ' ' || repeat('padding word here ', 50)
            FROM generate_series(1, 20000) i;
            SET max_parallel_maintenance_workers = 0;
            CREATE INDEX stats_grow_idx ON stats_grow USING paradedb (id, tenant_id, name)
                WITH (key_field = 'id', partition_by = 'tenant_id', target_segment_count = 4);
            "#,
        )
        .unwrap();
        let indexrel = open_index("stats_grow_idx");
        let split_points = persisted_split_points(&indexrel, "tenant_id")
            .unwrap()
            .expect("fresh build");
        assert_eq!(split_points.len(), 3);

        Spi::run("INSERT INTO stats_grow (tenant_id, name) VALUES (42, 'late row');").unwrap();
        assert!(
            persisted_split_points(&indexrel, "tenant_id")
                .unwrap()
                .is_none()
        );

        let reader = SearchIndexReader::open(
            &indexrel,
            SearchQueryInput::All,
            false,
            MvccSatisfies::Snapshot,
        )
        .unwrap();
        assert_eq!(
            reader.segment_ids().len(),
            5,
            "four cells plus the new segment"
        );
        let boundaries = RangePartitioning {
            partition_by: FieldName::from("tenant_id"),
            split_points,
        };
        let mut everywhere: Option<Vec<SegmentId>> = None;
        for partition in 0..4 {
            let chosen = segments_for_partition(&reader, &boundaries, partition);
            assert_eq!(
                chosen.len(),
                2,
                "its cell and the unboxed segment: {chosen:?}"
            );
            everywhere = Some(match everywhere {
                None => chosen,
                Some(prev) => prev.into_iter().filter(|id| chosen.contains(id)).collect(),
            });
        }
        assert_eq!(
            everywhere.unwrap().len(),
            1,
            "the unboxed segment survives every partition"
        );
    }

    /// Without a mutable segment, a late insert lands in an immutable segment that carries
    /// empirical statistics but no box. The grid is gone, yet the segment still prunes on its own
    /// range, and a merge that takes it in keeps the statistics and drops the box.
    #[pg_test]
    fn unboxed_segment_prunes_on_empirical_stats_and_merges_without_a_box() {
        Spi::run(
            r#"
            CREATE TABLE stats_unboxed (id BIGSERIAL PRIMARY KEY, tenant_id BIGINT, name TEXT);
            INSERT INTO stats_unboxed (tenant_id, name)
            SELECT (i * 7919) % 100, 'lorem ipsum ' || i || ' ' || repeat('padding word here ', 50)
            FROM generate_series(1, 20000) i;
            SET max_parallel_maintenance_workers = 0;
            CREATE INDEX stats_unboxed_idx ON stats_unboxed USING paradedb (id, tenant_id, name)
                WITH (key_field = 'id', partition_by = 'tenant_id', target_segment_count = 4,
                      mutable_segment_rows = 0);
            "#,
        )
        .unwrap();
        let indexrel = open_index("stats_unboxed_idx");
        let split_points = persisted_split_points(&indexrel, "tenant_id")
            .unwrap()
            .expect("fresh build");
        assert_eq!(split_points.len(), 3);

        Spi::run(
            r#"
            INSERT INTO stats_unboxed (tenant_id, name)
            SELECT 42, 'lorem ipsum ' || i || ' ' || repeat('padding word here ', 50)
            FROM generate_series(1, 5000) i;
            "#,
        )
        .unwrap();
        assert!(
            persisted_split_points(&indexrel, "tenant_id")
                .unwrap()
                .is_none()
        );
        let reader = SearchIndexReader::open(
            &indexrel,
            SearchQueryInput::All,
            false,
            MvccSatisfies::Snapshot,
        )
        .unwrap();
        assert_eq!(reader.segment_ids().len(), 5);
        let boundaries = RangePartitioning {
            partition_by: FieldName::from("tenant_id"),
            split_points,
        };
        let late_row = EmpiricalStats {
            min: PdbOwnedValue::I64(42),
            max: PdbOwnedValue::I64(42),
            nullable: false,
        };
        for partition in 0..4 {
            let (lower, upper) = boundaries.partition_range(partition).unwrap();
            let expected = if late_row.intersects(&lower, &upper) {
                2
            } else {
                1
            };
            let chosen = segments_for_partition(&reader, &boundaries, partition);
            assert_eq!(chosen.len(), expected, "partition {partition}: {chosen:?}");
        }
        // `ALTER INDEX` refuses a relation this transaction still holds open.
        drop(reader);
        drop(indexrel);

        // A layer takes segments no larger than itself and closes a candidate once it fills the
        // layer by a third over. Five near-equal segments fill 3.4 layers only with the fifth one
        // in, so one candidate takes them all. Background layers would hand the merge to a worker
        // that cannot see this transaction's segments.
        let largest: i64 = Spi::get_one(
            "SELECT max(byte_size)::bigint FROM paradedb.index_info('stats_unboxed_idx');",
        )
        .unwrap()
        .unwrap();
        Spi::run(&format!(
            "ALTER INDEX stats_unboxed_idx SET (layer_sizes = '{}', background_layer_sizes = '0');",
            largest * 17 / 5
        ))
        .unwrap();
        // The row that triggers the merge lands in its own segment beside the merged one.
        Spi::run("INSERT INTO stats_unboxed (tenant_id, name) VALUES (43, 'later row');").unwrap();
        let indexrel = open_index("stats_unboxed_idx");
        let (field, stats) = segment_stats(&indexrel, "tenant_id");
        assert_eq!(stats.len(), 2, "the merged segment and the trigger row");
        let ranges: Vec<EmpiricalStats> = stats
            .iter()
            .map(|s| {
                assert!(
                    s.logical(field).unwrap().is_none(),
                    "a source without a box leaves the merge without one"
                );
                s.empirical(field).unwrap().unwrap()
            })
            .collect();
        assert!(
            ranges
                .iter()
                .any(|r| r.min == PdbOwnedValue::I64(0) && r.max == PdbOwnedValue::I64(99)),
            "the merge recomputes the range over every source: {ranges:?}"
        );
        assert!(
            persisted_split_points(&indexrel, "tenant_id")
                .unwrap()
                .is_none()
        );
    }

    /// A recent index keeps timestamps in an `I64` column while the partition bounds arrive as
    /// `Date`, so the empirical statistics must lift to `Date` before they can prune.
    #[pg_test]
    fn empirical_stats_prune_on_a_timestamp_key() {
        Spi::run(
            r#"
            CREATE TABLE stats_ts (id BIGSERIAL PRIMARY KEY, created_at TIMESTAMP, name TEXT);
            INSERT INTO stats_ts (created_at, name)
            SELECT TIMESTAMP '2024-01-01' + (i || ' minutes')::interval,
                   'row ' || i || ' ' || repeat('padding word here ', 50)
            FROM generate_series(1, 20000) i;
            ANALYZE stats_ts;
            SET max_parallel_maintenance_workers = 0;
            CREATE INDEX stats_ts_idx ON stats_ts USING paradedb (id, created_at, name)
                WITH (key_field = 'id', target_segment_count = 4);
            "#,
        )
        .unwrap();
        let indexrel = open_index("stats_ts_idx");
        let (field, stats) = segment_stats(&indexrel, "created_at");
        assert!(stats.len() > 1, "need several segments to prune between");
        let ranges: Vec<EmpiricalStats> = stats
            .iter()
            .map(|s| {
                let raw = s.empirical(field).unwrap().unwrap();
                assert!(
                    matches!(raw.min, PdbOwnedValue::I64(_)),
                    "stored as the column's raw micros"
                );
                raw.into_dates().unwrap()
            })
            .collect();
        let mut mins: Vec<PdbOwnedValue> = ranges.iter().map(|r| r.min.clone()).collect();
        mins.sort_by(PdbOwnedValue::total_cmp);

        let reader = SearchIndexReader::open(
            &indexrel,
            SearchQueryInput::All,
            false,
            MvccSatisfies::Snapshot,
        )
        .unwrap();
        let boundaries = RangePartitioning {
            partition_by: FieldName::from("created_at"),
            split_points: vec![mins[1].clone(), mins[mins.len() - 1].clone()],
        };
        let mut pruned_somewhere = false;
        for partition in 0..3 {
            let (lower, upper) = boundaries.partition_range(partition).unwrap();
            let expected = ranges
                .iter()
                .filter(|r| r.intersects(&lower, &upper))
                .count();
            let chosen = segments_for_partition(&reader, &boundaries, partition);
            assert_eq!(chosen.len(), expected, "partition {partition}");
            pruned_somewhere |= chosen.len() < stats.len();
        }
        assert!(pruned_somewhere);
    }

    fn text_partitioned_index(table: &str, index: &str, normalizer: &str) {
        Spi::run(&format!(
            r#"
            CREATE TABLE {table} (id BIGSERIAL PRIMARY KEY, name TEXT, about TEXT);
            INSERT INTO {table} (name, about)
            SELECT CASE WHEN i % 3 = 0 THEN 'Zed' || i WHEN i % 3 = 1 THEN 'alice' || i ELSE 'Bob' || i END,
                   repeat('padding word here ', 50)
            FROM generate_series(1, 20000) i;
            SET max_parallel_maintenance_workers = 0;
            CREATE INDEX {index} ON {table} USING paradedb (id, name)
                WITH (key_field = 'id', partition_by = 'name', target_segment_count = 4,
                      text_fields = '{{"name": {{"tokenizer": {{"type": "keyword"}}, "fast": true, "normalizer": "{normalizer}"}}}}');
            "#
        ))
        .unwrap();
    }

    /// Routing compares raw text, but the partition query reads the fast column. A normalizer
    /// reorders that column, so such a field gets no box.
    #[pg_test]
    fn normalized_text_field_gets_no_logical_bounds() {
        text_partitioned_index("stats_text_lower", "stats_text_lower_idx", "lowercase");
        let indexrel = open_index("stats_text_lower_idx");
        let (field, stats) = segment_stats(&indexrel, "name");
        assert!(stats.len() > 1);
        for segment in &stats {
            assert!(segment.logical(field).unwrap().is_none());
            let empirical = segment.empirical(field).unwrap().unwrap();
            assert!(matches!(empirical.min, PdbOwnedValue::Str(_)));
        }
        assert!(persisted_split_points(&indexrel, "name").unwrap().is_none());
    }

    /// With the raw normalizer the fast column keeps the routing order, so the box holds.
    #[pg_test]
    fn raw_text_field_gets_logical_bounds() {
        text_partitioned_index("stats_text_raw", "stats_text_raw_idx", "raw");
        let indexrel = open_index("stats_text_raw_idx");
        let (field, stats) = segment_stats(&indexrel, "name");
        assert!(stats.len() > 1);
        for segment in &stats {
            let bounds = segment
                .logical(field)
                .unwrap()
                .expect("raw text keeps its box");
            let empirical = segment.empirical(field).unwrap().unwrap();
            if let Bound::Included(lower) = &bounds.lower {
                assert!(!below(&empirical.min, lower));
            }
            if let Bound::Excluded(upper) = &bounds.upper {
                assert!(below(&empirical.max, upper));
            }
        }
        assert!(persisted_split_points(&indexrel, "name").unwrap().is_some());
    }

    #[test]
    fn bounds_union_and_intersection() {
        let i = |v: i64| PdbOwnedValue::I64(v);
        let a = LogicalBounds {
            lower: Bound::Included(i(10)),
            upper: Bound::Excluded(i(20)),
        };
        let b = LogicalBounds {
            lower: Bound::Included(i(20)),
            upper: Bound::Excluded(i(30)),
        };
        assert_eq!(
            a.union(&b),
            LogicalBounds {
                lower: Bound::Included(i(10)),
                upper: Bound::Excluded(i(30)),
            }
        );
        assert_eq!(
            a.union(&LogicalBounds {
                lower: Bound::Unbounded,
                upper: Bound::Excluded(i(10)),
            })
            .lower,
            Bound::Unbounded
        );
        // Half-open boxes that touch at 20 share no value.
        assert!(!a.intersects(&Bound::Included(i(20)), &Bound::Unbounded));
        assert!(a.intersects(&Bound::Included(i(19)), &Bound::Unbounded));
        assert!(a.intersects(&Bound::Unbounded, &Bound::Excluded(i(11))));
        assert!(!a.intersects(&Bound::Unbounded, &Bound::Excluded(i(10))));
        // A bound of another kind proves nothing.
        assert!(a.intersects(
            &Bound::Included(PdbOwnedValue::Str("x".into())),
            &Bound::Unbounded
        ));

        let e = EmpiricalStats {
            min: i(5),
            max: i(9),
            nullable: false,
        };
        assert!(e.intersects(&Bound::Included(i(9)), &Bound::Unbounded));
        assert!(!e.intersects(&Bound::Excluded(i(9)), &Bound::Unbounded));
        assert!(!e.intersects(&Bound::Unbounded, &Bound::Excluded(i(5))));
        assert!(e.intersects(&Bound::Unbounded, &Bound::Included(i(5))));
    }
}
