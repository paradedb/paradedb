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

//! A `topk` aggregate for DataFusion. This effectively wraps SortExec(fetch=K)
//! as a udaf, allowing us to use it alongside other aggregates in AggregateNode

use arrow_array::cast::AsArray;
use arrow_array::{RecordBatch, StructArray};
use arrow_schema::{DataType, Field, FieldRef, Fields, Schema, SchemaRef};
use arrow_select::concat::concat_batches;
use datafusion::common::utils::SingleRowListArrayBuilder;
use datafusion::error::{DataFusionError, Result};
use datafusion::execution::memory_pool::{MemoryPool, UnboundedMemoryPool};
use datafusion::execution::runtime_env::RuntimeEnvBuilder;
use datafusion::execution::{SendableRecordBatchStream, TaskContext};
use datafusion::logical_expr::function::StateFieldsArgs;
use datafusion::logical_expr::utils::AggregateOrderSensitivity;
use datafusion::logical_expr::{
    Accumulator, AggregateUDF, AggregateUDFImpl, Signature, Volatility,
};
use datafusion::physical_expr::{LexOrdering, PhysicalSortExpr};
use datafusion::physical_plan::ExecutionPlan;
use datafusion::physical_plan::expressions::Column;
use datafusion::physical_plan::sorts::sort::SortExec;
use datafusion::physical_plan::stream::RecordBatchStreamAdapter;
use datafusion::physical_plan::streaming::{PartitionStream, StreamingTableExec};
use datafusion::scalar::ScalarValue;
use futures::StreamExt;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use parking_lot::Mutex;
use std::sync::{Arc, LazyLock};
use std::task::{Context, Poll};

use super::{literal_arg, reject_distinct};

pub const TOPK_AS_AGG_NAME: &str = "topk_as_agg";

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

    /// Types only. The planner types the call through `return_field`, which
    /// keeps the real column names; this is the fallback with positional names.
    fn return_type(&self, arg_types: &[arrow_schema::DataType]) -> Result<arrow_schema::DataType> {
        let fields: Vec<FieldRef> = arg_types
            .iter()
            .enumerate()
            .map(|(i, t)| Arc::new(Field::new(format!("c{i}"), t.clone(), true)))
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

    /// The ORDER BY still reaches `accumulator()` either way. Declaring
    /// insensitivity is what stops the planner from putting a SortExec under
    /// the aggregate to satisfy it.
    fn order_sensitivity(&self) -> AggregateOrderSensitivity {
        AggregateOrderSensitivity::Insensitive
    }

    fn accumulator(
        &self,
        acc_args: datafusion::logical_expr::function::AccumulatorArgs,
    ) -> Result<Box<dyn Accumulator>> {
        reject_distinct(&acc_args, TOPK_AS_AGG_NAME)?;
        let payload = payload_fields(acc_args.expr_fields)?;

        let k = match literal_arg(&acc_args, payload.len(), TOPK_AS_AGG_NAME, "k")? {
            ScalarValue::UInt64(Some(k)) if *k > 0 => *k as usize,
            other => {
                return Err(DataFusionError::Internal(format!(
                    "{TOPK_AS_AGG_NAME} k must be a positive UInt64 literal, got {other}"
                )));
            }
        };

        // Only the payload columns reach update_batch, so each sort key must be one
        // of them; rebase the ORDER BY onto payload positions.
        let schema = Arc::new(Schema::new(payload.to_vec()));
        let sort_exprs = acc_args
            .order_bys
            .iter()
            .map(|sort| {
                let i = acc_args.exprs[..payload.len()]
                    .iter()
                    .position(|e| e.as_ref() == sort.expr.as_ref())
                    .ok_or_else(|| {
                        DataFusionError::Internal(format!(
                            "{TOPK_AS_AGG_NAME} ORDER BY {} is not one of its arguments",
                            sort.expr
                        ))
                    })?;
                let column = Column::new(schema.field(i).name(), i);
                Ok(PhysicalSortExpr::new(Arc::new(column), sort.options))
            })
            .collect::<Result<Vec<_>>>()?;
        let ordering = LexOrdering::new(sort_exprs).ok_or_else(|| {
            DataFusionError::Internal(format!("{TOPK_AS_AGG_NAME} requires an ORDER BY"))
        })?;
        Ok(Box::new(TopKAccumulator::new(schema, ordering, k)))
    }
}

/// Every argument but the trailing `k` literal.
fn payload_fields(arg_fields: &[FieldRef]) -> Result<&[FieldRef]> {
    match arg_fields.split_last() {
        Some((_k, payload)) if !payload.is_empty() => Ok(payload),
        _ => Err(DataFusionError::Internal(format!(
            "{TOPK_AS_AGG_NAME} takes at least one payload column and a trailing k",
        ))),
    }
}

/// `List<Struct<payload>>`, with the item field spelled the way
/// `SingleRowListArrayBuilder::build_list_scalar` spells it in `evaluate`.
fn list_of_rows(payload: &[FieldRef]) -> DataType {
    let row = DataType::Struct(Fields::from(payload.to_vec()));
    DataType::List(Arc::new(Field::new_list_field(row, true)))
}

struct TopKAccumulator {
    schema: SchemaRef,
    ordering: LexOrdering,
    k: usize,
    pool: Arc<UnboundedMemoryPool>,
    tx: Option<UnboundedSender<Result<RecordBatch>>>,
    stream: Mutex<Option<SendableRecordBatchStream>>, // Accumulator is Send + Sync
}
/// Manual because `SendableRecordBatchStream` is not `Debug`.
impl std::fmt::Debug for TopKAccumulator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TopKAccumulator")
            .field("k", &self.k)
            .field("ordering", &self.ordering)
            .field("open", &self.tx.is_some())
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
struct ChannelPartition {
    schema: SchemaRef,
    /// Rx needs to be owned by the stream adapter at execution time, but must be owned by
    /// ChannelPartition until then. Mutex<Option<..>> is a do that while still safely being Send +
    /// Sync
    rx: Mutex<Option<UnboundedReceiver<Result<RecordBatch>>>>,
}
impl PartitionStream for ChannelPartition {
    fn schema(&self) -> &SchemaRef {
        &self.schema
    }
    fn execute(&self, _ctx: Arc<TaskContext>) -> SendableRecordBatchStream {
        let rx = self
            .rx
            .lock()
            .take()
            .expect("SortExec executes its input once");
        Box::pin(RecordBatchStreamAdapter::new(Arc::clone(&self.schema), rx))
    }
}

impl TopKAccumulator {
    pub fn new(schema: SchemaRef, ordering: LexOrdering, k: usize) -> Self {
        Self {
            schema,
            ordering,
            k,
            pool: Arc::new(UnboundedMemoryPool::default()),
            tx: None,
            stream: Mutex::new(None),
        }
    }

    fn open(&mut self) -> Result<()> {
        let (tx, rx) = futures::channel::mpsc::unbounded();
        let single_input_partition = ChannelPartition {
            schema: self.schema.clone(),
            rx: Mutex::new(Some(rx)),
        };
        // StreamingTableExec is the simplest way to have an ExecutionPlan for an arbitrary batch of
        // data. SortExec requires an ExecutionPlan as input.
        let input = StreamingTableExec::try_new(
            self.schema.clone(),
            vec![Arc::new(single_input_partition)],
            None,
            [],
            false,
            None,
        )?;
        let sort_exec =
            SortExec::new(self.ordering.clone(), Arc::new(input)).with_fetch(Some(self.k));
        let runtime = RuntimeEnvBuilder::new()
            .with_memory_pool(Arc::clone(&self.pool) as Arc<dyn MemoryPool>)
            .build_arc()?;
        let ctx = Arc::new(TaskContext::default().with_runtime(runtime));
        *self.stream.get_mut() = Some(sort_exec.execute(0, ctx)?);
        self.tx = Some(tx);
        Ok(())
    }

    /// Process the latest batch, expecting no output batches.
    fn pump(&mut self) -> Result<()> {
        let waker = futures::task::noop_waker();
        let mut cx = Context::from_waker(&waker);
        let stream = self
            .stream
            .get_mut()
            .as_mut()
            .expect("stream should already be opened");
        if stream.poll_next_unpin(&mut cx).is_pending() {
            Ok(())
        } else {
            Err(DataFusionError::Internal(
                "TopKAccumulator recieved an output batch sooner than expected. This likely represents a change in the behavior of SortExec.".to_string()
            ))
        }
    }

    /// Process the remainder of the stream, expecting output batches now.
    fn drain(&mut self, out: &mut Vec<RecordBatch>) -> Result<()> {
        let waker = futures::task::noop_waker();
        let mut cx = Context::from_waker(&waker);
        let stream = self
            .stream
            .get_mut()
            .as_mut()
            .expect("stream should already be opened");
        while let Poll::Ready(Some(batch)) = stream.poll_next_unpin(&mut cx) {
            out.push(batch?);
        }
        Ok(())
    }

    fn push(&mut self, batch: RecordBatch) -> Result<()> {
        if self.tx.is_none() {
            self.open()?;
        }
        self.tx
            .as_ref()
            .expect("opened above")
            .unbounded_send(Ok(batch))
            .map_err(|e| DataFusionError::Internal(e.to_string()))?;
        self.pump()
    }
}

impl Accumulator for TopKAccumulator {
    fn update_batch(&mut self, values: &[arrow_array::ArrayRef]) -> Result<()> {
        let n = self.schema.fields().len();
        let batch = RecordBatch::try_new(Arc::clone(&self.schema), values[..n].to_vec())?;
        self.push(batch)
    }

    fn evaluate(&mut self) -> Result<datafusion::scalar::ScalarValue> {
        let mut batches = vec![];
        if self.tx.take().is_some() {
            // dropping the sender ends the input
            self.drain(&mut batches)?; // TopK::emit() runs on this poll 
            *self.stream.get_mut() = None;
        }
        let batch = concat_batches(&self.schema, &batches)?;
        Ok(SingleRowListArrayBuilder::new(Arc::new(StructArray::from(batch))).build_list_scalar())
    }

    fn size(&self) -> usize {
        size_of_val(self) + self.pool.reserved()
    }

    fn state(&mut self) -> Result<Vec<datafusion::scalar::ScalarValue>> {
        Ok(vec![self.evaluate()?])
    }

    fn merge_batch(&mut self, states: &[arrow_array::ArrayRef]) -> Result<()> {
        // One list per partial state row, holding that partial's K rows as a struct array.
        for rows in states[0].as_list::<i32>().iter().flatten() {
            if rows.is_empty() {
                continue;
            }
            let columns = rows.as_struct().columns().to_vec();
            self.push(RecordBatch::try_new(Arc::clone(&self.schema), columns)?)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_array::types::Int64Type;
    use arrow_array::{Array, ArrayRef, Int64Array, StringArray};
    use arrow_schema::SortOptions;
    use datafusion::datasource::MemTable;
    use datafusion::logical_expr::expr::AggregateFunction;
    use datafusion::physical_plan::displayable;
    use datafusion::prelude::{Expr, SessionConfig, SessionContext, col, lit};

    /// `(score, id)`: score nullable, id not.
    fn schema() -> SchemaRef {
        Arc::new(Schema::new(vec![
            Field::new("score", DataType::Int64, true),
            Field::new("id", DataType::Int64, false),
        ]))
    }

    fn batch(rows: &[(Option<i64>, i64)]) -> RecordBatch {
        let score = Int64Array::from(rows.iter().map(|r| r.0).collect::<Vec<_>>());
        let id = Int64Array::from(rows.iter().map(|r| r.1).collect::<Vec<_>>());
        RecordBatch::try_new(schema(), vec![Arc::new(score) as ArrayRef, Arc::new(id)]).unwrap()
    }

    /// `ORDER BY score DESC NULLS FIRST, id ASC`.
    fn accumulator(k: usize) -> TopKAccumulator {
        let ordering = LexOrdering::new(vec![
            PhysicalSortExpr::new(
                Arc::new(Column::new("score", 0)),
                SortOptions {
                    descending: true,
                    nulls_first: true,
                },
            ),
            PhysicalSortExpr::new(
                Arc::new(Column::new("id", 1)),
                SortOptions {
                    descending: false,
                    nulls_first: false,
                },
            ),
        ])
        .unwrap();
        TopKAccumulator::new(schema(), ordering, k)
    }

    /// Reads a single `List<Struct<score, id>>` scalar back as rows.
    fn rows_of(value: &ScalarValue) -> Vec<(Option<i64>, i64)> {
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

    /// Runs the UDAF through DataFusion's own planner and AggregateExec: the ORDER BY
    /// must not become a SortExec under the aggregate, the Partial/Final split must
    /// round-trip the state, and the declared return type must match what
    /// `evaluate` emits.
    #[test]
    fn plans_without_a_sort_and_splits_partial_final() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
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
            let topk = Expr::AggregateFunction(AggregateFunction::new_udf(
                topk_as_agg_udaf(),
                vec![col("score"), col("id"), lit(2u64)],
                false,
                None,
                vec![col("score").sort(false, true), col("id").sort(true, false)],
                None,
            ));
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
            let batch = concat_batches(&batches[0].schema(), &batches).unwrap();
            let groups = batch.column(0).as_string::<i32>();
            let lists = batch.column(1).as_list::<i32>();
            #[allow(clippy::type_complexity)]
            let mut rows: Vec<(String, Vec<(Option<i64>, i64)>)> = (0..batch.num_rows())
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
}
