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

//! Loads what `paradedb.vector_router_export` wrote and runs tantivy's router audit over it.
//! `run.sh` documents the inputs; the audit itself is `tantivy::vector::router_audit`.

use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use tantivy::schema::Metric;
use tantivy::vector::clustering::SuperKMeansClusterer;
use tantivy::vector::router_audit::{
    AuditInput, OnlinePoint, Phases, default_ks, offline_recall, rebuild_check_stops, run_audit,
};

#[derive(Parser)]
#[command(about = "IVF router quality audit over a pg_search router export")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Phases A and B: writes the audit report.
    Run(RunArgs),
    /// The stored router's coverage recall at online operating points.
    Online(OnlineArgs),
}

#[derive(clap::Args)]
struct Inputs {
    /// Directory `paradedb.vector_router_export` wrote.
    #[arg(long)]
    export: PathBuf,
    /// Tab-separated `ctid`, `id` lines, one per table row.
    #[arg(long)]
    ids: PathBuf,
    /// JSON lines of `{query_id, set, emb, gt_ids}`.
    #[arg(long)]
    queries: PathBuf,
}

#[derive(clap::Args)]
struct RunArgs {
    #[command(flatten)]
    inputs: Inputs,
    /// Where the report goes.
    #[arg(long)]
    output: PathBuf,
    /// Skip the structure and rule sweep.
    #[arg(long)]
    skip_phase_b: bool,
}

#[derive(clap::Args)]
struct OnlineArgs {
    #[command(flatten)]
    inputs: Inputs,
    /// The operating points `collect_perf.py` wrote.
    #[arg(long)]
    points: PathBuf,
    /// Query set the points were measured on.
    #[arg(long, default_value = "held_out")]
    set: String,
    /// Where the recalls go.
    #[arg(long)]
    output: PathBuf,
}

/// `meta.json`.
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Meta {
    dim: usize,
    metric: Metric,
    clusters: usize,
    docs: u64,
    router_kind: String,
    max_leaf_size: usize,
    training_sample_ratio: f32,
    quantization: Option<Vec<u8>>,
    spill: bool,
    segment_id: String,
}

#[derive(Deserialize)]
struct Query {
    query_id: i64,
    set: String,
    emb: Vec<f32>,
    gt_ids: Vec<String>,
}

/// One point of `collect_perf.py`'s output: per-query values in query-id order.
#[derive(Deserialize)]
struct Point {
    label: String,
    rerank_factor: f32,
    query_ids: Vec<i64>,
    router_k: Vec<usize>,
    clusters: Vec<usize>,
}

#[derive(Serialize)]
struct OnlineRecall {
    label: String,
    offline_recall: f64,
}

/// The `u64` a `(block,offset)` ctid is indexed under.
fn ctid_key(text: &str) -> Result<u64> {
    let (block, offset) = text
        .trim_matches(|c| c == '(' || c == ')')
        .split_once(',')
        .with_context(|| format!("malformed ctid {text}"))?;
    Ok((block.parse::<u64>()? << 16) | offset.parse::<u64>()?)
}

fn read_u32s(path: &Path) -> Result<Vec<u32>> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let (words, _) = bytes.as_chunks::<4>();
    Ok(words.iter().copied().map(u32::from_le_bytes).collect())
}

fn read_f32s(path: &Path) -> Result<Vec<f32>> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let (words, _) = bytes.as_chunks::<4>();
    Ok(words.iter().copied().map(f32::from_le_bytes).collect())
}

/// Each table id's cluster, from the export's `(ctid, cluster)` rows and the table's ids.
fn id_clusters(export: &Path, ids: &Path, meta: &Meta) -> Result<HashMap<String, u32>> {
    let bytes = fs::read(export.join("doc_clusters"))?;
    let (rows, _) = bytes.as_chunks::<12>();
    let ctid_clusters: HashMap<u64, u32> = rows
        .iter()
        .map(|row| {
            (
                u64::from_le_bytes(row[..8].try_into().expect("eight bytes")),
                u32::from_le_bytes(row[8..].try_into().expect("four bytes")),
            )
        })
        .collect();
    ensure!(
        ctid_clusters.len() as u64 == meta.docs,
        "doc_clusters holds {} documents, meta.json {}",
        ctid_clusters.len(),
        meta.docs
    );
    let mut clusters = HashMap::with_capacity(ctid_clusters.len());
    let mut table_rows = 0u64;
    for line in BufReader::new(fs::File::open(ids)?).lines() {
        let line = line?;
        let (ctid, id) = line
            .split_once('\t')
            .with_context(|| format!("malformed id line {line}"))?;
        table_rows += 1;
        if let Some(&cluster) = ctid_clusters.get(&ctid_key(ctid)?) {
            clusters.insert(id.to_string(), cluster);
        }
    }
    ensure!(
        table_rows == meta.docs,
        "the table holds {table_rows} rows, the doc-cluster map {}",
        meta.docs
    );
    Ok(clusters)
}

/// The audit input and the query ids, in file order.
fn load(inputs: &Inputs) -> Result<(AuditInput, Vec<i64>, Meta)> {
    let meta: Meta = serde_json::from_slice(&fs::read(inputs.export.join("meta.json"))?)?;
    let sizes = read_u32s(&inputs.export.join("sizes.u32"))?;
    ensure!(
        sizes.len() == meta.clusters,
        "sizes.u32 holds {} clusters, meta.json {}",
        sizes.len(),
        meta.clusters
    );
    let id_clusters = id_clusters(&inputs.export, &inputs.ids, &meta)?;
    let mut queries = Vec::new();
    let mut gt_clusters = Vec::new();
    let mut query_set = Vec::new();
    let mut query_ids = Vec::new();
    for line in BufReader::new(fs::File::open(&inputs.queries)?).lines() {
        let query: Query = serde_json::from_str(&line?)?;
        ensure!(
            query.emb.len() == meta.dim,
            "query {} has {} dimensions",
            query.query_id,
            query.emb.len()
        );
        let clusters = query
            .gt_ids
            .iter()
            .map(|id| {
                id_clusters
                    .get(id)
                    .copied()
                    .with_context(|| format!("ground-truth id {id} has no cluster"))
            })
            .collect::<Result<Vec<u32>>>()?;
        queries.push(query.emb);
        gt_clusters.push(clusters);
        query_set.push(query.set);
        query_ids.push(query.query_id);
    }
    let input = AuditInput {
        dim: meta.dim,
        metric: meta.metric,
        centroids: read_f32s(&inputs.export.join("centroids.f32"))?,
        router_slot: fs::read(inputs.export.join("router.bin"))?,
        ks: default_ks(sizes.len()),
        sizes,
        queries,
        gt_clusters,
        query_set,
        clusterer: SuperKMeansClusterer::new()
            .with_max_leaf_size(meta.max_leaf_size)
            .with_training_sample_ratio(meta.training_sample_ratio),
    };
    Ok((input, query_ids, meta))
}

fn run(args: &RunArgs) -> Result<()> {
    let (input, _, meta) = load(&args.inputs)?;
    let report = run_audit(
        &input,
        Phases {
            a: true,
            b: !args.skip_phase_b,
        },
    )?;
    let document = serde_json::json!({ "meta": meta, "report": report });
    fs::write(&args.output, serde_json::to_vec_pretty(&document)?)?;

    let mut failures = Vec::new();
    if rebuild_check_stops(&report.rebuild_check) {
        failures.push(
            "stored router above its rebuilds' 95th percentile at every held-out, factor 2, APS on \
             target"
                .to_string(),
        );
    }
    if report
        .phase_b
        .as_ref()
        .is_some_and(|phase_b| !phase_b.s_reproduces_k1)
    {
        failures.push("S(2, 0.25, on) did not reproduce K1's output".to_string());
    }
    if !failures.is_empty() {
        bail!(
            "stop conditions tripped; report written to {}:\n{}",
            args.output.display(),
            failures.join("\n")
        );
    }
    Ok(())
}

fn online(args: &OnlineArgs) -> Result<()> {
    let (mut input, query_ids, _) = load(&args.inputs)?;
    let keep: Vec<usize> = (0..query_ids.len())
        .filter(|&q| input.query_set[q] == args.set)
        .collect();
    let kept_ids: Vec<i64> = keep.iter().map(|&q| query_ids[q]).collect();
    input.queries = keep
        .iter()
        .map(|&q| std::mem::take(&mut input.queries[q]))
        .collect();
    input.gt_clusters = keep
        .iter()
        .map(|&q| std::mem::take(&mut input.gt_clusters[q]))
        .collect();
    input.query_set = vec![args.set.clone(); keep.len()];

    let points: Vec<Point> = serde_json::from_slice(&fs::read(&args.points)?)?;
    for point in &points {
        ensure!(
            point.query_ids == kept_ids,
            "point {} measured other queries than {}",
            point.label,
            args.set
        );
    }
    let online: Vec<OnlinePoint> = points
        .iter()
        .map(|point| OnlinePoint {
            rerank_factor: point.rerank_factor,
            router_k: point.router_k.clone(),
            clusters: point.clusters.clone(),
        })
        .collect();
    let recalls = offline_recall(&input, &online)?;
    let rows: Vec<OnlineRecall> = points
        .iter()
        .zip(recalls)
        .map(|(point, offline_recall)| OnlineRecall {
            label: point.label.clone(),
            offline_recall,
        })
        .collect();
    fs::write(&args.output, serde_json::to_vec_pretty(&rows)?)?;
    Ok(())
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Run(args) => run(&args),
        Command::Online(args) => online(&args),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ctid_keys_match_the_indexed_u64() {
        assert_eq!(ctid_key("(0,1)").unwrap(), 1);
        assert_eq!(ctid_key("(3,7)").unwrap(), (3 << 16) | 7);
        assert!(ctid_key("(3)").is_err());
    }
}
