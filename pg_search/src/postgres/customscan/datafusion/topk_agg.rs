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
//! most K rows. The accumulator never evaluates a sort key itself: DataFusion
//! evaluates the ORDER BY expressions over the aggregate's input and hands them
//! to `update_batch` after the arguments, and the accumulator keeps those
//! ordering columns next to the payload, in its state as well as in memory, so
//! a merge can sort without re-evaluating anything. A sort key therefore need
//! not be an argument, only an expression over the aggregate's input.
//!
//! Each input batch is prefiltered against the state's worst row with Arrow
//! comparison kernels; the survivors are then sorted together with the state
//! and the first K rows become the new state. In distinct mode the sort key is
//! the ordering columns followed by the whole distinct key (every non-ctid
//! payload column), so equal rows land adjacent and collapse to one
//! representative: the row with the smallest ctid tuple, standing in for the
//! `min(ctid)` of a DISTINCT group-by. Every ORDER BY key is a function of the
//! distinct key (Postgres requires it in the select list), so state order is
//! ORDER BY order in both modes and the worst row is always last. Without an
//! ORDER BY or DISTINCT the state is simply the first K rows to arrive.

use arrow_array::cast::AsArray;
use arrow_array::types::UInt64Type;
use arrow_array::{
    Array, ArrayRef, BooleanArray, RecordBatch, RecordBatchOptions, Scalar, StructArray,
    UInt32Array,
};
use arrow_schema::{DataType, Field, FieldRef, Fields, Schema, SchemaRef, SortOptions};
use arrow_select::coalesce::BatchCoalescer;
use arrow_select::concat::concat_batches;
use arrow_select::filter::filter_record_batch;
use arrow_select::interleave::interleave_record_batch;
use arrow_select::take::take;
use datafusion::arrow::compute::SortColumn;
use datafusion::common::utils::{SingleRowListArrayBuilder, normalize_float_zero};
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

use crate::postgres::customscan::datafusion::fill_nulls_u64;

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

    /// The state is the result's `List<Struct>` widened by the ordering columns:
    /// a Final merges on them, so a partial's K rows must carry them. The
    /// ordering fields come from DataFusion, which derives them from the ORDER BY
    /// expressions; `row_schema` derives the accumulator's schema the same way,
    /// since a partial's output is checked against this declaration exactly.
    fn state_fields(&self, args: StateFieldsArgs) -> Result<Vec<FieldRef>> {
        let payload = payload_fields(args.input_fields)?;
        let fields: Vec<_> = payload
            .iter()
            .chain(args.ordering_fields)
            .cloned()
            .collect();
        Ok(vec![Arc::new(Field::new(
            format!("{}[rows]", args.name),
            list_of_rows(&fields),
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

        // The accumulator's batches must carry the declared row type exactly: the
        // payload from the return field (the argument fields' names differ) plus
        // the ordering fields, named by position as the state declares them.
        let schema = row_schema(&acc_args)?;

        // The ORDER BY keys, in order, each with its own direction and null
        // placement. DataFusion evaluates them and places them after the payload,
        // so they sit at positions n.. and no argument has to be matched.
        let mut sort: Vec<(usize, Option<SortOptions>)> = acc_args
            .order_bys
            .iter()
            .enumerate()
            .map(|(i, s)| (n + i, Some(s.options)))
            .collect();

        let (sort, ctid_positions) = if acc_args.is_distinct {
            let ctids = positions_arg(&acc_args, n + 1, name, "ctid positions", n)?;
            if let Some(p) = ctids
                .iter()
                .find(|&&p| schema.field(p).data_type() != &DataType::UInt64)
            {
                return Err(DataFusionError::Internal(format!(
                    "{name} ctid position {p} is not a UInt64 column"
                )));
            }
            // The rest of the distinct key: every payload column that is not a
            // ctid, appended after the ORDER BY keys with default options, so the
            // full sort key is the distinct key and equal rows sort adjacent.
            sort.extend((0..n).filter(|p| !ctids.contains(p)).map(|p| (p, None)));
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
            n,
        )?))
    }
}

const NUM_TRAILING_ARG_LITERALS: usize = 2;

/// Every argument but the `trailing` literals.
fn payload_fields(arg_fields: &[FieldRef]) -> Result<&[FieldRef]> {
    match arg_fields.len().checked_sub(NUM_TRAILING_ARG_LITERALS) {
        Some(n) => Ok(&arg_fields[..n]),
        _ => Err(DataFusionError::Internal(format!(
            "a Top-K aggregate takes at least zero payload columns and {NUM_TRAILING_ARG_LITERALS} trailing literal(s)"
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

/// The accumulator's row schema: the payload struct inside the `List<Struct>`
/// return field, followed by the ordering columns of the ORDER BY. It is the
/// state's row type, of which the result is the payload prefix.
fn row_schema(acc_args: &AccumulatorArgs) -> Result<SchemaRef> {
    use datafusion::physical_expr::aggregate::utils::ordering_fields;

    if let DataType::List(item) = acc_args.return_field.data_type()
        && let DataType::Struct(fields) = item.data_type()
    {
        let ordering_types: Vec<_> = acc_args
            .order_bys
            .iter()
            .map(|s| s.expr.data_type(acc_args.schema))
            .collect::<Result<_>>()?;
        let mut fields = fields.clone().to_vec();
        // Named by position after the payload, as `list_of_rows` names the state's
        // fields: the state batch's struct type must match the declared one exactly.
        let n = fields.len();
        fields.extend(
            ordering_fields(acc_args.order_bys, &ordering_types)
                .into_iter()
                .enumerate()
                .map(|(i, f)| Arc::new(f.as_ref().clone().with_name(format!("c{}", n + i)))),
        );
        return Ok(Arc::new(Schema::new(fields)));
    }
    Err(DataFusionError::Internal(format!(
        "a Top-K aggregate's return type must be a list of structs, got {}",
        acc_args.return_field.data_type()
    )))
}

/// The K best rows under an ORDER BY, kept as a RecordBatch sorted by `sort`.
/// A row is the payload followed by the ordering columns (see `row_schema`).
struct FusedTopK {
    schema: SchemaRef,
    k: usize,
    /// Row positions with their sort options: the ordering columns, one per
    /// ORDER BY key with its direction, followed in distinct mode by every
    /// non-ctid payload column (default options), so that it is the full
    /// distinct key. Empty without an ORDER BY or DISTINCT.
    sort: Vec<(usize, Option<SortOptions>)>,
    /// Payload positions of the ctid columns.
    ctid_positions: Vec<usize>,
    distinct: bool,
    /// At most K rows in `sort` order, so the last row is the worst; in arrival
    /// order when `sort` is empty.
    entries: RecordBatch,
    /// Where the payload ends and the ordering columns begin: `update_batch`
    /// cuts the trailing literals out of its input at this point, and `evaluate`
    /// projects the state down to it.
    num_payload_fields: usize,
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
        num_payload_fields: usize,
    ) -> Result<Self> {
        Ok(Self {
            entries: RecordBatch::new_empty(Arc::clone(&schema)),
            sort,
            schema,
            ctid_positions,
            k,
            distinct,
            num_payload_fields,
        })
    }

    /// The `sort` columns of the worst row, one single-row array each, once the
    /// state is full. `None` while it is not, so that nothing is filtered.
    fn worst_key(&self) -> Option<Vec<ArrayRef>> {
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

        let Some(worst) = self.worst_key() else {
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

    /// Admits the rows of `batch` (a row of `schema`: payload then ordering
    /// columns) that belong in the K best, and returns whether `entries` was
    /// replaced.
    fn absorb(&mut self, batch: &RecordBatch) -> Result<bool> {
        use datafusion::arrow::compute::concat;
        use datafusion::arrow::compute::{lexsort_to_indices, partition};

        if self.k == 0 {
            return Ok(false);
        }

        // Arrival mode (no ORDER BY, no DISTINCT) admits in arrival order, so once
        // full nothing later can enter: skip the batch outright.
        if !self.distinct && self.sort.is_empty() && self.entries.num_rows() == self.k {
            return Ok(false);
        }

        // 1) kernel-based prefilter against the worst row
        let survivors = filter_record_batch(batch, &self.could_enter(batch)?)?;
        if survivors.num_rows() == 0 {
            // nothing can enter
            return Ok(false);
        }

        // 1a) Arrival mode, not yet full: take enough survivors to fill k and call it good.
        if self.sort.is_empty() {
            let sk = (self.k - self.entries.num_rows()).min(survivors.num_rows());
            let enough_survivors = survivors.slice(0, sk);
            self.entries = concat_batches(&self.schema, [&self.entries, &enough_survivors])?;
            return Ok(true);
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

            // prep ctids in the same order, filling NULLS with u64::MAX so they sort last in min_by
            let ctids: Vec<_> = self
                .ctid_positions
                .iter()
                .map(|idx| {
                    let arr = take(
                        &concat(&[self.entries.column(*idx), survivors.column(*idx)])?,
                        &order,
                        None,
                    )?;
                    fill_nulls_u64(arr, u64::MAX)
                })
                .collect::<Result<_>>()?;
            let ctids: Vec<_> = ctids
                .iter()
                .map(|a| a.as_primitive::<UInt64Type>())
                .collect();

            // 4) For each of the first k groups, pick one row to represent it.
            let mut picks = Vec::with_capacity(self.k.min(ranges.len()));
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
                    .min_by(|a, b| {
                        let at = ctids.iter().map(|c| c.value(*a));
                        let bt = ctids.iter().map(|c| c.value(*b));
                        at.cmp(bt)
                    })
                    .expect("should always produce a value since range.len() > 0");
                picks.push(
                    self.pick_indices_from_sorted_concat(order.value(sorted_order_min_ctid_idx)),
                );
            }

            // 5) Construct the new record batch accordingly
            self.entries = interleave_record_batch(&[&self.entries, &survivors], &picks)?;
            Ok(true)
        } else {
            // 3) Interleave the record batches based on sort order
            self.entries = self.new_batch_from_sorted(&order, &survivors)?;
            Ok(true)
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
        // DataFusion's column layout here is payload ++ trailing literals ++ ORDER BYs, so skip
        // the literals.
        let n = self.num_payload_fields;
        let columns = values[..n]
            .iter()
            .chain(&values[n + NUM_TRAILING_ARG_LITERALS..])
            // incoming batches need their +/- zeroes normalized for sql equality semantics.
            .map(normalize_float_zero)
            .collect();

        // we must specify a row count to cover for the case where there are no payload columns
        let num_rows = values.first().map_or(0, |a| a.len());
        let options = RecordBatchOptions::new().with_row_count(Some(num_rows));
        let batch = RecordBatch::try_new_with_options(Arc::clone(&self.schema), columns, &options)?;
        let updated = self.absorb(&batch)?;
        if updated {
            self.compact_entries()?;
        }
        Ok(())
    }

    /// Other partials' K rows, with their ordering columns, so they go through
    /// the same admission as input rows without anything being re-evaluated.
    /// That shared admission is also what deduplicates across workers in
    /// distinct mode. A state list is a whole row of `schema`, so nothing is
    /// sliced off here.
    fn merge_batch(&mut self, states: &[ArrayRef]) -> Result<()> {
        let mut updated = false;
        for rows in states[0].as_list::<i32>().iter().flatten() {
            if !rows.is_empty() {
                let columns = rows.as_struct().columns();
                // we must specify a row count to cover for the case where there are no payload columns
                let options = RecordBatchOptions::new().with_row_count(Some(rows.len()));
                let batch = RecordBatch::try_new_with_options(
                    Arc::clone(&self.schema),
                    columns.to_vec(),
                    &options,
                )?;
                updated |= self.absorb(&batch)?;
            }
        }
        if updated {
            self.compact_entries()?;
        }
        Ok(())
    }

    /// The result is the payload alone, so the ordering columns are projected
    /// away; `state` keeps them.
    fn evaluate(&mut self) -> Result<ScalarValue> {
        let batch = self.drain();
        let payload_indices: Vec<_> = (0..self.num_payload_fields).collect();
        let batch = batch.project(&payload_indices)?;
        Ok(SingleRowListArrayBuilder::new(Arc::new(StructArray::from(batch))).build_list_scalar())
    }

    fn state(&mut self) -> Result<Vec<ScalarValue>> {
        let batch = self.drain();
        Ok(vec![
            SingleRowListArrayBuilder::new(Arc::new(StructArray::from(batch))).build_list_scalar(),
        ])
    }

    fn size(&self) -> usize {
        size_of_val(self)
            + self.entries.get_array_memory_size()
            + self.sort.capacity() * size_of::<(usize, Option<SortOptions>)>()
            + self.ctid_positions.capacity() * size_of::<usize>()
    }
}

/// `topk_as_agg(payload…, k, ctid_positions) ORDER BY sort_exprs`, with the aggregate's
/// DISTINCT flag set by `distinct`.
///
/// `sort_exprs` need not be in the `payload`: DataFusion evaluates them over the
/// DataFusion input the aggregate is applied to and the accumulator keeps the results
/// as ordering columns, so a sort key only has to be an expression over that input.
/// The emitted rows are exactly the `payload`.
///
/// In distinct mode the distinct key is every payload column but the `ctid_positions`, and
/// each group is represented by its row with the smallest ctid tuple, in place of a group-by's
/// `min(ctid)`.
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

    /// The `(score, id)` payload: score nullable, id not.
    fn payload_fields_of_test_rows() -> Vec<Field> {
        vec![
            Field::new("score", DataType::Int64, true),
            Field::new("id", DataType::Int64, false),
        ]
    }

    /// The accumulator's row under `ORDER BY score DESC NULLS FIRST, id ASC`: the payload,
    /// then the ordering columns DataFusion evaluates from the ORDER BY, which here are
    /// copies of `score` and `id` (nullable, as `ordering_fields` declares them).
    fn schema() -> SchemaRef {
        let mut fields = payload_fields_of_test_rows();
        fields.push(Field::new("score", DataType::Int64, true));
        fields.push(Field::new("id", DataType::Int64, true));
        Arc::new(Schema::new(fields))
    }

    /// The accumulator's row without an ORDER BY: the payload alone.
    fn payload_schema() -> SchemaRef {
        Arc::new(Schema::new(payload_fields_of_test_rows()))
    }

    fn score_and_id(rows: &[Row]) -> (ArrayRef, ArrayRef) {
        let score = Int64Array::from(rows.iter().map(|r| r.0).collect::<Vec<_>>());
        let id = Int64Array::from(rows.iter().map(|r| r.1).collect::<Vec<_>>());
        (Arc::new(score), Arc::new(id))
    }

    /// Stand-ins for the two trailing literal arguments (`k` and the ctid positions),
    /// which DataFusion evaluates to constant columns and `update_batch` skips.
    fn literal_columns(num_rows: usize) -> [ArrayRef; NUM_TRAILING_ARG_LITERALS] {
        let literal: ArrayRef = Arc::new(UInt64Array::from(vec![0u64; num_rows]));
        [Arc::clone(&literal), literal]
    }

    /// What DataFusion hands `update_batch` for `rows` under
    /// `ORDER BY score DESC NULLS FIRST, id ASC`: payload, literals, then the
    /// evaluated ORDER BY keys.
    fn values(rows: &[Row]) -> Vec<ArrayRef> {
        let (score, id) = score_and_id(rows);
        let mut values = vec![Arc::clone(&score), Arc::clone(&id)];
        values.extend(literal_columns(rows.len()));
        values.extend([score, id]);
        values
    }

    /// The same without an ORDER BY: payload and literals only.
    fn values_without_order_by(rows: &[Row]) -> Vec<ArrayRef> {
        let (score, id) = score_and_id(rows);
        let mut values = vec![score, id];
        values.extend(literal_columns(rows.len()));
        values
    }

    /// Arrival mode, `ORDER BY score DESC NULLS FIRST, id ASC`: the sort keys are the
    /// ordering columns after the two payload columns.
    fn accumulator(k: usize) -> FusedTopK {
        FusedTopK::new(
            schema(),
            vec![(2, Some(DESC_NULLS_FIRST)), (3, Some(ASC_NULLS_LAST))],
            vec![],
            k,
            false,
            2,
        )
        .unwrap()
    }

    /// Arrival mode without an ORDER BY.
    fn accumulator_without_order_by(k: usize) -> FusedTopK {
        FusedTopK::new(payload_schema(), vec![], vec![], k, false, 2).unwrap()
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
            acc.update_batch(&values(rows)).unwrap();
        }
        // NULLS FIRST puts both nulls ahead of every score; id orders the two nulls.
        assert_eq!(
            rows_of(&acc.evaluate().unwrap()),
            vec![(None, 2), (None, 6), (Some(9), 4)]
        );
    }

    /// No ORDER BY: there are no sort columns, so rows are appended in arrival
    /// order, and once full nothing later can displace an earlier arrival.
    #[test]
    fn no_order_by_keeps_first_arrivals() {
        let mut acc = accumulator_without_order_by(2);
        acc.update_batch(&values_without_order_by(&[
            (Some(5), 1),
            (None, 2),
            (Some(9), 3),
        ]))
        .unwrap();
        acc.update_batch(&values_without_order_by(&[(Some(7), 4)]))
            .unwrap();
        assert_eq!(
            rows_of(&acc.evaluate().unwrap()),
            vec![(Some(5), 1), (None, 2)]
        );
    }

    #[test]
    fn arrival_mode_keeps_identical_rows() {
        let mut acc = accumulator(3);
        acc.update_batch(&values(&[(Some(5), 1), (Some(5), 1), (Some(2), 2)]))
            .unwrap();
        acc.update_batch(&values(&[(Some(5), 1)])).unwrap();
        // Three identical rows are three rows, and they beat (2, 2).
        assert_eq!(
            rows_of(&acc.evaluate().unwrap()),
            vec![(Some(5), 1), (Some(5), 1), (Some(5), 1)]
        );
    }

    /// A `k` of zero admits nothing: `absorb` returns before looking at the
    /// batch, with sort keys and without, and the result is an empty list rather
    /// than an error.
    #[test]
    fn zero_k_keeps_nothing() {
        let mut acc = accumulator(0);
        acc.update_batch(&values(&[(Some(5), 1), (None, 2)]))
            .unwrap();
        acc.update_batch(&values(&[(Some(9), 3)])).unwrap();
        assert_eq!(rows_of(&acc.evaluate().unwrap()), Vec::<Row>::new());

        let mut acc = accumulator_without_order_by(0);
        acc.update_batch(&values_without_order_by(&[(Some(5), 1), (None, 2)]))
            .unwrap();
        acc.update_batch(&values_without_order_by(&[(Some(9), 3)]))
            .unwrap();
        assert_eq!(rows_of(&acc.evaluate().unwrap()), Vec::<Row>::new());
    }

    #[test]
    fn merge_combines_partial_states() {
        let mut first = accumulator(2);
        first
            .update_batch(&values(&[(Some(5), 1), (Some(3), 2), (Some(1), 3)]))
            .unwrap();
        let mut second = accumulator(2);
        second
            .update_batch(&values(&[(Some(4), 4), (Some(9), 5)]))
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

    /// The distinct accumulator's row: the `(score, id, ctid)` payload, then the
    /// ordering columns for `ORDER BY score DESC NULLS FIRST, id ASC`.
    fn distinct_accumulator_schema() -> SchemaRef {
        let mut fields = distinct_schema().fields().to_vec();
        fields.push(Arc::new(Field::new("score", DataType::Int64, true)));
        fields.push(Arc::new(Field::new("id", DataType::Int64, true)));
        Arc::new(Schema::new(fields))
    }

    /// What DataFusion hands `update_batch` for distinct rows: payload, literals,
    /// then the evaluated ORDER BY keys.
    fn distinct_values(rows: &[(Option<i64>, i64, Option<u64>)]) -> Vec<ArrayRef> {
        let batch = distinct_batch(rows);
        let mut values = batch.columns().to_vec();
        values.extend(literal_columns(rows.len()));
        values.extend([Arc::clone(batch.column(0)), Arc::clone(batch.column(1))]);
        values
    }

    /// Distinct mode, `ORDER BY score DESC NULLS FIRST, id ASC`, key `(score, id)`,
    /// ctid at 2. The sort is what `accumulator()` builds: the ordering columns
    /// (positions 3 and 4), then the non-ctid payload columns as the rest of the key.
    fn distinct_accumulator(k: usize) -> FusedTopK {
        FusedTopK::new(
            distinct_accumulator_schema(),
            vec![
                (3, Some(DESC_NULLS_FIRST)),
                (4, Some(ASC_NULLS_LAST)),
                (0, None),
                (1, None),
            ],
            vec![2],
            k,
            true,
            3,
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
        acc.update_batch(&distinct_values(&[
            (Some(5), 1, Some(9)),
            (Some(5), 1, Some(4)),
            (Some(3), 2, Some(7)),
        ]))
        .unwrap();
        // (9, 3) enters and evicts (3, 2); a later (3, 2) must not come back; the
        // (5, 1) duplicates only lower its ctid, a NULL ctid changes nothing.
        acc.update_batch(&distinct_values(&[
            (Some(9), 3, Some(2)),
            (Some(5), 1, Some(6)),
            (Some(3), 2, Some(1)),
            (Some(5), 1, None),
            (Some(1), 4, Some(8)),
        ]))
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
            .update_batch(&distinct_values(&[
                (Some(5), 1, Some(9)),
                (Some(3), 2, Some(7)),
            ]))
            .unwrap();
        let mut second = distinct_accumulator(2);
        second
            .update_batch(&distinct_values(&[
                (Some(5), 1, Some(4)),
                (Some(9), 3, Some(2)),
            ]))
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

    /// A `(g, score, id)` table in two partitions, so the planner has a reason to
    /// split Partial from Final.
    fn scores_table() -> MemTable {
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
        MemTable::try_new(
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
        .unwrap()
    }

    /// Runs the UDAF through DataFusion's own planner and AggregateExec: the ORDER BY
    /// must not become a SortExec under the aggregate, the Partial/Final split must
    /// round-trip the state, and the declared return type must match what
    /// `evaluate` emits.
    #[test]
    fn plans_without_a_sort_and_splits_partial_final() {
        runtime().block_on(async {
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
                .read_table(Arc::new(scores_table()))
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

    /// An ORDER BY the payload cannot express: `score` is not an argument, and the
    /// first key is an expression over it. The accumulator sorts on the ordering
    /// columns DataFusion evaluates and appends, never on the payload, and the
    /// Partial/Final split carries them in the state.
    #[test]
    fn orders_by_evaluated_keys_outside_the_payload() {
        use datafusion::prelude::lit;

        runtime().block_on(async {
            let ctx =
                SessionContext::new_with_config(SessionConfig::new().with_target_partitions(2));
            // ORDER BY score + 1 DESC NULLS LAST, id ASC
            let topk = topk_as_agg(
                &[col("id")],
                vec![
                    (col("score") + lit(1)).sort(false, false),
                    col("id").sort(true, false),
                ],
                2,
                &[],
                false,
            );
            let df = ctx
                .read_table(Arc::new(scores_table()))
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
            let rows = batch.column(0).as_list::<i32>().value(0);
            let rows = rows.as_struct();
            // The payload is `id` alone: the ordering columns never reach the result.
            assert_eq!(rows.num_columns(), 1);
            let ids: Vec<i64> = rows.column(0).as_primitive::<Int64Type>().values().to_vec();
            // Scores 9 and 8 win; the NULL score sorts last under NULLS LAST.
            assert_eq!(ids, vec![4, 6]);
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
