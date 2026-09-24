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

//! Top-K aggregates for DataFusion. `topk_as_agg` keeps the K best payload rows
//! under an ORDER BY; `distinct_topk_as_agg` does the same over distinct rows, so
//! a `SELECT DISTINCT … ORDER BY … LIMIT k` runs as one aggregate function that
//! can sit beside other aggregates in a single aggregate node.
//!
//! Both share one accumulator: a bounded ordered map keyed on the row-format
//! encoding of the sort keys followed by a suffix. In distinct mode the suffix is
//! the remaining distinct columns, so equal rows collide; the first row seen holds
//! the group, carrying the element-wise minimum of its ctids, which is what the
//! `min(ctid)` of a DISTINCT group-by produces. Otherwise the suffix is an
//! arrival counter, so nothing collides. Every ORDER BY key is inside the distinct
//! key (Postgres requires it in the select list), so map order is ORDER BY order
//! in both modes and the worst entry is always `last`.

use arrow_array::cast::AsArray;
use arrow_array::types::UInt64Type;
use arrow_array::{Array, ArrayRef, RecordBatch, StructArray, UInt32Array, UInt64Array};
use arrow_schema::{DataType, Field, FieldRef, Fields, Schema, SchemaRef, SortOptions};
use arrow_select::take::take;
use datafusion::arrow::row::{OwnedRow, RowConverter, SortField};
use datafusion::common::utils::SingleRowListArrayBuilder;
use datafusion::error::{DataFusionError, Result};
use datafusion::logical_expr::expr::AggregateFunction;
use datafusion::logical_expr::function::{AccumulatorArgs, StateFieldsArgs};
use datafusion::logical_expr::utils::AggregateOrderSensitivity;
use datafusion::logical_expr::{
    Accumulator, AggregateUDF, AggregateUDFImpl, Signature, SortExpr, Volatility,
};
use datafusion::prelude::{Expr, lit};
use datafusion::scalar::ScalarValue;
use std::collections::BTreeMap;
use std::collections::btree_map::Entry as MapEntry;
use std::sync::{Arc, LazyLock};

use super::{literal_arg, reject_distinct};

pub const TOPK_AS_AGG_NAME: &str = "topk_as_agg";
pub const DISTINCT_TOPK_AS_AGG_NAME: &str = "distinct_topk_as_agg";
pub const TOPK_AGG_ROWS_COL_NAME: &str = "__topk";

static TOPK_AS_AGG: LazyLock<Arc<AggregateUDF>> =
    LazyLock::new(|| Arc::new(AggregateUDF::from(TopKAgg::new(false))));
static DISTINCT_TOPK_AS_AGG: LazyLock<Arc<AggregateUDF>> =
    LazyLock::new(|| Arc::new(AggregateUDF::from(TopKAgg::new(true))));

pub fn topk_as_agg_udaf() -> Arc<AggregateUDF> {
    Arc::clone(&TOPK_AS_AGG)
}

pub fn distinct_topk_as_agg_udaf() -> Arc<AggregateUDF> {
    Arc::clone(&DISTINCT_TOPK_AS_AGG)
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct TopKAgg {
    signature: Signature,
    /// `distinct_topk_as_agg` takes two more trailing literals after `k`: the
    /// distinct-key positions and the ctid positions within the payload.
    distinct: bool,
}

impl TopKAgg {
    fn new(distinct: bool) -> Self {
        Self {
            signature: Signature::variadic_any(Volatility::Immutable),
            distinct,
        }
    }

    /// Literal arguments after the payload.
    fn trailing(&self) -> usize {
        if self.distinct { 3 } else { 1 }
    }
}

impl AggregateUDFImpl for TopKAgg {
    fn name(&self) -> &str {
        if self.distinct {
            DISTINCT_TOPK_AS_AGG_NAME
        } else {
            TOPK_AS_AGG_NAME
        }
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    /// Types only, so every payload field is assumed nullable. The planner
    /// types the call through `return_field`, which keeps nullability.
    fn return_type(&self, arg_types: &[DataType]) -> Result<DataType> {
        let fields: Vec<FieldRef> = arg_types
            .iter()
            .map(|t| Arc::new(Field::new("", t.clone(), true)))
            .collect();
        Ok(list_of_rows(payload_fields(&fields, self.trailing())?))
    }

    fn return_field(&self, arg_fields: &[FieldRef]) -> Result<FieldRef> {
        let rows = list_of_rows(payload_fields(arg_fields, self.trailing())?);
        Ok(Arc::new(Field::new(self.name(), rows, true)))
    }

    /// The state is the same `List<Struct>` as the result: `state()` is `evaluate()`.
    fn state_fields(&self, args: StateFieldsArgs) -> Result<Vec<FieldRef>> {
        Ok(vec![Arc::new(Field::new(
            format!("{}[rows]", args.name),
            args.return_type().clone(),
            true,
        ))])
    }

    /// `Beneficial`, not `Insensitive`: both keep the planner from putting a
    /// SortExec under the aggregate (`get_aggregate_expr_req` imposes no input
    /// ordering for either), but `AggregateFunctionExpr::order_bys()` returns
    /// nothing for an insensitive aggregate, and that accessor is what the proto
    /// serializer reads. An insensitive Top-K therefore reaches MPP workers
    /// without its ORDER BY and fails there. `Beneficial` keeps the ordering on
    /// every path, including `create_accumulator`.
    fn order_sensitivity(&self) -> AggregateOrderSensitivity {
        AggregateOrderSensitivity::Beneficial
    }

    /// Required for a `Beneficial` aggregate. Whether the input happens to be
    /// ordered changes nothing here; the map orders regardless.
    fn with_beneficial_ordering(
        self: Arc<Self>,
        _beneficial_ordering: bool,
    ) -> Result<Option<Arc<dyn AggregateUDFImpl>>> {
        Ok(Some(self))
    }

    fn accumulator(&self, acc_args: AccumulatorArgs) -> Result<Box<dyn Accumulator>> {
        let name = self.name();
        reject_distinct(&acc_args, name)?;
        let payload = payload_fields(acc_args.expr_fields, self.trailing())?;
        let n = payload.len();

        let k = match literal_arg(&acc_args, n, name, "k")? {
            ScalarValue::UInt64(Some(k)) if *k > 0 => *k as usize,
            other => {
                return Err(DataFusionError::Internal(format!(
                    "{name} k must be a positive UInt64 literal, got {other}"
                )));
            }
        };

        // The accumulator's batches must carry the declared row type exactly, so
        // take the payload schema from the return field rather than the argument
        // fields, whose names differ.
        let schema = row_schema(&acc_args.return_field)?;

        // Only the payload columns reach update_batch, so each sort key must be one
        // of them; rebase the ORDER BY onto payload positions.
        let sort: Vec<(usize, SortOptions)> = acc_args
            .order_bys
            .iter()
            .map(|s| {
                acc_args.exprs[..n]
                    .iter()
                    .position(|e| e.as_ref() == s.expr.as_ref())
                    .map(|i| (i, s.options))
                    .ok_or_else(|| {
                        DataFusionError::Internal(format!(
                            "{name} ORDER BY {} is not one of its arguments",
                            s.expr
                        ))
                    })
            })
            .collect::<Result<_>>()?;
        if sort.is_empty() {
            return Err(DataFusionError::Internal(format!(
                "{name} requires an ORDER BY"
            )));
        }

        let (suffix, ctid_positions) = if self.distinct {
            let keys = positions_arg(&acc_args, n + 1, name, "distinct key positions", n)?;
            let ctids = positions_arg(&acc_args, n + 2, name, "ctid positions", n)?;
            if let Some((p, _)) = sort.iter().find(|(p, _)| !keys.contains(p)) {
                return Err(DataFusionError::Internal(format!(
                    "{name} ORDER BY position {p} is not in the distinct key"
                )));
            }
            if let Some(p) = ctids
                .iter()
                .find(|&&p| schema.field(p).data_type() != &DataType::UInt64)
            {
                return Err(DataFusionError::Internal(format!(
                    "{name} ctid position {p} is not a UInt64 column"
                )));
            }
            let rest = keys
                .iter()
                .copied()
                .filter(|p| !sort.iter().any(|(s, _)| s == p))
                .collect();
            (Suffix::Distinct { positions: rest }, ctids)
        } else {
            (Suffix::Arrival { next: 0 }, Vec::new())
        };

        Ok(Box::new(FusedTopK::new(
            schema,
            sort,
            suffix,
            ctid_positions,
            k,
        )?))
    }
}

/// Every argument but the `trailing` literals.
fn payload_fields(arg_fields: &[FieldRef], trailing: usize) -> Result<&[FieldRef]> {
    match arg_fields.len().checked_sub(trailing) {
        Some(n) if n > 0 => Ok(&arg_fields[..n]),
        _ => Err(DataFusionError::Internal(format!(
            "a Top-K aggregate takes at least one payload column and {trailing} trailing literal(s)"
        ))),
    }
}

/// A `List<UInt64>` literal argument naming payload positions.
fn positions_arg(
    args: &AccumulatorArgs,
    index: usize,
    name: &str,
    what: &str,
    payload_len: usize,
) -> Result<Vec<usize>> {
    let ScalarValue::List(list) = literal_arg(args, index, name, what)? else {
        return Err(DataFusionError::Internal(format!(
            "{name} {what} must be a List<UInt64> literal"
        )));
    };
    let values = list.value(0);
    let values = values.as_primitive_opt::<UInt64Type>().ok_or_else(|| {
        DataFusionError::Internal(format!("{name} {what} must be a List<UInt64> literal"))
    })?;
    if values.null_count() > 0 {
        return Err(DataFusionError::Internal(format!(
            "{name} {what} must not contain NULL"
        )));
    }
    let positions: Vec<usize> = values.values().iter().map(|&v| v as usize).collect();
    if let Some(p) = positions.iter().find(|&&p| p >= payload_len) {
        return Err(DataFusionError::Internal(format!(
            "{name} {what} position {p} is outside the payload"
        )));
    }
    Ok(positions)
}

fn positions_lit(positions: &[usize]) -> Expr {
    let values: Vec<ScalarValue> = positions
        .iter()
        .map(|&p| ScalarValue::UInt64(Some(p as u64)))
        .collect();
    lit(ScalarValue::List(ScalarValue::new_list_nullable(
        &values,
        &DataType::UInt64,
    )))
}

/// `List<Struct<c0, c1, …>>` over the payload's types and nullability, with the
/// item field spelled the way `SingleRowListArrayBuilder::build_list_scalar`
/// spells it in `evaluate`.
///
/// The struct fields are named by position. Argument names are the bare column
/// names, which collide across relations and differ between the logical and
/// physical planners for expressions; positions are unique and identical at
/// both layers, so callers read the rows back with `get_field(…, "c{i}")`.
fn list_of_rows(payload: &[FieldRef]) -> DataType {
    let fields: Vec<FieldRef> = payload
        .iter()
        .enumerate()
        .map(|(i, f)| {
            Arc::new(Field::new(
                format!("c{i}"),
                f.data_type().clone(),
                f.is_nullable(),
            ))
        })
        .collect();
    let row = DataType::Struct(Fields::from(fields));
    DataType::List(Arc::new(Field::new_list_field(row, true)))
}

/// The payload schema inside a `List<Struct>` return field.
fn row_schema(return_field: &Field) -> Result<SchemaRef> {
    if let DataType::List(item) = return_field.data_type()
        && let DataType::Struct(fields) = item.data_type()
    {
        return Ok(Arc::new(Schema::new(fields.clone())));
    }
    Err(DataFusionError::Internal(format!(
        "a Top-K aggregate's return type must be a list of structs, got {}",
        return_field.data_type()
    )))
}

/// What follows the sort keys in an entry's key.
enum Suffix {
    /// The remaining distinct columns, so equal rows collide.
    Distinct { positions: Vec<usize> },
    /// An arrival counter, so nothing collides. Counters never leave the
    /// accumulator: a merging side assigns its own as it re-inserts rows.
    Arrival { next: u64 },
}

struct Entry {
    /// Length of the sort-key prefix inside the map key, so the worst entry's
    /// prefix can be compared against candidates without re-encoding it.
    prefix_len: usize,
    payload: OwnedRow,
    /// Element-wise minimum of the group's ctids, one per `ctid_positions`
    /// entry; empty in arrival mode. Written over the payload's ctid columns at
    /// drain time.
    ctids: Vec<Option<u64>>,
}

/// The K best rows under an ORDER BY, as a bounded ordered map. The map key is
/// the row-format encoding of the sort keys with their sort options, followed by
/// the suffix; row-format bytes are the concatenation of per-column encodings,
/// so a converter over the sort keys alone produces exactly the key's prefix,
/// which is what the prefilter compares.
struct FusedTopK {
    schema: SchemaRef,
    k: usize,
    sort_positions: Vec<usize>,
    suffix: Suffix,
    ctid_positions: Vec<usize>,
    /// Sort keys with their options.
    prefix: RowConverter,
    /// Sort keys followed by the suffix columns.
    key: RowConverter,
    /// A whole payload row, for storage and emit.
    payload: RowConverter,
    entries: BTreeMap<Vec<u8>, Entry>,
}

impl std::fmt::Debug for FusedTopK {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FusedTopK")
            .field("k", &self.k)
            .field("sort_positions", &self.sort_positions)
            .field("distinct", &matches!(self.suffix, Suffix::Distinct { .. }))
            .field("entries", &self.entries.len())
            .finish_non_exhaustive()
    }
}

impl FusedTopK {
    fn new(
        schema: SchemaRef,
        sort: Vec<(usize, SortOptions)>,
        suffix: Suffix,
        ctid_positions: Vec<usize>,
        k: usize,
    ) -> Result<Self> {
        let sort_fields: Vec<SortField> = sort
            .iter()
            .map(|&(p, options)| {
                SortField::new_with_options(schema.field(p).data_type().clone(), options)
            })
            .collect();
        let mut key_fields = sort_fields.clone();
        match &suffix {
            Suffix::Distinct { positions } => key_fields.extend(
                positions
                    .iter()
                    .map(|&p| SortField::new(schema.field(p).data_type().clone())),
            ),
            Suffix::Arrival { .. } => key_fields.push(SortField::new(DataType::UInt64)),
        }
        let payload_fields = schema
            .fields()
            .iter()
            .map(|f| SortField::new(f.data_type().clone()))
            .collect();
        Ok(Self {
            prefix: RowConverter::new(sort_fields)?,
            key: RowConverter::new(key_fields)?,
            payload: RowConverter::new(payload_fields)?,
            entries: BTreeMap::new(),
            sort_positions: sort.into_iter().map(|(p, _)| p).collect(),
            schema,
            suffix,
            ctid_positions,
            k,
        })
    }

    fn worst_prefix(&self) -> Option<&[u8]> {
        self.entries
            .last_key_value()
            .map(|(key, e)| &key[..e.prefix_len])
    }

    fn absorb(&mut self, columns: &[ArrayRef]) -> Result<()> {
        // Encode only the sort keys for the whole batch.
        let sort_arrays: Vec<ArrayRef> = self
            .sort_positions
            .iter()
            .map(|&p| Arc::clone(&columns[p]))
            .collect();
        let prefixes = self.prefix.convert_columns(&sort_arrays)?;

        // When full, drop every row that cannot beat the worst entry. A row of a
        // group already in the map has its group's prefix, which is at or above
        // the worst, so a duplicate is never hidden by this step.
        let full = self.entries.len() >= self.k;
        let worst = self.worst_prefix();
        let survivors: Vec<u32> = (0..columns[0].len())
            .filter(|&i| !full || worst.is_some_and(|w| prefixes.row(i).as_ref() <= w))
            .map(|i| i as u32)
            .collect();
        if survivors.is_empty() {
            return Ok(());
        }

        // Encode full keys and payloads for the survivors only.
        let indices = UInt32Array::from(survivors.clone());
        let taken: Vec<ArrayRef> = columns
            .iter()
            .map(|c| take(c.as_ref(), &indices, None))
            .collect::<std::result::Result<_, _>>()?;
        let mut key_arrays: Vec<ArrayRef> = self
            .sort_positions
            .iter()
            .map(|&p| Arc::clone(&taken[p]))
            .collect();
        match &mut self.suffix {
            Suffix::Distinct { positions } => {
                key_arrays.extend(positions.iter().map(|&p| Arc::clone(&taken[p])));
            }
            Suffix::Arrival { next } => {
                let count = survivors.len() as u64;
                key_arrays.push(Arc::new(UInt64Array::from_iter_values(
                    *next..*next + count,
                )));
                *next += count;
            }
        }
        let keys = self.key.convert_columns(&key_arrays)?;
        let payloads = self.payload.convert_columns(&taken)?;

        // Admit one row at a time. Survivors were chosen against the worst entry
        // at batch start; the map re-checks against the current one.
        for (j, &i) in survivors.iter().enumerate() {
            let key = keys.row(j).as_ref().to_vec();
            if self.entries.len() >= self.k
                && self
                    .entries
                    .last_key_value()
                    .is_some_and(|(worst, _)| key.as_slice() > worst.as_slice())
            {
                continue;
            }
            let row_ctids = |p: usize| {
                let col = taken[p].as_primitive::<UInt64Type>();
                col.is_valid(j).then(|| col.value(j))
            };
            match self.entries.entry(key) {
                MapEntry::Occupied(mut occupied) => {
                    // Distinct mode only: the same group again. The first row keeps
                    // the group; its ctids take the element-wise minimum, NULLs
                    // ignored, the way `min(ctid)` does.
                    let entry = occupied.get_mut();
                    for (slot, &p) in entry.ctids.iter_mut().zip(&self.ctid_positions) {
                        if let Some(ctid) = row_ctids(p) {
                            *slot = Some(slot.map_or(ctid, |cur| cur.min(ctid)));
                        }
                    }
                }
                MapEntry::Vacant(vacant) => {
                    vacant.insert(Entry {
                        prefix_len: prefixes.row(i as usize).as_ref().len(),
                        payload: payloads.row(j).owned(),
                        ctids: self.ctid_positions.iter().map(|&p| row_ctids(p)).collect(),
                    });
                    if self.entries.len() > self.k {
                        self.entries.pop_last();
                    }
                }
            }
        }
        Ok(())
    }

    /// The entries in map order, which is ORDER BY order, as one batch, with the
    /// ctid minima written over the ctid columns. Empties the map.
    fn drain(&mut self) -> Result<RecordBatch> {
        if self.entries.is_empty() {
            return Ok(RecordBatch::new_empty(Arc::clone(&self.schema)));
        }
        let entries: Vec<Entry> = std::mem::take(&mut self.entries).into_values().collect();
        let mut arrays = self
            .payload
            .convert_rows(entries.iter().map(|e| e.payload.row()))?;
        for (i, &p) in self.ctid_positions.iter().enumerate() {
            let ctids: Vec<Option<u64>> = entries.iter().map(|e| e.ctids[i]).collect();
            arrays[p] = Arc::new(UInt64Array::from(ctids));
        }
        Ok(RecordBatch::try_new(Arc::clone(&self.schema), arrays)?)
    }
}

impl Accumulator for FusedTopK {
    fn update_batch(&mut self, values: &[ArrayRef]) -> Result<()> {
        let n = self.schema.fields().len();
        self.absorb(&values[..n])
    }

    /// Other partials' K rows. They go through the same admission, which is what
    /// deduplicates across workers in distinct mode.
    fn merge_batch(&mut self, states: &[ArrayRef]) -> Result<()> {
        for rows in states[0].as_list::<i32>().iter().flatten() {
            if !rows.is_empty() {
                self.absorb(rows.as_struct().columns())?;
            }
        }
        Ok(())
    }

    fn evaluate(&mut self) -> Result<ScalarValue> {
        let batch = self.drain()?;
        Ok(SingleRowListArrayBuilder::new(Arc::new(StructArray::from(batch))).build_list_scalar())
    }

    fn state(&mut self) -> Result<Vec<ScalarValue>> {
        Ok(vec![self.evaluate()?])
    }

    fn size(&self) -> usize {
        size_of_val(self)
            + self
                .entries
                .iter()
                .map(|(key, e)| key.len() + e.payload.as_ref().len() + e.ctids.len() * 16 + 64)
                .sum::<usize>()
            + self.prefix.size()
            + self.key.size()
            + self.payload.size()
    }
}

/// `topk_as_agg(payload…, k) ORDER BY sort_exprs`.
///
/// The first `payload.len()` fields of each emitted row are `payload` in order.
/// Sort keys the payload does not already carry are appended after it, since
/// the accumulator can only sort on columns it receives.
pub fn topk_as_agg(payload: &[Expr], sort_exprs: Vec<SortExpr>, k: usize) -> Expr {
    let args = topk_args(payload, &sort_exprs, k);
    Expr::AggregateFunction(AggregateFunction::new_udf(
        topk_as_agg_udaf(),
        args,
        false,
        None,
        sort_exprs,
        None,
    ))
}

/// `distinct_topk_as_agg(payload…, k, key_positions, ctid_positions) ORDER BY sort_exprs`.
///
/// `key_positions` are the payload positions of the distinct key, which must
/// contain every sort key; `ctid_positions` are the payload's ctid columns, which
/// take the element-wise minimum over each distinct group.
pub fn distinct_topk_as_agg(
    payload: &[Expr],
    sort_exprs: Vec<SortExpr>,
    k: usize,
    key_positions: &[usize],
    ctid_positions: &[usize],
) -> Expr {
    let mut args = topk_args(payload, &sort_exprs, k);
    args.push(positions_lit(key_positions));
    args.push(positions_lit(ctid_positions));
    Expr::AggregateFunction(AggregateFunction::new_udf(
        distinct_topk_as_agg_udaf(),
        args,
        false,
        None,
        sort_exprs,
        None,
    ))
}

fn topk_args(payload: &[Expr], sort_exprs: &[SortExpr], k: usize) -> Vec<Expr> {
    let mut args = payload.to_vec();
    for sort in sort_exprs {
        if !args.contains(&sort.expr) {
            args.push(sort.expr.clone());
        }
    }
    args.push(lit(k as u64));
    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_array::types::Int64Type;
    use arrow_array::{Int64Array, StringArray};
    use datafusion::datasource::MemTable;
    use datafusion::physical_plan::displayable;
    use datafusion::prelude::{SessionConfig, SessionContext, col};

    const DESC_NULLS_FIRST: SortOptions = SortOptions {
        descending: true,
        nulls_first: true,
    };
    const ASC_NULLS_LAST: SortOptions = SortOptions {
        descending: false,
        nulls_first: false,
    };

    /// A `(score, id)` row: score nullable, id not.
    type Row = (Option<i64>, i64);

    /// `(score, id)`: score nullable, id not.
    fn schema() -> SchemaRef {
        Arc::new(Schema::new(vec![
            Field::new("score", DataType::Int64, true),
            Field::new("id", DataType::Int64, false),
        ]))
    }

    fn batch(rows: &[Row]) -> RecordBatch {
        let score = Int64Array::from(rows.iter().map(|r| r.0).collect::<Vec<_>>());
        let id = Int64Array::from(rows.iter().map(|r| r.1).collect::<Vec<_>>());
        RecordBatch::try_new(schema(), vec![Arc::new(score) as ArrayRef, Arc::new(id)]).unwrap()
    }

    /// Arrival mode, `ORDER BY score DESC NULLS FIRST, id ASC`.
    fn accumulator(k: usize) -> FusedTopK {
        FusedTopK::new(
            schema(),
            vec![(0, DESC_NULLS_FIRST), (1, ASC_NULLS_LAST)],
            Suffix::Arrival { next: 0 },
            vec![],
            k,
        )
        .unwrap()
    }

    /// Reads a single `List<Struct<score, id, …>>` scalar back as `(score, id)` rows.
    fn rows_of(value: &ScalarValue) -> Vec<Row> {
        let ScalarValue::List(list) = value else {
            panic!("expected a list scalar, got {value}");
        };
        assert_eq!(list.len(), 1);
        let rows = list.value(0);
        let rows = rows.as_struct();
        let score = rows.column(0).as_primitive::<Int64Type>();
        let id = rows.column(1).as_primitive::<Int64Type>();
        (0..rows.len())
            .map(|i| (score.is_valid(i).then(|| score.value(i)), id.value(i)))
            .collect()
    }

    #[test]
    fn keeps_top_k_across_batches() {
        let mut acc = accumulator(3);
        for rows in [
            &[(Some(5), 1), (None, 2), (Some(1), 3)][..],
            &[(Some(9), 4), (Some(5), 5)],
            &[(None, 6), (Some(0), 7)],
        ] {
            acc.update_batch(batch(rows).columns()).unwrap();
        }
        // NULLS FIRST puts both nulls ahead of every score; id orders the two nulls.
        assert_eq!(
            rows_of(&acc.evaluate().unwrap()),
            vec![(None, 2), (None, 6), (Some(9), 4)]
        );
    }

    #[test]
    fn arrival_mode_keeps_identical_rows() {
        let mut acc = accumulator(3);
        acc.update_batch(batch(&[(Some(5), 1), (Some(5), 1), (Some(2), 2)]).columns())
            .unwrap();
        acc.update_batch(batch(&[(Some(5), 1)]).columns()).unwrap();
        // Three identical rows are three rows, and they beat (2, 2).
        assert_eq!(
            rows_of(&acc.evaluate().unwrap()),
            vec![(Some(5), 1), (Some(5), 1), (Some(5), 1)]
        );
    }

    #[test]
    fn merge_combines_partial_states() {
        let mut first = accumulator(2);
        first
            .update_batch(batch(&[(Some(5), 1), (Some(3), 2), (Some(1), 3)]).columns())
            .unwrap();
        let mut second = accumulator(2);
        second
            .update_batch(batch(&[(Some(4), 4), (Some(9), 5)]).columns())
            .unwrap();
        // A partial that saw no input: its state is an empty list, not a null one.
        let mut idle = accumulator(2);

        let row_type = DataType::Struct(schema().fields().clone());
        let states = ScalarValue::iter_to_array([
            first.state().unwrap().remove(0),
            second.state().unwrap().remove(0),
            idle.state().unwrap().remove(0),
            ScalarValue::new_null_list(row_type, true, 1),
        ])
        .unwrap();

        // Final mode: never fed through update_batch, only merged.
        let mut merged = accumulator(2);
        merged.merge_batch(&[states]).unwrap();
        assert_eq!(
            rows_of(&merged.evaluate().unwrap()),
            vec![(Some(9), 5), (Some(5), 1)]
        );
    }

    /// `(score, id, ctid)` with the distinct key on `(score, id)`.
    fn distinct_schema() -> SchemaRef {
        Arc::new(Schema::new(vec![
            Field::new("score", DataType::Int64, true),
            Field::new("id", DataType::Int64, false),
            Field::new("ctid", DataType::UInt64, true),
        ]))
    }

    fn distinct_batch(rows: &[(Option<i64>, i64, Option<u64>)]) -> RecordBatch {
        RecordBatch::try_new(
            distinct_schema(),
            vec![
                Arc::new(Int64Array::from(
                    rows.iter().map(|r| r.0).collect::<Vec<_>>(),
                )) as ArrayRef,
                Arc::new(Int64Array::from(
                    rows.iter().map(|r| r.1).collect::<Vec<_>>(),
                )),
                Arc::new(UInt64Array::from(
                    rows.iter().map(|r| r.2).collect::<Vec<_>>(),
                )),
            ],
        )
        .unwrap()
    }

    /// Distinct mode, `ORDER BY score DESC NULLS FIRST, id ASC`, key `(score, id)`, ctid at 2.
    fn distinct_accumulator(k: usize) -> FusedTopK {
        FusedTopK::new(
            distinct_schema(),
            vec![(0, DESC_NULLS_FIRST), (1, ASC_NULLS_LAST)],
            Suffix::Distinct { positions: vec![] },
            vec![2],
            k,
        )
        .unwrap()
    }

    fn distinct_rows_of(value: &ScalarValue) -> Vec<(Option<i64>, i64, Option<u64>)> {
        let ScalarValue::List(list) = value else {
            panic!("expected a list scalar, got {value}");
        };
        let rows = list.value(0);
        let rows = rows.as_struct();
        let score = rows.column(0).as_primitive::<Int64Type>();
        let id = rows.column(1).as_primitive::<Int64Type>();
        let ctid = rows.column(2).as_primitive::<UInt64Type>();
        (0..rows.len())
            .map(|i| {
                (
                    score.is_valid(i).then(|| score.value(i)),
                    id.value(i),
                    ctid.is_valid(i).then(|| ctid.value(i)),
                )
            })
            .collect()
    }

    #[test]
    fn distinct_mode_collapses_groups_and_keeps_min_ctid() {
        let mut acc = distinct_accumulator(2);
        acc.update_batch(
            distinct_batch(&[
                (Some(5), 1, Some(9)),
                (Some(5), 1, Some(4)),
                (Some(3), 2, Some(7)),
            ])
            .columns(),
        )
        .unwrap();
        // (9, 3) enters and evicts (3, 2); a later (3, 2) must not come back; the
        // (5, 1) duplicates only lower its ctid, a NULL ctid changes nothing.
        acc.update_batch(
            distinct_batch(&[
                (Some(9), 3, Some(2)),
                (Some(5), 1, Some(6)),
                (Some(3), 2, Some(1)),
                (Some(5), 1, None),
                (Some(1), 4, Some(8)),
            ])
            .columns(),
        )
        .unwrap();
        assert_eq!(
            distinct_rows_of(&acc.evaluate().unwrap()),
            vec![(Some(9), 3, Some(2)), (Some(5), 1, Some(4))]
        );
    }

    #[test]
    fn distinct_merge_deduplicates_across_partials() {
        let mut first = distinct_accumulator(2);
        first
            .update_batch(distinct_batch(&[(Some(5), 1, Some(9)), (Some(3), 2, Some(7))]).columns())
            .unwrap();
        let mut second = distinct_accumulator(2);
        second
            .update_batch(distinct_batch(&[(Some(5), 1, Some(4)), (Some(9), 3, Some(2))]).columns())
            .unwrap();
        let states = ScalarValue::iter_to_array([
            first.state().unwrap().remove(0),
            second.state().unwrap().remove(0),
        ])
        .unwrap();

        let mut merged = distinct_accumulator(2);
        merged.merge_batch(&[states]).unwrap();
        // (5, 1) appears in both partials and merges to the smaller ctid.
        assert_eq!(
            distinct_rows_of(&merged.evaluate().unwrap()),
            vec![(Some(9), 3, Some(2)), (Some(5), 1, Some(4))]
        );
    }

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    /// Runs the UDAF through DataFusion's own planner and AggregateExec: the ORDER BY
    /// must not become a SortExec under the aggregate, the Partial/Final split must
    /// round-trip the state, and the declared return type must match what
    /// `evaluate` emits.
    #[test]
    fn plans_without_a_sort_and_splits_partial_final() {
        runtime().block_on(async {
            let table_schema = Arc::new(Schema::new(vec![
                Field::new("g", DataType::Utf8, false),
                Field::new("score", DataType::Int64, true),
                Field::new("id", DataType::Int64, false),
            ]));
            let part = |rows: &[(&str, Option<i64>, i64)]| {
                RecordBatch::try_new(
                    Arc::clone(&table_schema),
                    vec![
                        Arc::new(StringArray::from(
                            rows.iter().map(|r| r.0).collect::<Vec<_>>(),
                        )) as ArrayRef,
                        Arc::new(Int64Array::from(
                            rows.iter().map(|r| r.1).collect::<Vec<_>>(),
                        )),
                        Arc::new(Int64Array::from(
                            rows.iter().map(|r| r.2).collect::<Vec<_>>(),
                        )),
                    ],
                )
                .unwrap()
            };
            // Two partitions so the planner has a reason to split Partial from Final.
            let table = MemTable::try_new(
                Arc::clone(&table_schema),
                vec![
                    vec![part(&[
                        ("x", Some(5), 1),
                        ("x", Some(3), 2),
                        ("y", None, 3),
                    ])],
                    vec![part(&[
                        ("x", Some(9), 4),
                        ("y", Some(7), 5),
                        ("y", Some(8), 6),
                    ])],
                ],
            )
            .unwrap();

            let ctx =
                SessionContext::new_with_config(SessionConfig::new().with_target_partitions(2));
            let topk = topk_as_agg(
                &[col("score"), col("id")],
                vec![col("score").sort(false, true), col("id").sort(true, false)],
                2,
            );
            let df = ctx
                .read_table(Arc::new(table))
                .unwrap()
                .aggregate(vec![col("g")], vec![topk.alias("topk")])
                .unwrap();

            let plan = df.clone().create_physical_plan().await.unwrap();
            let text = displayable(plan.as_ref()).indent(true).to_string();
            assert!(text.contains("mode=Partial"), "{text}");
            assert!(text.contains("mode=FinalPartitioned"), "{text}");
            assert!(!text.contains("SortExec"), "{text}");

            let batches = df.collect().await.unwrap();
            let batch =
                arrow_select::concat::concat_batches(&batches[0].schema(), &batches).unwrap();
            let groups = batch.column(0).as_string::<i32>();
            let lists = batch.column(1).as_list::<i32>();
            let mut rows: Vec<(String, Vec<Row>)> = (0..batch.num_rows())
                .map(|i| {
                    let list = ScalarValue::List(Arc::new(lists.slice(i, 1)));
                    (groups.value(i).to_string(), rows_of(&list))
                })
                .collect();
            rows.sort();
            assert_eq!(
                rows,
                vec![
                    ("x".into(), vec![(Some(9), 4), (Some(5), 1)]),
                    ("y".into(), vec![(None, 3), (Some(8), 6)]),
                ]
            );
        });
    }

    /// The distinct UDAF through the planner: the position literals survive
    /// planning, and the Partial/Final merge deduplicates a group split across
    /// partitions down to its minimum ctid.
    #[test]
    fn distinct_plan_merges_duplicates_across_partitions() {
        runtime().block_on(async {
            let table = MemTable::try_new(
                distinct_schema(),
                vec![
                    vec![distinct_batch(&[
                        (Some(5), 1, Some(9)),
                        (Some(3), 2, Some(7)),
                        (Some(9), 3, Some(2)),
                    ])],
                    vec![distinct_batch(&[
                        (Some(5), 1, Some(4)),
                        (Some(3), 2, Some(1)),
                        (Some(1), 4, Some(8)),
                        (Some(5), 1, Some(6)),
                    ])],
                ],
            )
            .unwrap();

            let ctx =
                SessionContext::new_with_config(SessionConfig::new().with_target_partitions(2));
            let topk = distinct_topk_as_agg(
                &[col("score"), col("id"), col("ctid")],
                vec![col("score").sort(false, true), col("id").sort(true, false)],
                2,
                &[0, 1],
                &[2],
            );
            let df = ctx
                .read_table(Arc::new(table))
                .unwrap()
                .aggregate(vec![], vec![topk.alias("topk")])
                .unwrap();

            let plan = df.clone().create_physical_plan().await.unwrap();
            let text = displayable(plan.as_ref()).indent(true).to_string();
            assert!(text.contains("mode=Partial"), "{text}");
            assert!(text.contains("mode=Final"), "{text}");
            assert!(!text.contains("SortExec"), "{text}");

            let batches = df.collect().await.unwrap();
            let batch =
                arrow_select::concat::concat_batches(&batches[0].schema(), &batches).unwrap();
            assert_eq!(batch.num_rows(), 1);
            let list = ScalarValue::List(Arc::new(batch.column(0).as_list::<i32>().slice(0, 1)));
            assert_eq!(
                distinct_rows_of(&list),
                vec![(Some(9), 3, Some(2)), (Some(5), 1, Some(4))]
            );
        });
    }
}
