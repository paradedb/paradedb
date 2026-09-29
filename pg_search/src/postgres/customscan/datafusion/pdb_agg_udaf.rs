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

//! `pdb.agg()` as one aggregate function.
//!
//! The aggregate scan lowers a `pdb.agg()` spec onto the group keys of its
//! `Aggregate` node: every `terms` level is a grouping set. JoinScan computes
//! its window aggregates in the node that also holds its Top-K, which has to
//! stay free of group keys, so here the grouping moves inside the accumulator.
//! Each `terms` level interns its keys into bucket ids, and the aggregates the
//! aggregate scan would have planned for that level run per bucket. The finished
//! buckets are laid out as the rows the grouping-set plan produces, and the
//! aggregate scan's assembler folds them into the document.

use std::mem::size_of_val;
use std::ops::Range;
use std::sync::{Arc, LazyLock, Mutex};

use arrow_array::cast::AsArray;
use arrow_array::{Array, ArrayRef, RecordBatch, RecordBatchOptions, UInt64Array, new_null_array};
use arrow_schema::{DataType, Field, FieldRef, Schema, SchemaRef};
use datafusion::arrow::compute::cast;
use datafusion::arrow::ipc::reader::StreamReader;
use datafusion::arrow::ipc::writer::StreamWriter;
use datafusion::common::ScalarValue;
use datafusion::error::{DataFusionError, Result};
use datafusion::logical_expr::expr::AggregateFunctionParams;
use datafusion::logical_expr::function::{AccumulatorArgs, StateFieldsArgs};
use datafusion::logical_expr::{
    Accumulator, Aggregate, AggregateUDF, AggregateUDFImpl, EmitTo, Expr, GroupsAccumulator,
    Signature, Volatility, lit, udaf_default_human_display,
};
use datafusion::physical_expr::aggregate::{AggregateExprBuilder, AggregateFunctionExpr};
use datafusion::physical_plan::aggregates::group_values::{GroupValues, new_group_values};
use datafusion::physical_plan::aggregates::order::GroupOrdering;
use datafusion_functions_aggregate_common::aggregate::groups_accumulator::GroupsAccumulatorAdapter;
use pgrx::{IntoDatum, pg_sys};

use super::{literal_arg, reject_distinct};
use crate::postgres::customscan::aggregatescan::datafusion_exec::{
    make_plan_position_col, pdb_key_expr, pdb_metric_call,
};
use crate::postgres::customscan::aggregatescan::pdb_agg::{
    PdbAggColumn, PdbAggFieldRef, PdbAggPlan, PdbAggRequest, PdbKeySpec, assemble_pdb_agg_rows,
};
use crate::postgres::customscan::joinscan::build::RelNode;

pub const PDB_AGG_NAME: &str = "pdb_agg";

static PDB_AGG: LazyLock<Arc<AggregateUDF>> =
    LazyLock::new(|| Arc::new(AggregateUDF::from(PdbAgg::new())));

pub fn pdb_agg_udaf() -> Arc<AggregateUDF> {
    Arc::clone(&PDB_AGG)
}

/// The leading literal argument of a call: the request. The key columns and the
/// arguments of each metric follow.
const LITERAL_ARGS: usize = 1;

/// The document as it leaves the aggregate. It is one value fanned out to every
/// row the scan returns, so it travels as a dictionary: a key per row rather
/// than a copy of the document.
fn document_type() -> DataType {
    DataType::Dictionary(Box::new(DataType::Int32), Box::new(DataType::Utf8))
}

/// One row of a document column as a jsonb Datum.
pub fn json_document_to_datum(
    col: &dyn Array,
    row_idx: usize,
) -> anyhow::Result<Option<pg_sys::Datum>> {
    if col.is_null(row_idx) || col.data_type() == &DataType::Null {
        return Ok(None);
    }
    let text = cast(&col.slice(row_idx, 1), &DataType::Utf8)?;
    let document: serde_json::Value = serde_json::from_str(text.as_string::<i32>().value(0))?;
    Ok(pgrx::JsonB(document).into_datum())
}

/// [`request_plan`] leaves these out, so the plan's columns hold none.
const NO_ROOT_COLUMNS: &str = "a lone request has no SQL group keys or standard aggregates";

/// The layout of a lone request: no SQL group keys and no standard aggregates.
/// The call and its accumulator both derive the argument order from it. The
/// entry index only identifies a `FILTER`, which this path does not take.
fn request_plan(request: &PdbAggRequest) -> Result<PdbAggPlan> {
    PdbAggPlan::build(&[(0, request, false)], 0, 0)
}

/// The request as a call argument. JSON rather than postcard: a request carries
/// its spec as `serde_json::Value` and as Tantivy's flattened `Aggregation`,
/// which need a self-describing format. Dictionary-encoded, so materializing
/// the literal per input row costs a key rather than a copy of the request.
fn request_literal(request: &PdbAggRequest) -> Result<Expr> {
    let bytes = serde_json::to_vec(request).map_err(|e| DataFusionError::External(Box::new(e)))?;
    Ok(lit(ScalarValue::Dictionary(
        Box::new(DataType::Int32),
        Box::new(ScalarValue::Binary(Some(bytes))),
    )))
}

fn request_from_args(args: &AccumulatorArgs) -> Result<PdbAggRequest> {
    request_from_literal(literal_arg(args, 0, PDB_AGG_NAME, "request")?)
}

fn request_from_literal(literal: &ScalarValue) -> Result<PdbAggRequest> {
    let bytes = match literal {
        ScalarValue::Dictionary(_, inner) => match inner.as_ref() {
            ScalarValue::Binary(Some(bytes)) => bytes,
            other => {
                return Err(DataFusionError::Internal(format!(
                    "{PDB_AGG_NAME} request must be a non-null Binary literal, got {other}"
                )));
            }
        },
        other => {
            return Err(DataFusionError::Internal(format!(
                "{PDB_AGG_NAME} request must be a dictionary-encoded Binary literal, got {other}"
            )));
        }
    };
    serde_json::from_slice(bytes).map_err(|e| {
        DataFusionError::Internal(format!("{PDB_AGG_NAME} request does not decode: {e}"))
    })
}

/// `pdb_agg(request, keys..., metric arguments...)`: the document of `request`
/// over the rows of `plan`. The keys are the `terms` fields of the spec and the
/// metric arguments those of the aggregates the aggregate scan plans for it,
/// both in the order of the request's [`PdbAggPlan`].
pub fn pdb_agg(request: &PdbAggRequest, plan: &RelNode) -> Result<Expr> {
    pdb_agg_call(
        request,
        |key| pdb_key_expr(key, plan),
        |field| make_plan_position_col(plan, field.plan_position, &field.field_name),
    )
}

/// [`pdb_agg`] over whatever supplies the key expressions and the metric
/// columns.
fn pdb_agg_call(
    request: &PdbAggRequest,
    key: impl Fn(&PdbKeySpec) -> Expr,
    column: impl Fn(&PdbAggFieldRef) -> Expr,
) -> Result<Expr> {
    let pdb_plan = request_plan(request)?;
    let mut args = vec![request_literal(request)?];
    args.extend(pdb_plan.keys.iter().map(key));
    for metric in &pdb_plan.metrics {
        args.extend(pdb_metric_call(metric, &column).args);
    }
    Ok(pdb_agg_udaf().call(args))
}

/// One metric of the request's plan as the aggregate it runs as.
struct Metric {
    expr: Arc<AggregateFunctionExpr>,
    /// Where its arguments sit among the call's.
    args: Range<usize>,
    state_fields: Vec<FieldRef>,
}

impl Metric {
    /// One accumulator over every bucket of a level. The standard aggregates
    /// have a vectorized form; pg_search's own run one accumulator per bucket.
    fn groups_accumulator(&self) -> Result<Box<dyn GroupsAccumulator>> {
        if self.expr.groups_accumulator_supported() {
            return self.expr.create_groups_accumulator();
        }
        let expr = Arc::clone(&self.expr);
        Ok(Box::new(GroupsAccumulatorAdapter::new(move || {
            expr.create_accumulator()
        })))
    }
}

/// The buckets of one level of the plan: a `terms` level, or the root, which
/// is the one bucket of every row.
struct Level {
    /// Positions into the plan's keys, in the order they are interned.
    keys: Vec<usize>,
    /// `None` for the root, which has no key to intern.
    buckets: Option<Box<dyn GroupValues>>,
    num_buckets: usize,
    /// The metrics read from this level's rows, by position in the plan, each
    /// with its accumulator.
    metrics: Vec<(usize, Box<dyn GroupsAccumulator>)>,
    /// The bucket of each row of the batch at hand.
    bucket_of_row: Vec<usize>,
}

impl Level {
    /// Assign each of the `num_rows` rows of `keys` to its bucket.
    fn intern(&mut self, keys: &[ArrayRef], num_rows: usize) -> Result<()> {
        self.bucket_of_row.clear();
        match &mut self.buckets {
            Some(buckets) => {
                buckets.intern(keys, &mut self.bucket_of_row)?;
                self.num_buckets = buckets.len();
            }
            None => {
                self.bucket_of_row.resize(num_rows, 0);
                self.num_buckets = 1;
            }
        }
        Ok(())
    }

    fn emit_keys(&mut self) -> Result<Vec<ArrayRef>> {
        match &mut self.buckets {
            Some(buckets) => buckets.emit(EmitTo::All),
            None => Ok(Vec::new()),
        }
    }

    fn size(&self) -> usize {
        self.buckets.as_ref().map_or(0, |buckets| buckets.size())
            + self
                .metrics
                .iter()
                .map(|(_, accumulator)| accumulator.size())
                .sum::<usize>()
            + self.bucket_of_row.capacity() * size_of::<usize>()
    }
}

struct PdbAggAccumulator {
    plan: PdbAggPlan,
    key_fields: Vec<FieldRef>,
    metrics: Vec<Metric>,
    /// The levels that hold anything: a root without metrics of its own, that of
    /// a spec whose root is a `terms`, is left out. Behind a lock only because
    /// an accumulator has to be `Sync`, which the bucket state is not.
    levels: Mutex<Vec<(usize, Level)>>,
}

impl std::fmt::Debug for PdbAggAccumulator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PdbAggAccumulator")
    }
}

impl PdbAggAccumulator {
    fn try_new(args: &AccumulatorArgs) -> Result<Self> {
        let request = request_from_args(args)?;
        let plan = request_plan(&request)?;

        let arg_fields = args.expr_fields;
        let keys_end = LITERAL_ARGS + plan.keys.len();
        let key_fields = arg_fields
            .get(LITERAL_ARGS..keys_end)
            .ok_or_else(|| wrong_arguments(arg_fields.len()))?
            .to_vec();

        let input_schema = Arc::new(args.schema.clone());
        let mut next_arg = keys_end;
        let mut metrics = Vec::with_capacity(plan.metrics.len());
        for (j, metric) in plan.metrics.iter().enumerate() {
            // Only the function and the number of arguments are read here, and
            // neither depends on the column.
            let call = pdb_metric_call(metric, |_| lit(ScalarValue::Null));
            let metric_args = next_arg..next_arg + call.args.len();
            next_arg = metric_args.end;
            let exprs = args
                .exprs
                .get(metric_args.clone())
                .ok_or_else(|| wrong_arguments(arg_fields.len()))?
                .to_vec();
            let expr = AggregateExprBuilder::new(call.udaf, exprs)
                .schema(Arc::clone(&input_schema))
                .alias(format!("__pdb_m{j}"))
                .build()?;
            metrics.push(Metric {
                state_fields: expr.state_fields()?,
                expr: Arc::new(expr),
                args: metric_args,
            });
        }
        if next_arg != arg_fields.len() {
            return Err(wrong_arguments(arg_fields.len()));
        }

        let mut levels = Vec::new();
        for (level, level_metrics) in plan.metrics_by_level().into_iter().enumerate() {
            let keys = plan.levels[level].clone();
            if keys.is_empty() && level_metrics.is_empty() {
                continue;
            }
            let buckets = if keys.is_empty() {
                None
            } else {
                let fields: Vec<FieldRef> =
                    keys.iter().map(|&k| Arc::clone(&key_fields[k])).collect();
                Some(new_group_values(
                    Arc::new(Schema::new(fields)),
                    &GroupOrdering::None,
                )?)
            };
            let level_metrics = level_metrics
                .into_iter()
                .map(|j| Ok((j, metrics[j].groups_accumulator()?)))
                .collect::<Result<_>>()?;
            levels.push((
                level,
                Level {
                    keys,
                    buckets,
                    num_buckets: 0,
                    metrics: level_metrics,
                    bucket_of_row: Vec::new(),
                },
            ));
        }

        Ok(Self {
            plan,
            key_fields,
            metrics,
            levels: Mutex::new(levels),
        })
    }

    fn levels(&mut self) -> &mut Vec<(usize, Level)> {
        self.levels
            .get_mut()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// The buckets of every level as the rows the aggregate scan's grouping-set
    /// plan produces, in the column order of the plan. A column a level does
    /// not have is NULL on its rows.
    fn bucket_rows(&mut self) -> Result<RecordBatch> {
        let nullable =
            |field: &FieldRef| -> FieldRef { Arc::new(field.as_ref().clone().with_nullable(true)) };
        let layout: Vec<PdbAggColumn> = self.plan.columns().collect();
        let fields: Vec<FieldRef> = layout
            .iter()
            .map(|column| match column {
                PdbAggColumn::GroupingId => Arc::new(Field::new(
                    Aggregate::INTERNAL_GROUPING_ID,
                    DataType::UInt64,
                    false,
                )),
                PdbAggColumn::Key(key) => nullable(&self.key_fields[*key]),
                PdbAggColumn::Metric(metric) => nullable(&self.metrics[*metric].expr.field()),
                PdbAggColumn::GroupKey(_) | PdbAggColumn::StdAgg(_) => {
                    unreachable!("{NO_ROOT_COLUMNS}")
                }
            })
            .collect();

        let grouping_ids: Vec<u64> = (0..self.plan.levels.len())
            .map(|level| self.plan.grouping_id_for_level(level))
            .collect();
        let num_keys = self.key_fields.len();
        let num_metrics = self.metrics.len();
        let mut columns: Vec<Vec<ArrayRef>> = vec![Vec::new(); fields.len()];
        let mut num_rows = 0;
        for (level, state) in self.levels() {
            let rows = state.num_buckets;
            if rows == 0 {
                continue;
            }
            num_rows += rows;
            let mut keys: Vec<Option<ArrayRef>> = vec![None; num_keys];
            let emitted = state.emit_keys()?;
            for (&key, values) in state.keys.iter().zip(emitted) {
                keys[key] = Some(values);
            }
            let mut metrics: Vec<Option<ArrayRef>> = vec![None; num_metrics];
            for (metric, accumulator) in &mut state.metrics {
                metrics[*metric] = Some(accumulator.evaluate(EmitTo::All)?);
            }
            for ((column, holds), field) in columns.iter_mut().zip(&layout).zip(&fields) {
                let values =
                    match holds {
                        PdbAggColumn::GroupingId => Some(Arc::new(UInt64Array::from(
                            vec![grouping_ids[*level]; rows],
                        )) as ArrayRef),
                        PdbAggColumn::Key(key) => keys[*key].take(),
                        PdbAggColumn::Metric(metric) => metrics[*metric].take(),
                        PdbAggColumn::GroupKey(_) | PdbAggColumn::StdAgg(_) => {
                            unreachable!("{NO_ROOT_COLUMNS}")
                        }
                    };
                column.push(match values {
                    Some(values) if values.data_type() == field.data_type() => values,
                    Some(values) => cast(&values, field.data_type())?,
                    None => new_null_array(field.data_type(), rows),
                });
            }
        }

        let schema = Arc::new(Schema::new(fields));
        let columns = columns
            .iter()
            .zip(schema.fields())
            .map(|(column, field)| {
                if column.is_empty() {
                    return Ok(new_null_array(field.data_type(), 0));
                }
                let column: Vec<&dyn Array> = column.iter().map(|values| values.as_ref()).collect();
                Ok(arrow_select::concat::concat(&column)?)
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(RecordBatch::try_new_with_options(
            schema,
            columns,
            &RecordBatchOptions::new().with_row_count(Some(num_rows)),
        )?)
    }

    /// The schema a level's buckets travel under between accumulators: its keys,
    /// then the state of each of its metrics.
    fn level_state_schema(&self, state: &Level) -> SchemaRef {
        let mut fields: Vec<FieldRef> = state
            .keys
            .iter()
            .map(|&key| Arc::new(self.key_fields[key].as_ref().clone().with_nullable(true)))
            .collect();
        for (metric, _) in &state.metrics {
            fields.extend(
                self.metrics[*metric]
                    .state_fields
                    .iter()
                    .map(|field| Arc::new(field.as_ref().clone().with_nullable(true))),
            );
        }
        Arc::new(Schema::new(fields))
    }
}

fn wrong_arguments(found: usize) -> DataFusionError {
    DataFusionError::Internal(format!(
        "{PDB_AGG_NAME} was called with {found} arguments, which is not what its request takes"
    ))
}

/// A level's buckets are framed by their length, a level without any by zero.
const LEVEL_LENGTH_BYTES: usize = size_of::<u64>();

impl Accumulator for PdbAggAccumulator {
    fn update_batch(&mut self, values: &[ArrayRef]) -> Result<()> {
        let num_rows = values.first().map_or(0, |values| values.len());
        if num_rows == 0 {
            return Ok(());
        }
        let metrics = &self.metrics;
        for (_, state) in self
            .levels
            .get_mut()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
        {
            let keys: Vec<ArrayRef> = state
                .keys
                .iter()
                .map(|&key| Arc::clone(&values[LITERAL_ARGS + key]))
                .collect();
            state.intern(&keys, num_rows)?;
            for (metric, accumulator) in &mut state.metrics {
                accumulator.update_batch(
                    &values[metrics[*metric].args.clone()],
                    &state.bucket_of_row,
                    None,
                    state.num_buckets,
                )?;
            }
        }
        Ok(())
    }

    /// Runs on empty input too: no level has a bucket then, and the assembler
    /// synthesizes the root the way it does for the aggregate scan. It does the
    /// same for a spec whose root is a `terms`, which keeps no root bucket.
    fn evaluate(&mut self) -> Result<ScalarValue> {
        let rows = self.bucket_rows()?;
        let assembled = assemble_pdb_agg_rows(rows.schema(), &[rows], &self.plan, Some(&[]))?;
        let document = assembled
            .json
            .into_iter()
            .next()
            .and_then(|documents| documents.into_iter().next())
            .ok_or_else(|| {
                DataFusionError::Internal(format!("{PDB_AGG_NAME} assembled no document"))
            })?;
        Ok(ScalarValue::Dictionary(
            Box::new(DataType::Int32),
            Box::new(ScalarValue::Utf8(Some(document.to_string()))),
        ))
    }

    fn size(&self) -> usize {
        let levels = self
            .levels
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        size_of_val(self)
            + levels
                .iter()
                .map(|(_, state)| size_of_val(state) + state.size())
                .sum::<usize>()
    }

    /// Every level's buckets, keys beside the state of each metric, as one
    /// Arrow IPC stream per level. Levels differ in schema, so they do not share
    /// a stream.
    fn state(&mut self) -> Result<Vec<ScalarValue>> {
        let schemas: Vec<SchemaRef> = {
            let levels = self
                .levels
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            levels
                .iter()
                .map(|(_, state)| self.level_state_schema(state))
                .collect()
        };
        let mut bytes = Vec::new();
        for ((_, state), schema) in self.levels().iter_mut().zip(schemas) {
            let frame = bytes.len();
            bytes.extend_from_slice(&0u64.to_le_bytes());
            if state.num_buckets == 0 {
                continue;
            }
            let mut columns = state.emit_keys()?;
            for (_, accumulator) in &mut state.metrics {
                columns.extend(accumulator.state(EmitTo::All)?);
            }
            let columns = columns
                .into_iter()
                .zip(schema.fields())
                .map(|(values, field)| {
                    if values.data_type() == field.data_type() {
                        Ok(values)
                    } else {
                        Ok(cast(&values, field.data_type())?)
                    }
                })
                .collect::<Result<Vec<_>>>()?;
            let batch = RecordBatch::try_new(Arc::clone(&schema), columns)?;
            let mut writer = StreamWriter::try_new(&mut bytes, &schema)?;
            writer.write(&batch)?;
            writer.finish()?;
            drop(writer);
            let length = (bytes.len() - frame - LEVEL_LENGTH_BYTES) as u64;
            bytes[frame..frame + LEVEL_LENGTH_BYTES].copy_from_slice(&length.to_le_bytes());
            state.num_buckets = 0;
        }
        Ok(vec![ScalarValue::Binary(Some(bytes))])
    }

    fn merge_batch(&mut self, states: &[ArrayRef]) -> Result<()> {
        let metrics = &self.metrics;
        let levels = self
            .levels
            .get_mut()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for mut bytes in states[0].as_binary::<i32>().iter().flatten() {
            for (_, state) in levels.iter_mut() {
                let (length, rest) = bytes
                    .split_at_checked(LEVEL_LENGTH_BYTES)
                    .ok_or_else(truncated_state)?;
                let length = u64::from_le_bytes(length.try_into().expect("split at its length"));
                let (level_bytes, rest) = rest
                    .split_at_checked(length as usize)
                    .ok_or_else(truncated_state)?;
                bytes = rest;
                if level_bytes.is_empty() {
                    continue;
                }
                for batch in StreamReader::try_new(level_bytes, None)? {
                    let batch = batch?;
                    let (keys, mut metric_states) = batch.columns().split_at(state.keys.len());
                    state.intern(keys, batch.num_rows())?;
                    for (metric, accumulator) in &mut state.metrics {
                        let (values, rest) =
                            metric_states.split_at(metrics[*metric].state_fields.len());
                        metric_states = rest;
                        accumulator.merge_batch(values, &state.bucket_of_row, state.num_buckets)?;
                    }
                }
            }
        }
        Ok(())
    }
}

fn truncated_state() -> DataFusionError {
    DataFusionError::Internal(format!("{PDB_AGG_NAME} state is truncated"))
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct PdbAgg {
    signature: Signature,
}

impl PdbAgg {
    fn new() -> Self {
        Self {
            signature: Signature::variadic_any(Volatility::Immutable),
        }
    }
}

impl AggregateUDFImpl for PdbAgg {
    fn name(&self) -> &str {
        PDB_AGG_NAME
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> Result<DataType> {
        Ok(document_type())
    }

    /// EXPLAIN shows the spec, the way the other renderings of `pdb.agg()` do.
    /// The arguments are the serialized request, which prints as the first
    /// bytes of a binary literal, and columns the spec already names.
    fn human_display(&self, params: &AggregateFunctionParams) -> Result<String> {
        match params.args.first() {
            Some(Expr::Literal(literal, _)) => {
                let request = request_from_literal(literal)?;
                Ok(format!("{PDB_AGG_NAME}({})", request.agg_json))
            }
            _ => udaf_default_human_display(self, params),
        }
    }

    fn accumulator(&self, args: AccumulatorArgs) -> Result<Box<dyn Accumulator>> {
        reject_distinct(&args, PDB_AGG_NAME)?;
        Ok(Box::new(PdbAggAccumulator::try_new(&args)?))
    }

    fn state_fields(&self, args: StateFieldsArgs) -> Result<Vec<FieldRef>> {
        Ok(vec![
            Field::new(format!("{}[buckets]", args.name), DataType::Binary, true).into(),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::MvccVisibility;
    use crate::schema::SearchFieldType;
    use arrow_array::{Float64Array, Int64Array, StringViewArray};
    use datafusion::datasource::MemTable;
    use datafusion::physical_plan::displayable;
    use datafusion::prelude::{SessionConfig, SessionContext, col};
    use serde_json::{Value, json};

    /// `(category, brand, price, qty)`, every column nullable.
    type Row = (Option<&'static str>, Option<&'static str>, Option<f64>, i64);

    fn schema() -> SchemaRef {
        Arc::new(Schema::new(vec![
            Field::new("category", DataType::Utf8View, true),
            Field::new("brand", DataType::Utf8View, true),
            Field::new("price", DataType::Float64, true),
            Field::new("qty", DataType::Int64, true),
        ]))
    }

    fn batch(rows: &[Row]) -> RecordBatch {
        let category = StringViewArray::from(rows.iter().map(|r| r.0).collect::<Vec<_>>());
        let brand = StringViewArray::from(rows.iter().map(|r| r.1).collect::<Vec<_>>());
        let price = Float64Array::from(rows.iter().map(|r| r.2).collect::<Vec<_>>());
        let qty = Int64Array::from(rows.iter().map(|r| r.3).collect::<Vec<_>>());
        RecordBatch::try_new(
            schema(),
            vec![
                Arc::new(category) as ArrayRef,
                Arc::new(brand),
                Arc::new(price),
                Arc::new(qty),
            ],
        )
        .unwrap()
    }

    fn request(spec: Value) -> PdbAggRequest {
        PdbAggRequest::lower(spec, MvccVisibility::default(), &|name| {
            let field_type = match name {
                "category" | "brand" => SearchFieldType::Text(pg_sys::TEXTOID),
                "price" => SearchFieldType::F64(pg_sys::FLOAT8OID),
                "qty" => SearchFieldType::I64(pg_sys::INT8OID),
                other => return Err(format!("no field '{other}'")),
            };
            Ok(PdbAggFieldRef {
                rti: 1,
                attno: 1,
                field_name: name.to_string(),
                field_type,
                plan_position: 0,
                is_array: false,
            })
        })
        .expect("a spec this backend runs")
    }

    /// The document of `spec` over `partitions`, computed the way JoinScan
    /// plans it: one aggregate without group keys. More than one partition
    /// splits the aggregate into a partial and a final stage.
    fn document(spec: Value, partitions: Vec<Vec<RecordBatch>>) -> Value {
        let request = request(spec);
        let call = pdb_agg_call(
            &request,
            |key| col(key.field.field_name.as_str()),
            |field| col(field.field_name.as_str()),
        )
        .unwrap();

        let num_partitions = partitions.len();
        let ctx = SessionContext::new_with_config(
            SessionConfig::new().with_target_partitions(num_partitions),
        );
        let table = MemTable::try_new(schema(), partitions).unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let batches = runtime.block_on(async {
            let df = ctx
                .read_table(Arc::new(table))
                .unwrap()
                .aggregate(vec![], vec![call.alias("document")])
                .unwrap();
            if num_partitions > 1 {
                let plan = df.clone().create_physical_plan().await.unwrap();
                let plan = displayable(plan.as_ref()).indent(false).to_string();
                assert!(
                    plan.contains("mode=Partial") && plan.contains("mode=Final"),
                    "expected a partial and a final stage:\n{plan}"
                );
            }
            df.collect().await.unwrap()
        });

        assert_eq!(batches.iter().map(RecordBatch::num_rows).sum::<usize>(), 1);
        let batch = batches.iter().find(|b| b.num_rows() == 1).unwrap();
        let document = cast(batch.column(0), &DataType::Utf8).unwrap();
        serde_json::from_str(document.as_string::<i32>().value(0)).unwrap()
    }

    fn rows() -> Vec<Vec<Row>> {
        vec![
            vec![
                (Some("shoes"), Some("acme"), Some(10.0), 1),
                (Some("shoes"), Some("zeta"), Some(30.0), 2),
                (Some("hats"), Some("acme"), None, 3),
            ],
            vec![
                (Some("shoes"), Some("acme"), Some(20.0), 4),
                (None, Some("acme"), Some(5.0), 5),
                (Some("hats"), None, Some(7.0), 6),
            ],
        ]
    }

    fn nested_spec() -> Value {
        json!({
            "terms": {"field": "category", "order": {"_key": "asc"}},
            "aggs": {
                "avg_price": {"avg": {"field": "price"}},
                "brands": {
                    "terms": {"field": "brand", "order": {"_key": "asc"}},
                    "aggs": {
                        "total_qty": {"sum": {"field": "qty"}},
                        "prices": {"cardinality": {"field": "price"}}
                    }
                }
            }
        })
    }

    #[test]
    fn nested_terms_with_metrics() {
        let batches: Vec<RecordBatch> = rows().iter().map(|rows| batch(rows)).collect();
        let document = document(nested_spec(), vec![batches]);
        assert_eq!(
            document,
            json!({
                "buckets": [
                    {
                        "key": "hats",
                        "doc_count": 2,
                        "avg_price": {"value": 7.0},
                        "brands": {
                            "buckets": [
                                {
                                    "key": "acme",
                                    "doc_count": 1,
                                    "total_qty": {"value": 3.0},
                                    "prices": {"value": 0.0}
                                },
                                {
                                    "key": null,
                                    "doc_count": 1,
                                    "total_qty": {"value": 6.0},
                                    "prices": {"value": 1.0}
                                }
                            ],
                            "sum_other_doc_count": 0
                        }
                    },
                    {
                        "key": "shoes",
                        "doc_count": 3,
                        "avg_price": {"value": 20.0},
                        "brands": {
                            "buckets": [
                                {
                                    "key": "acme",
                                    "doc_count": 2,
                                    "total_qty": {"value": 5.0},
                                    "prices": {"value": 2.0}
                                },
                                {
                                    "key": "zeta",
                                    "doc_count": 1,
                                    "total_qty": {"value": 2.0},
                                    "prices": {"value": 1.0}
                                }
                            ],
                            "sum_other_doc_count": 0
                        }
                    },
                    {
                        "key": null,
                        "doc_count": 1,
                        "avg_price": {"value": 5.0},
                        "brands": {
                            "buckets": [
                                {
                                    "key": "acme",
                                    "doc_count": 1,
                                    "total_qty": {"value": 5.0},
                                    "prices": {"value": 1.0}
                                }
                            ],
                            "sum_other_doc_count": 0
                        }
                    }
                ],
                "sum_other_doc_count": 0
            })
        );
    }

    /// The partial stage hands its buckets over as state, and the final stage
    /// merges buckets that more than one partition saw.
    #[test]
    fn partial_and_final_stages_agree_with_a_single_pass() {
        let single: Vec<RecordBatch> = rows().iter().map(|rows| batch(rows)).collect();
        let split: Vec<Vec<RecordBatch>> = rows().iter().map(|rows| vec![batch(rows)]).collect();
        for spec in [
            nested_spec(),
            json!({"avg": {"field": "price"}}),
            json!({"cardinality": {"field": "brand"}}),
        ] {
            assert_eq!(
                document(spec.clone(), split.clone()),
                document(spec, vec![single.clone()])
            );
        }
    }

    #[test]
    fn metric_at_the_root() {
        let batches: Vec<RecordBatch> = rows().iter().map(|rows| batch(rows)).collect();
        assert_eq!(
            document(json!({"avg": {"field": "price"}}), vec![batches.clone()]),
            json!({"value": 14.4})
        );
        assert_eq!(
            document(json!({"value_count": {"field": "brand"}}), vec![batches]),
            json!({"value": 5.0})
        );
    }

    #[test]
    fn empty_input_answers_with_the_empty_document() {
        let empty = vec![vec![batch(&[])]];
        assert_eq!(
            document(json!({"avg": {"field": "price"}}), empty.clone()),
            json!({"value": null})
        );
        assert_eq!(
            document(json!({"terms": {"field": "category"}}), empty),
            json!({"buckets": [], "sum_other_doc_count": 0, "doc_count_error_upper_bound": 0})
        );
    }
}
