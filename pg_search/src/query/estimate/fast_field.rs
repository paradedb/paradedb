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

use super::{Context, Estimate};
use crate::index::stats::distribution::Distribution;
use std::ops::{Bound, RangeBounds};
use tantivy::SegmentReader;
use tantivy::columnar::{ColumnType, MonotonicallyMappableToU64};
use tantivy::query::Occur;
use tantivy::schema::{Term, Type};

fn range_fraction(
    stats: &Distribution,
    lower: Bound<u128>,
    upper: Bound<u128>,
    num_docs: u32,
) -> f64 {
    let contains = |value| (lower, upper).contains(&value);
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

fn term_bound(bound: &Bound<Term>, typ: ColumnType) -> Option<Bound<u128>> {
    let encode = |term: &Term| {
        let value = term.value();
        let value = value.as_json_value_bytes().unwrap_or_else(|| value.clone());
        if value.typ().numerical_type() != typ.numerical_type() {
            let number = value
                .as_f64()
                .or_else(|| value.as_i64().map(|v| v as f64))
                .or_else(|| value.as_u64().map(|v| v as f64))?;
            return Some(u128::from(match typ {
                ColumnType::I64 => (number as i64).to_u64(),
                ColumnType::U64 => number as u64,
                ColumnType::F64 => number.to_u64(),
                _ => return None,
            }));
        }
        if typ == ColumnType::IpAddr {
            Some(u128::from(value.as_ip_addr()?))
        } else {
            value.as_u64_lenient().map(u128::from)
        }
    };
    Some(match bound {
        Bound::Unbounded => Bound::Unbounded,
        Bound::Included(term) => Bound::Included(encode(term)?),
        Bound::Excluded(term) => Bound::Excluded(encode(term)?),
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
    let query_type = term.value().json_path_type().unwrap_or(term.typ());
    let mut estimates = Vec::new();
    for (ordinal, (column, code)) in ctx.manifest.columns.iter().enumerate() {
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
                            .map_or(Bound::Excluded(u128::MAX), |(ord, _)| Bound::Included(*ord))
                    } else {
                        pos.checked_sub(1).map_or(Bound::Excluded(0), |pos| {
                            Bound::Included(stats.dictionary[pos].0)
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
            let (Some(lower), Some(upper)) = (term_bound(lower, typ), term_bound(upper, typ))
            else {
                return Ok(None);
            };
            (lower, upper)
        };
        let fraction = range_fraction(&stats, lower, upper, ctx.reader.max_doc());
        estimates.push((
            Occur::Should,
            Estimate {
                fraction,
                work: ctx.reader.max_doc() as u64,
            },
        ));
    }
    Ok(Some(Estimate::combine(estimates, 1, ctx.reader.max_doc())))
}
