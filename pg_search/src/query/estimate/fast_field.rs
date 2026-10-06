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

use super::{Context, Estimate, combine};
use crate::index::stats::distribution::Distribution;
use std::ops::Bound;
use tantivy::SegmentReader;
use tantivy::columnar::{ColumnType, MonotonicallyMappableToU64};
use tantivy::query::Occur;
use tantivy::schema::{Term, Type};

#[derive(Clone, Copy, PartialEq)]
enum Value {
    Integer(i128),
    Float(f64),
    Encoded(u128),
}

impl Value {
    fn from_term(term: &Term) -> Option<Self> {
        let bytes = term.value();
        let bytes = bytes.as_json_value_bytes().unwrap_or_else(|| bytes.clone());
        Some(match bytes.typ() {
            Type::I64 => Self::Integer(bytes.as_i64()? as i128),
            Type::U64 => Self::Integer(bytes.as_u64()? as i128),
            Type::F64 => Self::Float(bytes.as_f64()?),
            Type::Bool => Self::Encoded(bytes.as_bool()?.to_u64() as u128),
            Type::Date => Self::Encoded(bytes.as_date()?.to_u64() as u128),
            Type::IpAddr => Self::Encoded(u128::from(bytes.as_ip_addr()?)),
            _ => return None,
        })
    }
    fn compare(self, encoded: u128, typ: ColumnType) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        match self {
            Self::Encoded(value) => encoded.cmp(&value),
            Self::Integer(value) => match typ {
                ColumnType::I64 => (i64::from_u64(encoded as u64) as i128).cmp(&value),
                ColumnType::U64 => (encoded as i128).cmp(&value),
                ColumnType::F64 => f64::from_u64(encoded as u64).total_cmp(&(value as f64)),
                _ => Ordering::Less,
            },
            Self::Float(value) => {
                let actual = match typ {
                    ColumnType::I64 => i64::from_u64(encoded as u64) as f64,
                    ColumnType::U64 => encoded as f64,
                    ColumnType::F64 => f64::from_u64(encoded as u64),
                    _ => return Ordering::Less,
                };
                actual.total_cmp(&value)
            }
        }
    }
}

fn range_fraction(
    stats: &Distribution,
    typ: ColumnType,
    lower: Bound<Value>,
    upper: Bound<Value>,
    num_docs: u32,
) -> f64 {
    use std::cmp::Ordering;
    let contains = |value| {
        let above = match lower {
            Bound::Unbounded => true,
            Bound::Included(bound) => bound.compare(value, typ) != Ordering::Less,
            Bound::Excluded(bound) => bound.compare(value, typ) == Ordering::Greater,
        };
        let below = match upper {
            Bound::Unbounded => true,
            Bound::Included(bound) => bound.compare(value, typ) != Ordering::Greater,
            Bound::Excluded(bound) => bound.compare(value, typ) == Ordering::Less,
        };
        above && below
    };
    if stats.sample.is_empty() {
        return 0.0;
    }
    let total = stats.sample.len() as f64;
    if stats.sample.len() == num_docs as usize || stats.sample.iter().any(|row| row.len() > 1) {
        return stats
            .sample
            .iter()
            .filter(|row| row.iter().any(|&value| contains(value)))
            .count() as f64
            / total;
    }
    let common_mass = stats
        .common
        .iter()
        .map(|entry| entry.documents as f64 / total)
        .sum::<f64>();
    let remaining = (stats.present_docs as f64 / num_docs as f64 - common_mass).max(0.0);
    if let (Bound::Included(a), Bound::Included(b)) = (lower, upper)
        && a == b
    {
        if let Some(entry) = stats.common.iter().find(|entry| contains(entry.value)) {
            return entry.documents as f64 / total;
        }
        return remaining / (stats.distinct - stats.common.len() as f64).max(1.0);
    }
    let common = stats
        .common
        .iter()
        .filter(|entry| contains(entry.value))
        .map(|entry| entry.documents as f64 / total)
        .sum::<f64>();
    let histogram = if stats.histogram.is_empty() {
        0.0
    } else {
        stats
            .histogram
            .iter()
            .filter(|&&value| contains(value))
            .count() as f64
            / stats.histogram.len() as f64
    };
    (common + remaining * histogram).clamp(0.0, 1.0)
}

fn column_name(term: &Term, reader: &SegmentReader) -> String {
    let name = reader.schema().get_field_name(term.field());
    match term.get_json_path() {
        Some(path) => format!("{name}\u{1}{path}"),
        None => name.to_owned(),
    }
}

fn term_bound(bound: &Bound<Term>) -> Option<Bound<Value>> {
    Some(match bound {
        Bound::Unbounded => Bound::Unbounded,
        Bound::Included(term) => Bound::Included(Value::from_term(term)?),
        Bound::Excluded(term) => Bound::Excluded(Value::from_term(term)?),
    })
}

fn payload_bound(bound: &Bound<Term>) -> Bound<Vec<u8>> {
    let payload = |term: &Term| {
        let bytes = term.value();
        let bytes = bytes.as_json_value_bytes().unwrap_or_else(|| bytes.clone());
        bytes.as_serialized()[1..].to_vec()
    };
    bound.as_ref().map(payload)
}

/// Arrays use sampled documents; scalar ranges combine common values and histogram buckets.
pub(super) fn range(
    bounds: (&Bound<Term>, &Bound<Term>),
    ctx: &Context<'_>,
) -> tantivy::Result<Option<Estimate>> {
    let (lower, upper) = bounds;
    let term = match lower {
        Bound::Included(t) | Bound::Excluded(t) => t,
        Bound::Unbounded => match upper {
            Bound::Included(t) | Bound::Excluded(t) => t,
            Bound::Unbounded => {
                return Ok(Some(Estimate::count(
                    ctx.reader.max_doc(),
                    ctx.reader.max_doc() as u64,
                    ctx.reader,
                )));
            }
        },
    };
    let name = column_name(term, ctx.reader);
    let Some((_, manifest)) = ctx.stats() else {
        return Ok(None);
    };
    let query_type = term.value().json_path_type().unwrap_or(term.typ());
    let mut estimates = Vec::new();
    for (ordinal, (column, code)) in manifest.columns.iter().enumerate() {
        if column != &name {
            continue;
        }
        let Ok(typ) = ColumnType::try_from_code(*code) else {
            return Ok(None);
        };
        let compatible = match query_type {
            Type::I64 | Type::U64 | Type::F64 => {
                matches!(typ, ColumnType::I64 | ColumnType::U64 | ColumnType::F64)
            }
            Type::Str => typ == ColumnType::Str,
            Type::Bytes => typ == ColumnType::Bytes,
            Type::Bool => typ == ColumnType::Bool,
            Type::Date => typ == ColumnType::DateTime,
            Type::IpAddr => typ == ColumnType::IpAddr,
            _ => false,
        };
        if !compatible {
            continue;
        }
        let Some(stats) = ctx.distribution(ordinal) else {
            return Ok(None);
        };
        let (lower, upper) = if matches!(typ, ColumnType::Str | ColumnType::Bytes) {
            let map = |bound: &Bound<Term>, is_lower| match payload_bound(bound) {
                Bound::Unbounded => Bound::Unbounded,
                Bound::Included(value) | Bound::Excluded(value) => {
                    let inclusive = matches!(bound, Bound::Included(_));
                    let pos = stats.dictionary.partition_point(|(_, bytes)| {
                        bytes < &value || (bytes == &value && (inclusive != is_lower))
                    });
                    if is_lower {
                        stats
                            .dictionary
                            .get(pos)
                            .map_or(Bound::Excluded(Value::Encoded(u128::MAX)), |(ord, _)| {
                                Bound::Included(Value::Encoded(*ord))
                            })
                    } else {
                        pos.checked_sub(1)
                            .map_or(Bound::Excluded(Value::Encoded(0)), |pos| {
                                Bound::Included(Value::Encoded(stats.dictionary[pos].0))
                            })
                    }
                }
            };
            if let (Bound::Included(left), Bound::Included(right)) =
                (payload_bound(lower), payload_bound(upper))
                && left == right
                && stats
                    .dictionary
                    .binary_search_by(|(_, bytes)| bytes.cmp(&left))
                    .is_err()
            {
                if stats.sample.len() == ctx.reader.max_doc() as usize {
                    estimates.push((
                        Occur::Should,
                        Estimate::count(0, ctx.reader.max_doc() as u64, ctx.reader),
                    ));
                    continue;
                }
                let remaining = stats.present_docs as f64 / ctx.reader.max_doc() as f64
                    - stats
                        .common
                        .iter()
                        .map(|entry| entry.documents as f64 / stats.sample.len().max(1) as f64)
                        .sum::<f64>();
                estimates.push((
                    Occur::Should,
                    Estimate {
                        fraction: remaining.max(0.0)
                            / (stats.distinct - stats.common.len() as f64).max(1.0),
                        work: ctx.reader.max_doc() as u64,
                    },
                ));
                continue;
            }
            (map(lower, true), map(upper, false))
        } else {
            let (Some(lower), Some(upper)) = (term_bound(lower), term_bound(upper)) else {
                return Ok(None);
            };
            (lower, upper)
        };
        let fraction = range_fraction(&stats, typ, lower, upper, ctx.reader.max_doc());
        estimates.push((
            Occur::Should,
            Estimate {
                fraction,
                work: ctx.reader.max_doc() as u64,
            },
        ));
    }
    Ok(Some(combine(estimates, 1, ctx.reader.max_doc())))
}
