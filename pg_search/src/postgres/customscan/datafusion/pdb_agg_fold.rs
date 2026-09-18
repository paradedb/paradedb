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

//! An aggregate that folds the bucket rows of a `pdb.agg()` grouping-set plan
//! into the finished JSON document, inside the plan.
//!
//! The aggregate scan folds those rows in Rust after draining the stream. A
//! JoinScan window has to carry the document as a column on every joined row,
//! so the fold runs as an aggregate over the bucket rows and its one-row result
//! is joined back onto the rows. The accumulator only buffers rows; the
//! assembler that finalizes them is the aggregate scan's own.

use std::mem::size_of_val;
use std::sync::{Arc, LazyLock};

use arrow_array::cast::AsArray;
use arrow_array::{ArrayRef, RecordBatch};
use arrow_schema::{DataType, Field, FieldRef, Schema, SchemaRef};
use datafusion::arrow::ipc::reader::StreamReader;
use datafusion::arrow::ipc::writer::StreamWriter;
use datafusion::common::ScalarValue;
use datafusion::error::{DataFusionError, Result};
use datafusion::logical_expr::function::{AccumulatorArgs, StateFieldsArgs};
use datafusion::logical_expr::{
    Accumulator, AggregateUDF, AggregateUDFImpl, Expr, Signature, Volatility, lit,
};

use super::{literal_arg, reject_distinct};
use crate::postgres::customscan::aggregatescan::pdb_agg::{
    PdbAggPlan, PdbAggRequest, assemble_pdb_agg_rows,
};

pub const PDB_AGG_FOLD_NAME: &str = "pdb_agg_fold";

static PDB_AGG_FOLD: LazyLock<Arc<AggregateUDF>> =
    LazyLock::new(|| Arc::new(AggregateUDF::from(PdbAggFold::new())));

pub fn pdb_agg_fold_udaf() -> Arc<AggregateUDF> {
    Arc::clone(&PDB_AGG_FOLD)
}

/// Resolve the UDAF by name, for the plan codecs.
pub fn udaf_by_name(name: &str) -> Option<Arc<AggregateUDF>> {
    (name == PDB_AGG_FOLD_NAME).then(pdb_agg_fold_udaf)
}

/// The leading literal arguments of a call: the requests and the entry index.
/// The bucket columns follow.
const LITERAL_ARGS: usize = 2;

/// The requests the bucket rows were built for, as a call argument. JSON
/// rather than postcard: a request carries its spec as `serde_json::Value` and
/// as Tantivy's flattened `Aggregation`, which need a self-describing format.
/// Dictionary-encoded, so materializing the literal per bucket row costs a key
/// rather than a copy of the requests.
pub fn requests_literal(requests: &[&PdbAggRequest]) -> Result<Expr> {
    let bytes = serde_json::to_vec(requests).map_err(|e| DataFusionError::External(Box::new(e)))?;
    Ok(lit(ScalarValue::Dictionary(
        Box::new(DataType::Int32),
        Box::new(ScalarValue::Binary(Some(bytes))),
    )))
}

/// `pdb_agg_fold(requests, entry, bucket columns...)`: the document of the
/// `entry`-th request, from the bucket rows laid out by the `PdbAggPlan` built
/// over `requests` with no SQL group keys and no standard aggregates.
pub fn pdb_agg_fold(
    requests: Expr,
    entry: usize,
    bucket_columns: impl IntoIterator<Item = Expr>,
) -> Expr {
    let mut args = vec![requests, lit(entry as u32)];
    args.extend(bucket_columns);
    pdb_agg_fold_udaf().call(args)
}

fn requests_from_args(args: &AccumulatorArgs) -> Result<Vec<PdbAggRequest>> {
    let value = literal_arg(args, 0, PDB_AGG_FOLD_NAME, "requests")?;
    let bytes = match value {
        ScalarValue::Dictionary(_, inner) => match inner.as_ref() {
            ScalarValue::Binary(Some(bytes)) => bytes,
            other => {
                return Err(DataFusionError::Internal(format!(
                    "{PDB_AGG_FOLD_NAME} requests must be a non-null Binary literal, got {other}"
                )));
            }
        },
        ScalarValue::Binary(Some(bytes)) => bytes,
        other => {
            return Err(DataFusionError::Internal(format!(
                "{PDB_AGG_FOLD_NAME} requests must be a non-null Binary literal, got {other}"
            )));
        }
    };
    serde_json::from_slice(bytes).map_err(|e| {
        DataFusionError::Internal(format!("{PDB_AGG_FOLD_NAME} requests do not decode: {e}"))
    })
}

fn entry_from_args(args: &AccumulatorArgs) -> Result<usize> {
    match literal_arg(args, 1, PDB_AGG_FOLD_NAME, "entry")? {
        ScalarValue::UInt32(Some(entry)) => Ok(*entry as usize),
        other => Err(DataFusionError::Internal(format!(
            "{PDB_AGG_FOLD_NAME} entry must be a non-null UInt32 literal, got {other}"
        ))),
    }
}

struct FoldAccumulator {
    plan: PdbAggPlan,
    entry: usize,
    /// The bucket columns, in the plan's column order.
    schema: SchemaRef,
    batches: Vec<RecordBatch>,
}

impl std::fmt::Debug for FoldAccumulator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FoldAccumulator")
    }
}

impl Accumulator for FoldAccumulator {
    fn update_batch(&mut self, values: &[ArrayRef]) -> Result<()> {
        let columns = values.get(LITERAL_ARGS..).unwrap_or_default().to_vec();
        self.batches
            .push(RecordBatch::try_new(Arc::clone(&self.schema), columns)?);
        Ok(())
    }

    /// Runs on empty input too: a scalar aggregate answers with one row, and the
    /// assembler synthesizes the root row the grouping sets never emitted.
    fn evaluate(&mut self) -> Result<ScalarValue> {
        let assembled = assemble_pdb_agg_rows(
            Arc::clone(&self.schema),
            &self.batches,
            &self.plan,
            Some(&[]),
        )?;
        let document = assembled
            .json
            .into_iter()
            .next()
            .and_then(|mut documents| documents.get_mut(self.entry).map(serde_json::Value::take))
            .ok_or_else(|| {
                DataFusionError::Internal(format!(
                    "{PDB_AGG_FOLD_NAME}: no document for entry {}",
                    self.entry
                ))
            })?;
        // The document is constant across the rows it joins onto, so it is
        // broadcast as one dictionary value and a key per row.
        Ok(ScalarValue::Dictionary(
            Box::new(DataType::Int32),
            Box::new(ScalarValue::Utf8(Some(document.to_string()))),
        ))
    }

    fn size(&self) -> usize {
        size_of_val(self)
            + self
                .batches
                .iter()
                .map(RecordBatch::get_array_memory_size)
                .sum::<usize>()
    }

    /// The buffered rows as one Arrow IPC stream. The fold sits above a
    /// single-partition aggregate, so this only runs when a distributed plan
    /// splits the aggregate into partial and final stages.
    fn state(&mut self) -> Result<Vec<ScalarValue>> {
        let mut bytes = Vec::new();
        let mut writer = StreamWriter::try_new(&mut bytes, &self.schema)?;
        for batch in &self.batches {
            writer.write(batch)?;
        }
        writer.finish()?;
        Ok(vec![ScalarValue::Binary(Some(bytes))])
    }

    fn merge_batch(&mut self, states: &[ArrayRef]) -> Result<()> {
        for bytes in states[0].as_binary::<i32>().iter().flatten() {
            for batch in StreamReader::try_new(bytes, None)? {
                self.batches.push(batch?);
            }
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct PdbAggFold {
    signature: Signature,
}

impl PdbAggFold {
    fn new() -> Self {
        Self {
            signature: Signature::variadic_any(Volatility::Immutable),
        }
    }
}

impl AggregateUDFImpl for PdbAggFold {
    fn name(&self) -> &str {
        PDB_AGG_FOLD_NAME
    }

    fn signature(&self) -> &Signature {
        &self.signature
    }

    fn return_type(&self, _arg_types: &[DataType]) -> Result<DataType> {
        Ok(DataType::Dictionary(
            Box::new(DataType::Int32),
            Box::new(DataType::Utf8),
        ))
    }

    fn accumulator(&self, args: AccumulatorArgs) -> Result<Box<dyn Accumulator>> {
        reject_distinct(&args, PDB_AGG_FOLD_NAME)?;
        let requests = requests_from_args(&args)?;
        let entry = entry_from_args(&args)?;
        if entry >= requests.len() {
            return Err(DataFusionError::Internal(format!(
                "{PDB_AGG_FOLD_NAME} entry {entry} is out of range for {} requests",
                requests.len()
            )));
        }
        // The same layout the bucket rows were built with: the entry index only
        // identifies a FILTER, and the window path has none.
        let entries: Vec<(usize, &PdbAggRequest, bool)> = requests
            .iter()
            .enumerate()
            .map(|(i, request)| (i, request, false))
            .collect();
        let plan = PdbAggPlan::build(&entries, 0, 0)?;
        let fields: Vec<FieldRef> = args
            .expr_fields
            .get(LITERAL_ARGS..)
            .unwrap_or_default()
            .to_vec();
        Ok(Box::new(FoldAccumulator {
            plan,
            entry,
            schema: Arc::new(Schema::new(fields)),
            batches: Vec::new(),
        }))
    }

    fn state_fields(&self, args: StateFieldsArgs) -> Result<Vec<FieldRef>> {
        Ok(vec![
            Field::new(format!("{}[rows]", args.name), DataType::Binary, true).into(),
        ])
    }
}
