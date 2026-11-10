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

//! `pdb_string_agg`: PostgreSQL's `string_agg(text, text)` with a separator per row.
//!
//! DataFusion's `string_agg` takes only a literal separator. PostgreSQL puts each
//! non-NULL value after its own row's separator, drops the first separator, skips
//! rows whose value is NULL, and reads a NULL separator as empty.
//!
//! Each row becomes one piece, `[separator length: u32 LE][separator][value]`,
//! collected by DataFusion's `array_agg`, which keeps the ORDER BY keys in its state
//! so a `Partial` / `Final` merge stays ordered. A piece is binary rather than a
//! struct because `array_agg`'s `merge_batch` keeps struct elements as slices that
//! are charged for the whole incoming state.

use std::sync::{Arc, LazyLock};

use arrow_array::builder::BinaryBuilder;
use arrow_array::cast::AsArray;
use arrow_array::{Array, ArrayRef, BinaryArray, StringArray};
use arrow_schema::{DataType, Field, FieldRef};
use datafusion::common::{ScalarValue, internal_err};
use datafusion::error::{DataFusionError, Result};
use datafusion::functions_aggregate::array_agg::ArrayAgg;
use datafusion::logical_expr::expr::{AggregateFunction, Sort};
use datafusion::logical_expr::function::{AccumulatorArgs, StateFieldsArgs};
use datafusion::logical_expr::{
    Accumulator, AggregateUDF, AggregateUDFImpl, Expr, Signature, Volatility, cast,
};

use super::reject_distinct;

pub const STRING_AGG_NAME: &str = "pdb_string_agg";

static STRING_AGG: LazyLock<Arc<AggregateUDF>> =
    LazyLock::new(|| Arc::new(AggregateUDF::from(PgStringAgg::default())));

pub fn string_agg_udaf() -> Arc<AggregateUDF> {
    Arc::clone(&STRING_AGG)
}

/// `pdb_string_agg(value, separator ORDER BY ...)`.
pub fn string_agg(value: Expr, separator: Expr, order_by: Vec<Sort>) -> Expr {
    Expr::AggregateFunction(AggregateFunction::new_udf(
        string_agg_udaf(),
        vec![cast(value, DataType::Utf8), cast(separator, DataType::Utf8)],
        false,
        None,
        order_by,
        None,
    ))
}

fn piece_field() -> FieldRef {
    Arc::new(Field::new("piece", DataType::Binary, true))
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct PgStringAgg {
    signature: Signature,
    array_agg: ArrayAgg,
}

impl Default for PgStringAgg {
    fn default() -> Self {
        Self {
            signature: Signature::exact(
                vec![DataType::Utf8, DataType::Utf8],
                Volatility::Immutable,
            ),
            array_agg: ArrayAgg::default(),
        }
    }
}

impl AggregateUDFImpl for PgStringAgg {
    fn name(&self) -> &str {
        STRING_AGG_NAME
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> Result<DataType> {
        Ok(DataType::LargeUtf8)
    }

    fn state_fields(&self, args: StateFieldsArgs) -> Result<Vec<FieldRef>> {
        self.array_agg.state_fields(StateFieldsArgs {
            input_fields: &[piece_field()],
            ..args
        })
    }

    fn accumulator(&self, args: AccumulatorArgs) -> Result<Box<dyn Accumulator>> {
        reject_distinct(&args, STRING_AGG_NAME)?;
        let pieces = self.array_agg.accumulator(AccumulatorArgs {
            return_field: Arc::new(Field::new_list("pieces", piece_field(), true)),
            exprs: &args.exprs[..1],
            expr_fields: &[piece_field()],
            // A row whose value is NULL contributes nothing, separator included.
            ignore_nulls: true,
            ..args
        })?;
        Ok(Box::new(StringAggAccumulator { pieces }))
    }
}

#[derive(Debug)]
struct StringAggAccumulator {
    pieces: Box<dyn Accumulator>,
}

impl Accumulator for StringAggAccumulator {
    /// `values` holds the value, the separator, then the ORDER BY columns.
    fn update_batch(&mut self, values: &[ArrayRef]) -> Result<()> {
        let [value, separator, ordering @ ..] = values else {
            return internal_err!("{STRING_AGG_NAME} takes a value and a separator");
        };
        let (Some(value), Some(separator)) = (
            value.as_string_opt::<i32>(),
            separator.as_string_opt::<i32>(),
        ) else {
            return internal_err!("{STRING_AGG_NAME} takes Utf8 arguments");
        };
        let mut inputs = vec![Arc::new(encode_pieces(value, separator)) as ArrayRef];
        inputs.extend_from_slice(ordering);
        self.pieces.update_batch(&inputs)
    }

    fn evaluate(&mut self) -> Result<ScalarValue> {
        let ScalarValue::List(list) = self.pieces.evaluate()? else {
            return internal_err!("{STRING_AGG_NAME}: array_agg did not return a list");
        };
        if list.is_null(0) {
            return Ok(ScalarValue::LargeUtf8(None));
        }
        let pieces = list.value(0);
        let Some(pieces) = pieces.as_binary_opt::<i32>() else {
            return internal_err!("{STRING_AGG_NAME}: array_agg did not return binary pieces");
        };
        Ok(ScalarValue::LargeUtf8(join_pieces(pieces)?))
    }

    fn size(&self) -> usize {
        size_of_val(self) - size_of_val(&self.pieces) + self.pieces.size()
    }

    fn state(&mut self) -> Result<Vec<ScalarValue>> {
        self.pieces.state()
    }

    fn merge_batch(&mut self, states: &[ArrayRef]) -> Result<()> {
        self.pieces.merge_batch(states)
    }
}

/// One piece per row, NULL where the value is NULL; a NULL separator is empty.
fn encode_pieces(values: &StringArray, separators: &StringArray) -> BinaryArray {
    let bytes = |a: &StringArray| (a.value_offsets()[a.len()] - a.value_offsets()[0]) as usize;
    let mut pieces = BinaryBuilder::with_capacity(
        values.len(),
        bytes(values) + bytes(separators) + 4 * values.len(),
    );
    let mut piece = Vec::new();
    for (value, separator) in values.iter().zip(separators.iter()) {
        let Some(value) = value else {
            pieces.append_null();
            continue;
        };
        let separator = separator.unwrap_or("");
        piece.clear();
        piece.extend_from_slice(&(separator.len() as u32).to_le_bytes());
        piece.extend_from_slice(separator.as_bytes());
        piece.extend_from_slice(value.as_bytes());
        pieces.append_value(&piece);
    }
    pieces.finish()
}

/// Each value after its own separator, except the first. `None` when there is no piece.
fn join_pieces(pieces: &BinaryArray) -> Result<Option<String>> {
    let mut out: Option<Vec<u8>> = None;
    for piece in pieces.iter().flatten() {
        let Some((separator, value)) = piece
            .split_first_chunk::<4>()
            .and_then(|(len, rest)| rest.split_at_checked(u32::from_le_bytes(*len) as usize))
        else {
            return internal_err!("{STRING_AGG_NAME}: truncated piece");
        };
        match out.as_mut() {
            None => out = Some(value.to_vec()),
            Some(s) => {
                s.extend_from_slice(separator);
                s.extend_from_slice(value);
            }
        }
    }
    out.map(|s| {
        String::from_utf8(s)
            .map_err(|e| DataFusionError::Internal(format!("{STRING_AGG_NAME}: {e}")))
    })
    .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow_array::{Int64Array, RecordBatch};
    use arrow_schema::Schema;
    use datafusion::datasource::MemTable;
    use datafusion::functions_aggregate::array_agg::ArrayAggAccumulator;
    use datafusion::physical_plan::displayable;
    use datafusion::prelude::{SessionConfig, SessionContext, col};

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    #[test]
    fn matches_postgres() {
        let run = |pairs: &[(Option<&str>, Option<&str>)]| {
            let values: StringArray = pairs.iter().map(|p| p.0).collect();
            let separators: StringArray = pairs.iter().map(|p| p.1).collect();
            join_pieces(&encode_pieces(&values, &separators)).unwrap()
        };
        // The issue's example: each value follows its own separator, the first is dropped.
        assert_eq!(
            run(&[
                (Some("x"), Some("-")),
                (Some("y"), Some(";")),
                (Some("z"), Some("|"))
            ]),
            Some("x;y|z".into())
        );
        // A NULL value skips the row, separator included.
        assert_eq!(
            run(&[
                (None, Some("#")),
                (Some("a"), Some("-")),
                (Some("b"), Some("+"))
            ]),
            Some("a+b".into())
        );
        // A NULL separator is empty.
        assert_eq!(
            run(&[
                (Some("a"), Some("-")),
                (Some("b"), None),
                (Some("c"), Some("+"))
            ]),
            Some("ab+c".into())
        );
        // An empty value is still the first value; separators may be multi-byte.
        assert_eq!(
            run(&[
                (Some(""), Some("-")),
                (Some(""), Some("→ ")),
                (Some("é"), Some(""))
            ]),
            Some("→ é".into())
        );
        assert_eq!(run(&[(None, Some("-")), (None, Some("+"))]), None);
        assert_eq!(run(&[]), None);
    }

    /// The accumulator without ORDER BY, as DataFusion builds it.
    fn unordered() -> StringAggAccumulator {
        StringAggAccumulator {
            pieces: Box::new(ArrayAggAccumulator::try_new(&DataType::Binary, true).unwrap()),
        }
    }

    fn strings(values: &[Option<&str>]) -> ArrayRef {
        Arc::new(values.iter().copied().collect::<StringArray>())
    }

    /// Without ORDER BY, merging partial states must keep each value with its own
    /// separator.
    #[test]
    fn unordered_merge_keeps_pairs() {
        let mut first = unordered();
        first
            .update_batch(&[
                strings(&[Some("x"), Some("y")]),
                strings(&[Some("-"), Some(";;")]),
            ])
            .unwrap();
        let mut second = unordered();
        second
            .update_batch(&[
                strings(&[None, Some("z")]),
                strings(&[Some("#"), Some("|||")]),
            ])
            .unwrap();

        let mut merged = unordered();
        for partial in [&mut first, &mut second] {
            let state: Vec<ArrayRef> = partial
                .state()
                .unwrap()
                .into_iter()
                .map(|v| v.to_array().unwrap())
                .collect();
            merged.merge_batch(&state).unwrap();
        }
        assert_eq!(
            merged.evaluate().unwrap(),
            ScalarValue::LargeUtf8(Some("x;;y|||z".into()))
        );
    }

    /// Wrong argument types and corrupt pieces are errors, not panics.
    #[test]
    fn bad_input_is_an_error() {
        let ints: ArrayRef = Arc::new(Int64Array::from(vec![1]));
        assert!(
            unordered()
                .update_batch(&[Arc::clone(&ints), ints])
                .is_err()
        );
        assert!(unordered().update_batch(&[]).is_err());

        let truncated = BinaryArray::from_iter_values([[1u8, 0, 0].as_slice()]);
        assert!(join_pieces(&truncated).is_err());
        let separator_past_end = BinaryArray::from_iter_values([[9u8, 0, 0, 0, b'a'].as_slice()]);
        assert!(join_pieces(&separator_past_end).is_err());
    }

    /// `(g, id, name, sep)` in two partitions with ids interleaved across them, so a
    /// `Final` merge that lost the order would produce a different string.
    fn table() -> MemTable {
        let schema = Arc::new(Schema::new(vec![
            Field::new("g", DataType::Int64, false),
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8View, true),
            Field::new("sep", DataType::Utf8View, true),
        ]));
        let part = |rows: &[(i64, i64, Option<&str>, Option<&str>)]| {
            let names: StringArray = rows.iter().map(|r| r.2).collect();
            let seps: StringArray = rows.iter().map(|r| r.3).collect();
            RecordBatch::try_new(
                Arc::clone(&schema),
                vec![
                    Arc::new(Int64Array::from_iter_values(rows.iter().map(|r| r.0))),
                    Arc::new(Int64Array::from_iter_values(rows.iter().map(|r| r.1))),
                    datafusion::arrow::compute::cast(&names, &DataType::Utf8View).unwrap(),
                    datafusion::arrow::compute::cast(&seps, &DataType::Utf8View).unwrap(),
                ],
            )
            .unwrap()
        };
        MemTable::try_new(
            Arc::clone(&schema),
            vec![
                vec![part(&[
                    (1, 3, Some("c"), Some("+")),
                    (1, 1, None, Some("#")),
                    (2, 6, Some("f"), None),
                ])],
                vec![part(&[
                    (1, 4, Some("d"), Some("|")),
                    (1, 2, Some("b"), Some("-")),
                    (2, 5, Some("e"), Some(";")),
                ])],
            ],
        )
        .unwrap()
    }

    /// Through DataFusion's planner with a `Partial` / `Final` split: the ORDER BY has
    /// to survive the merge of the two partitions' states.
    #[test]
    fn ordered_across_partial_final() {
        runtime().block_on(async {
            let ctx =
                SessionContext::new_with_config(SessionConfig::new().with_target_partitions(2));
            for (asc, expected) in [(true, ["b+c|d", "ef"]), (false, ["d+c-b", "f;e"])] {
                let agg = string_agg(col("name"), col("sep"), vec![col("id").sort(asc, false)]);
                let df = ctx
                    .read_table(Arc::new(table()))
                    .unwrap()
                    .aggregate(vec![col("g")], vec![agg.alias("s")])
                    .unwrap()
                    .sort(vec![col("g").sort(true, false)])
                    .unwrap();

                let plan = df.clone().create_physical_plan().await.unwrap();
                let text = displayable(plan.as_ref()).indent(true).to_string();
                assert!(text.contains("mode=Partial"), "{text}");

                let batches = df.collect().await.unwrap();
                let got: Vec<String> = batches
                    .iter()
                    .flat_map(|b| {
                        let s = b.column(1).as_string::<i64>();
                        (0..s.len())
                            .map(|i| s.value(i).to_owned())
                            .collect::<Vec<_>>()
                    })
                    .collect();
                assert_eq!(got, expected, "asc={asc}");
            }
        });
    }
}
