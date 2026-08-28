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
//! Every immutable segment carries two facts about each fast field: the empirical `min` and
//! `max` of the values it holds, and, after a partitioned build, the box the build assigned it.
//! A missing file or entry means "unknown", and a consumer must keep the segment.
//!
//! `plugin` writes and merges the file, `pruning` reads it for range partitioning. The types
//! and the reader here are shared by both.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::io;
use std::ops::Bound;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tantivy::directory::error::OpenReadError;
use tantivy::directory::{CompositeFile, FileSlice};
use tantivy::index::{Segment, SegmentReader};
use tantivy::schema::Field;

use crate::postgres::datetime::PostgresDateTime;
use crate::postgres::pdb_owned_value::{exact_scalar_wire, PdbOwnedValue};
use crate::postgres::storage::block::STATS_EXT;

mod plugin;
mod pruning;
#[cfg(any(test, feature = "pg_test"))]
mod tests;

use plugin::stats_component;
pub(crate) use plugin::{register, StatsWriter};
pub(crate) use pruning::{persisted_split_points, segments_for_partition};

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

/// The half-open box a partitioned build assigned to a segment's partition on one field.
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
            if let Bound::Included(v) | Bound::Excluded(v) = own {
                if !(bound_comparable(lower, v) && bound_comparable(upper, v)) {
                    return true;
                }
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

    /// The `.stats` of `segment`, through its directory. `None` when the segment was written
    /// without the component.
    pub(crate) fn of_segment(segment: &Segment) -> io::Result<Option<Self>> {
        Self::from_component(segment.open_read(stats_component()))
    }

    /// The `.stats` of an open segment reader. `None` when the segment was written without the
    /// component.
    pub(crate) fn of_reader(reader: &SegmentReader) -> io::Result<Option<Self>> {
        Self::from_component(reader.open_read(stats_component()))
    }

    fn from_component(opened: Result<FileSlice, OpenReadError>) -> io::Result<Option<Self>> {
        match opened {
            Ok(slice) => Self::open(slice).map(Some),
            Err(OpenReadError::FileDoesNotExist(_)) => Ok(None),
            Err(e) => Err(io::Error::other(e)),
        }
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
