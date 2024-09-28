// Copyright (c) 2023-2024 Retake, Inc.
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

mod bench_hits;
mod benchmark;
mod cli;
mod elastic;
mod subcommand;
mod tables;

use anyhow::Result;
use async_std::task::block_on;
use cli::{Cli, Corpus, EsLogsCommand, HitsCommand, PgaBenchCommand, Subcommand};
use dotenvy::dotenv;
use tracing_subscriber::EnvFilter;

fn handle_pga_bench_command(pga_bench_args: cli::PgaBenchArgs) -> Result<()> {
    tracing::info!("Handling pga-bench command: {:?}", pga_bench_args);
    match pga_bench_args.command {
        PgaBenchCommand::ParquetRunAll => subcommand::bench_parquet_run_all(),
    }
}

fn handle_hits_command(hits: cli::HitsArgs) -> Result<()> {
    match hits.command {
        HitsCommand::Run {
            workload,
            url,
            full,
        } => block_on(bench_hits::bench_hits(&url, &workload, full)),
    }
}

fn handle_eslogs_command(eslogs: cli::EsLogsArgs) -> Result<()> {
    use EsLogsCommand::*;

    match eslogs.command {
        Generate {
            seed,
            events,
            table,
            url,
        } => block_on(subcommand::bench_eslogs_generate(seed, events, table, url)),
        BuildSearchIndex { table, index, url } => block_on(
            subcommand::bench_eslogs_build_search_index(table, index, url),
        ),
        QuerySearchIndex {
            index,
            query,
            limit,
            url,
        } => block_on(subcommand::bench_eslogs_query_search_index(
            index, query, limit, url,
        )),
        BuildGinIndex { table, index, url } => {
            block_on(subcommand::bench_eslogs_build_gin_index(table, index, url))
        }
        QueryGinIndex {
            table,
            query,
            limit,
            url,
        } => block_on(subcommand::bench_eslogs_query_gin_index(
            table, query, limit, url,
        )),
        BuildParquetTable { table, url } => {
            block_on(subcommand::bench_eslogs_build_parquet_table(table, url))
        }
        CountParquetTable { table, url } => {
            block_on(subcommand::bench_eslogs_count_parquet_table(table, url))
        }
        BuildElasticIndex {
            table,
            url,
            elastic_url,
        } => block_on(subcommand::bench_eslogs_build_elastic_table(
            url,
            table,
            elastic_url,
        )),
        QueryElasticIndex {
            elastic_url,
            field,
            term,
        } => block_on(subcommand::bench_eslogs_query_elastic_table(
            elastic_url,
            field,
            term,
        )),
    }
}

fn handle_bench_command(bench: cli::CorpusArgs) -> Result<()> {
    match bench.corpus {
        Corpus::Eslogs(eslogs) => handle_eslogs_command(eslogs),
        Corpus::Hits(hits) => handle_hits_command(hits),
    }
}

fn setup_logging() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init()
        .map_err(|e| anyhow::anyhow!("Failed to initialize logging: {}", e))
}

fn main() -> Result<()> {
    setup_logging()?;
    dotenv().ok();

    let cli = Cli::default();
    tracing::info!("Parsed CLI structure: {:#?}", cli);

    let _ = match cli.command {
        cli::Commands::Paradedb(paradedb_cmd) => {
            tracing::info!("Handling Paradedb command with args: {:#?}", paradedb_cmd);
            match paradedb_cmd.subcommand {
                Subcommand::Install => subcommand::install(),
                Subcommand::Bench(bench_args) => handle_bench_command(bench_args),
                Subcommand::PgaBench(pga_bench_args) => handle_pga_bench_command(pga_bench_args),
            }
        }
    };

    Ok(())
}
