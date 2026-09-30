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
//! under an ORDER BY; with the aggregate's DISTINCT flag set it does the same
//! over distinct rows, so a `SELECT DISTINCT … ORDER BY … LIMIT k` runs as one
//! aggregate function that can sit beside other aggregates in a single
//! aggregate node.
//!
//! Both modes share one accumulator whose state is a sorted RecordBatch of at
//! most K rows. Each input batch is prefiltered against the state's worst row
//! with Arrow comparison kernels; the survivors are then sorted together with
//! the state and the first K rows become the new state. In distinct mode the
//! sort key is the whole distinct key (the ORDER BY columns, then the remaining
//! non-ctid columns), so equal rows land adjacent and collapse to one
//! representative: the row with the smallest ctid tuple, standing in for the
//! `min(ctid)` of a DISTINCT group-by. Every ORDER BY key is inside the distinct
//! key (Postgres requires it in the select list), so state order is ORDER BY
//! order in both modes and the worst row is always last. Without an ORDER BY or
//! DISTINCT the state is simply the first K rows to arrive.

use arrow_array::cast::AsArray;
use arrow_array::types::UInt64Type;
use arrow_array::{Array, ArrayRef, BooleanArray, RecordBatch, Scalar, StructArray, UInt32Array};
use arrow_schema::{DataType, Field, FieldRef, Fields, Schema, SchemaRef, SortOptions};
use arrow_select::coalesce::BatchCoalescer;
use arrow_select::concat::concat_batches;
use arrow_select::filter::filter_record_batch;
use arrow_select::interleave::interleave_record_batch;
use arrow_select::take::take;
use datafusion::arrow::compute::SortColumn;
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
use std::sync::{Arc, LazyLock};

use super::literal_arg;

pub const TOPK_AS_AGG_NAME: &str = "topk_as_agg";
pub const TOPK_AGG_ROWS_COL_NAME: &str = "__topk";

static TOPK_AS_AGG: LazyLock<Arc<AggregateUDF>> =
    LazyLock::new(|| Arc::new(AggregateUDF::from(TopKAgg::new())));

pub fn topk_as_agg_udaf() -> Arc<AggregateUDF> {
    Arc::clone(&TOPK_AS_AGG)
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct TopKAgg {
    signature: Signature,
}

impl TopKAgg {
    fn new() -> Self {
        Self {
            signature: Signature::variadic_any(Volatility::Immutable),
        }
    }
}

impl AggregateUDFImpl for TopKAgg {
    fn name(&self) -> &str {
        TOPK_AS_AGG_NAME
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
        Ok(list_of_rows(payload_fields(&fields)?))
    }

    fn return_field(&self, arg_fields: &[FieldRef]) -> Result<FieldRef> {
        let rows = list_of_rows(payload_fields(arg_fields)?);
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
    /// ordered changes nothing here; the accumulator sorts regardless.
    fn with_beneficial_ordering(
        self: Arc<Self>,
        _beneficial_ordering: bool,
    ) -> Result<Option<Arc<dyn AggregateUDFImpl>>> {
        Ok(Some(self))
    }

    fn accumulator(&self, acc_args: AccumulatorArgs) -> Result<Box<dyn Accumulator>> {
        let name = self.name();
        let payload = payload_fields(acc_args.expr_fields)?;
        let n = payload.len();

        let k = match literal_arg(&acc_args, n, name, "k")? {
            ScalarValue::UInt64(Some(k)) => *k as usize,
            other => {
                return Err(DataFusionError::Internal(format!(
                    "{name} k must be a UInt64 literal, got {other}"
                )));
            }
        };

        // The accumulator's batches must carry the declared row type exactly, so
        // take the payload schema from the return field rather than the argument
        // fields, whose names differ.
        let schema = row_schema(&acc_args.return_field)?;

        // Only the payload columns reach update_batch, so each sort key must be one
        // of them; rebase the ORDER BY onto payload positions.
        let mut sort: Vec<(usize, Option<SortOptions>)> = acc_args
            .order_bys
            .iter()
            .map(|s| {
                acc_args.exprs[..n]
                    .iter()
                    .position(|e| e.as_ref() == s.expr.as_ref())
                    .map(|i| (i, Some(s.options)))
                    .ok_or_else(|| {
                        DataFusionError::Internal(format!(
                            "{name} ORDER BY {} is not one of its arguments",
                            s.expr
                        ))
                    })
            })
            .collect::<Result<_>>()?;

        let (sort, ctid_positions) = if acc_args.is_distinct {
            let ctids = positions_arg(&acc_args, n + 1, name, "ctid positions", n)?;
            if let Some((p, _)) = sort.iter().find(|(p, _)| ctids.contains(p)) {
                return Err(DataFusionError::Internal(format!(
                    "{name} ORDER BY position {p} is a ctid position, which is not supported"
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
            // The rest of the distinct key: every payload column that is neither a
            // ctid nor an ORDER BY key. Appended after the ORDER BY keys with default
            // options, so the full sort key is the distinct key and equal rows sort
            // adjacent.
            let rest: Vec<(usize, Option<SortOptions>)> = (0..n)
                .filter(|p| {
                    !ctids.iter().any(|ctidp| ctidp == p) && !sort.iter().any(|(s, _)| s == p)
                })
                .map(|p| (p, None))
                .collect();
            sort.extend(rest);
            (sort, ctids)
        } else {
            (sort, Vec::new())
        };

        Ok(Box::new(FusedTopK::new(
            schema,
            sort,
            ctid_positions,
            k,
            acc_args.is_distinct,
        )?))
    }
}

const NUM_TRAILING_ARG_LITERALS: usize = 2;

/// Every argument but the `trailing` literals.
fn payload_fields(arg_fields: &[FieldRef]) -> Result<&[FieldRef]> {
    match arg_fields.len().checked_sub(NUM_TRAILING_ARG_LITERALS) {
        Some(n) if n > 0 => Ok(&arg_fields[..n]),
        _ => Err(DataFusionError::Internal(format!(
            "a Top-K aggregate takes at least one payload column and {NUM_TRAILING_ARG_LITERALS} trailing literal(s)"
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
        .map(|(i, f)| Arc::new(f.as_ref().clone().with_name(format!("c{i}"))))
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

/// The K best rows under an ORDER BY, kept as a RecordBatch sorted by `sort`.
struct FusedTopK {
    schema: SchemaRef,
    k: usize,
    /// Payload positions with their sort options: the ORDER BY keys, followed in
    /// distinct mode by the remaining non-ctid columns (default options), so
    /// that it is the full distinct key. Empty without an ORDER BY or DISTINCT.
    sort: Vec<(usize, Option<SortOptions>)>,
    ctid_positions: Vec<usize>,
    distinct: bool,
    /// At most K rows in `sort` order, so the last row is the worst; in arrival
    /// order when `sort` is empty.
    entries: RecordBatch,
}

impl std::fmt::Debug for FusedTopK {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FusedTopK")
            .field("k", &self.k)
            .field("sort", &self.sort)
            .field("distinct", &self.distinct)
            .field("entries", &self.entries.num_rows())
            .field("ctid_positions", &self.ctid_positions)
            .finish_non_exhaustive()
    }
}

impl FusedTopK {
    fn new(
        schema: SchemaRef,
        sort: Vec<(usize, Option<SortOptions>)>,
        ctid_positions: Vec<usize>,
        k: usize,
        distinct: bool,
    ) -> Result<Self> {
        Ok(Self {
            entries: RecordBatch::new_empty(Arc::clone(&schema)),
            sort,
            schema,
            ctid_positions,
            k,
            distinct,
        })
    }

    /// The `sort` columns of the worst row, one single-row array each, once the
    /// state is full. `None` while it is not, so that nothing is filtered.
    fn worst_prefix(&self) -> Option<Vec<ArrayRef>> {
        if self.entries.num_rows() < self.k {
            None
        } else {
            let last = self.entries.num_rows() - 1;
            let worst = self
                .sort
                .iter()
                .map(|(p, _)| self.entries.column(*p).slice(last, 1))
                .collect();
            Some(worst)
        }
    }

    /// Which rows of `batch` could still enter the state: all of them while it is
    /// not full, otherwise those sorting at or before the worst row on `sort`.
    /// A lexicographic comparison done column by column with kernels: a row is
    /// decided by the first column on which it differs from the worst row.
    ///
    /// Ties on every column pass. In distinct mode a tie is a duplicate of the
    /// worst row, which may still win on ctid; otherwise it is merely
    /// conservative, and the sort decides.
    fn could_enter(&self, batch: &RecordBatch) -> Result<BooleanArray> {
        use datafusion::arrow::compute::kernels::cmp::not_distinct;
        use datafusion::arrow::compute::{and, or};

        let Some(worst) = self.worst_prefix() else {
            return Ok(BooleanArray::from(vec![true; batch.num_rows()]));
        };

        let mut survivors = BooleanArray::from(vec![false; batch.num_rows()]);
        // rows still tied with the worst row on every column seen so far
        let mut ties = BooleanArray::from(vec![true; batch.num_rows()]);

        for ((idx, opts), wcol) in self.sort.iter().zip(worst.iter()) {
            let col = batch.column(*idx);
            // find all that sort before for this column
            let opts = (*opts).unwrap_or(SortOptions::default());
            let before = sorts_before(col, wcol, &opts)?;
            // a row that was tied so far and sorts before on this column is decided:
            // OR it into the survivors
            survivors = or(&survivors, &and(&ties, &before)?)?;
            // ties now becomes all points where the comparison is equal (not_distinct = same value or both null)
            ties = and(&ties, &not_distinct(col, &Scalar::new(wcol))?)?;
            if ties.true_count() == 0 {
                // nothing left to decide
                break;
            }
        }
        // Any remaining ties could enter, so OR them into the survivors
        survivors = or(&survivors, &ties)?;

        Ok(survivors)
    }

    /// The `sort` columns over state ++ `batch`, in that order, so that an index
    /// below `entries.num_rows()` names a state row and anything else a row of
    /// `batch` (see `pick_indices_from_sorted_concat`).
    fn concatenated_sort_columns(&self, batch: &RecordBatch) -> Result<Vec<SortColumn>> {
        use datafusion::arrow::compute::concat;

        let sort_keys = self.sort.iter().map(|(idx, opts)| {
            let values = concat(&[self.entries.column(*idx), batch.column(*idx)])?;
            Ok(SortColumn {
                values,
                options: *opts,
            })
        });
        sort_keys.collect()
    }

    fn pick_indices_from_sorted_concat(&self, i: u32) -> (usize, usize) {
        let i = i as usize;
        let n = self.entries.num_rows();
        if i < n { (0, i) } else { (1, i - n) }
    }

    /// `batch` must be the same RecordBatch that was provided to `concatenated_sort_columns`
    fn new_batch_from_sorted(
        &self,
        order: &UInt32Array,
        batch: &RecordBatch,
    ) -> Result<RecordBatch> {
        let picks: Vec<_> = order
            .values()
            .iter()
            .map(|i| self.pick_indices_from_sorted_concat(*i))
            .collect();
        let new_batch = interleave_record_batch(&[&self.entries, batch], &picks)?;
        Ok(new_batch)
    }

    /// Rebuild `entries` through a `BatchCoalescer`. The kernels in `absorb`
    /// share `Utf8View`/`BinaryView` data buffers instead of copying them, so
    /// without this the K-row state would pin whole input batches (and `size`
    /// would report them). The coalescer copies sparse view buffers into a
    /// compact one and leaves everything else alone.
    fn compact_entries(&mut self) -> Result<()> {
        if self.entries.num_rows() == 0 {
            return Ok(());
        }
        let batch = self.drain();
        let mut coalescer = BatchCoalescer::new(Arc::clone(&self.schema), batch.num_rows());
        coalescer.push_batch(batch)?;
        coalescer.finish_buffered_batch()?;
        self.entries = coalescer
            .next_completed_batch()
            .expect("one pushed batch should always yield one finished batch");
        Ok(())
    }

    fn absorb(&mut self, batch: &RecordBatch) -> Result<()> {
        use datafusion::arrow::compute::concat;
        use datafusion::arrow::compute::{lexsort_to_indices, partition};

        if self.k == 0 {
            return Ok(());
        }

        // if non-distinct with no sort keys (so arrival mode) and full, we can skip the entire
        // batch.
        if !self.distinct && self.sort.is_empty() && self.entries.num_rows() == self.k {
            return Ok(());
        }

        // 1) kernel-based prefilter against worst_prefix
        let survivors = filter_record_batch(batch, &self.could_enter(batch)?)?;
        if survivors.num_rows() == 0 {
            // nothing can enter
            return Ok(());
        }

        // 1a) if no sort columns present (which means we're not in DISTINCT mode and admission is
        // in arrival order), just take enough survivors to fill k and call it good.
        if self.sort.is_empty() {
            let sk = (self.k - self.entries.num_rows()).min(survivors.num_rows());
            let enough_survivors = survivors.slice(0, sk);
            self.entries = concat_batches(&self.schema, [&self.entries, &enough_survivors])?;
            return Ok(());
        }

        // 2) Sort (state ++ survivors)
        let sort_columns = self.concatenated_sort_columns(&survivors)?;
        // DISTINCT collapses duplicates after the sort, so every survivor must be
        // placed; non-DISTINCT is purely sort order, so only the top k are needed.
        let sort_limit = if self.distinct { None } else { Some(self.k) };
        let order = lexsort_to_indices(&sort_columns, sort_limit)?;

        if self.distinct {
            // 3) materialize the full key in sorted order, so that equal rows are
            // adjacent and `partition` yields one range per distinct group
            let sorted_keys: Vec<_> = sort_columns
                .iter()
                .map(|c| Ok(take(c.values.as_ref(), &order, None)?))
                .collect::<Result<_>>()?;
            let ranges = partition(&sorted_keys)?.ranges();

            // prep ctids in the same order
            let ctids: Vec<_> = self
                .ctid_positions
                .iter()
                .map(|idx| {
                    Ok(take(
                        &concat(&[self.entries.column(*idx), survivors.column(*idx)])?,
                        &order,
                        None,
                    )?)
                })
                .collect::<Result<_>>()?;

            // 4) For each of the first k groups, pick one row to represent it.
            let mut picks = Vec::with_capacity(self.k);
            for range in ranges.iter().take(self.k) {
                // Every row of a group carries the same key values, so for the
                // output any row will do; without ctid columns the pick cannot be
                // observed at all.
                if range.len() == 1 || ctids.is_empty() {
                    picks.push(self.pick_indices_from_sorted_concat(order.value(range.start)));
                    continue;
                }

                // Otherwise take the row with the lexicographically smallest ctid
                // tuple, NULLs last (mapped to u64::MAX). Like a group-by's
                // `min(ctid)`, that is a property of the group's rows, not of the
                // order they arrived or sorted in, so the state is the same
                // whatever the batching.
                let sorted_order_min_ctid_idx = range
                    .clone()
                    .min_by_key(|i| {
                        let tuple: Vec<_> = ctids
                            .iter()
                            .map(|col| {
                                let c = col.as_primitive::<UInt64Type>();
                                if c.is_valid(*i) {
                                    c.value(*i)
                                } else {
                                    u64::MAX
                                }
                            })
                            .collect();
                        tuple
                    })
                    .expect("should always produce a value since range.len() > 0");
                picks.push(
                    self.pick_indices_from_sorted_concat(order.value(sorted_order_min_ctid_idx)),
                );
            }

            // 5) Construct the new record batch accordingly
            self.entries = interleave_record_batch(&[&self.entries, &survivors], &picks)?;
            Ok(())
        } else {
            // 3) Interleave the record batches based on sort order
            self.entries = self.new_batch_from_sorted(&order, &survivors)?;
            Ok(())
        }
    }

    /// Return the existing record batch, and replace it with an empty one.
    fn drain(&mut self) -> RecordBatch {
        let mut res = RecordBatch::new_empty(Arc::clone(&self.schema));
        std::mem::swap(&mut self.entries, &mut res);
        res
    }
}

/// The rows of `col` that sort strictly before the single value in `refcol`
/// under `opts`, as a null-free boolean array.
fn sorts_before(col: &ArrayRef, refcol: &ArrayRef, opts: &SortOptions) -> Result<BooleanArray> {
    use arrow_select::filter::prep_null_mask_filter;
    use datafusion::arrow::compute::kernels::cmp::{gt, lt};
    use datafusion::arrow::compute::or;
    use datafusion::arrow::compute::{is_not_null, is_null};

    // if refcol is null, then the result depends only on opts.nulls_first.
    if refcol.is_null(0) {
        if opts.nulls_first {
            return Ok(BooleanArray::from(vec![false; col.len()]));
        } else {
            return Ok(is_not_null(col)?);
        }
    }

    // otherwise compare using sort direction
    let refval = Scalar::new(refcol);
    let cmp = if opts.descending {
        gt(col, &refval)?
    } else {
        lt(col, &refval)?
    };
    // lt/gt comparisons with null will leave nulls in the boolean array, which are treated as
    // "UNKNOWN". This converts them to false, leaving cmp as an array containing everything that
    // definitively sorts before the refcol when considering sort direction.
    //
    // When null_count() is 0 there is no null bitmap on cmp, and `prep_null_mask_filter`
    // panics on its absence.
    let cmp = match cmp.null_count() {
        0 => cmp,
        _ => prep_null_mask_filter(&cmp),
    };

    // if nulls_first and the column has nulls, OR those into the result since they sort
    // before the non-null refcol
    if opts.nulls_first && col.null_count() > 0 {
        Ok(or(&cmp, &is_null(col)?)?)
    } else {
        Ok(cmp)
    }
}

impl Accumulator for FusedTopK {
    fn update_batch(&mut self, values: &[ArrayRef]) -> Result<()> {
        let n = self.schema.fields().len();
        let batch = RecordBatch::try_new(Arc::clone(&self.schema), values[..n].to_vec())?;
        self.absorb(&batch)?;
        self.compact_entries()
    }

    /// Other partials' K rows. They go through the same admission, which is what
    /// deduplicates across workers in distinct mode.
    fn merge_batch(&mut self, states: &[ArrayRef]) -> Result<()> {
        let n = self.schema.fields().len();
        for rows in states[0].as_list::<i32>().iter().flatten() {
            if !rows.is_empty() {
                let columns = rows.as_struct().columns();
                let batch = RecordBatch::try_new(Arc::clone(&self.schema), columns[..n].to_vec())?;
                self.absorb(&batch)?;
            }
        }
        self.compact_entries()
    }

    fn evaluate(&mut self) -> Result<ScalarValue> {
        let batch = self.drain();
        Ok(SingleRowListArrayBuilder::new(Arc::new(StructArray::from(batch))).build_list_scalar())
    }

    fn state(&mut self) -> Result<Vec<ScalarValue>> {
        Ok(vec![self.evaluate()?])
    }

    fn size(&self) -> usize {
        size_of_val(self)
            + self.entries.get_array_memory_size()
            + self.sort.capacity() * size_of::<(usize, Option<SortOptions>)>()
            + self.ctid_positions.capacity() * size_of::<usize>()
    }
}

/// `topk_as_agg(payload…, k, ctid_positions) ORDER BY sort_exprs`, with the
/// aggregate's DISTINCT flag set by `distinct`.
///
/// Sort expressions missing from `payload` are appended to it, since the
/// accumulator can only sort on columns it receives, so the emitted rows may
/// carry more columns than `payload`. In distinct mode the distinct key is every
/// payload column but the `ctid_positions`, and each group is represented by its
/// row with the smallest ctid tuple, in place of a group-by's `min(ctid)`.
pub fn topk_as_agg(
    payload: &[Expr],
    sort_exprs: Vec<SortExpr>,
    k: usize,
    ctid_positions: &[usize],
    distinct: bool,
) -> Expr {
    ctid_positions
        .iter()
        .for_each(|p| assert!(*p < payload.len(), "ctid_positions must be valid"));

    let mut args = payload.to_vec();
    for sort in sort_exprs.iter() {
        if !args.contains(&sort.expr) {
            args.push(sort.expr.clone());
        }
    }
    args.push(lit(k as u64));
    args.push(positions_lit(ctid_positions));

    Expr::AggregateFunction(AggregateFunction::new_udf(
        topk_as_agg_udaf(),
        args,
        distinct,
        None,
        sort_exprs,
        None,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_array::types::Int64Type;
    use arrow_array::{Int64Array, StringArray, UInt64Array};
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
            vec![(0, Some(DESC_NULLS_FIRST)), (1, Some(ASC_NULLS_LAST))],
            vec![],
            k,
            false,
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

    /// No ORDER BY: the prefix is empty, every row ties, and once full nothing
    /// later can displace an earlier arrival.
    #[test]
    fn no_order_by_keeps_first_arrivals() {
        let mut acc = FusedTopK::new(schema(), vec![], vec![], 2, false).unwrap();
        acc.update_batch(batch(&[(Some(5), 1), (None, 2), (Some(9), 3)]).columns())
            .unwrap();
        acc.update_batch(batch(&[(Some(7), 4)]).columns()).unwrap();
        assert_eq!(
            rows_of(&acc.evaluate().unwrap()),
            vec![(Some(5), 1), (None, 2)]
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

    /// A `k` of zero admits nothing: the empty map counts as full, so the
    /// prefilter drops every row, with sort keys and without, and the result is
    /// an empty list rather than an error.
    #[test]
    fn zero_k_keeps_nothing() {
        let no_order_by = FusedTopK::new(schema(), vec![], vec![], 0, false).unwrap();
        for mut acc in [accumulator(0), no_order_by] {
            acc.update_batch(batch(&[(Some(5), 1), (None, 2)]).columns())
                .unwrap();
            acc.update_batch(batch(&[(Some(9), 3)]).columns()).unwrap();
            assert_eq!(rows_of(&acc.evaluate().unwrap()), Vec::<Row>::new());
        }
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
            vec![(0, Some(DESC_NULLS_FIRST)), (1, Some(ASC_NULLS_LAST))],
            vec![2],
            k,
            true,
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
                &[],
                false,
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
            let topk = topk_as_agg(
                &[col("score"), col("id"), col("ctid")],
                vec![col("score").sort(false, true), col("id").sort(true, false)],
                2,
                &[2],
                true,
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
