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
use clap::Parser;
use std::env;

const DEFAULT_BENCH_ESLOGS_TABLE: &str = "benchmark_eslogs";
const DEFAULT_BENCH_ESLOGS_INDEX_NAME: &str = "benchmark_eslogs_pg_search";

#[derive(Debug, Parser)]
#[command(bin_name = "cargo-paradedb")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, clap::Subcommand)]
pub enum Commands {
    Paradedb(ParadedbCommands),
}

#[derive(Debug, clap::Parser)]
pub struct ParadedbCommands {
    #[command(subcommand)]
    pub subcommand: Subcommand,
}

// Top-level commands for the cargo-paradedb tool.
#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
    Install,
    Bench(CorpusArgs),
    PgaBench(PgaBenchArgs),
}

// A wrapper struct for a subcommand under 'cargo paradedb bench' which
// select a corpus to generate/run.
#[derive(Debug, clap::Parser)]
pub struct CorpusArgs {
    #[command(subcommand)]
    pub corpus: Corpus,
}

/// Which benchmark dataset to run or generate.
#[derive(Debug, clap::Subcommand)]
pub enum Corpus {
    // The generated logs from the ElasticSearch benchmark tool.
    Eslogs(EsLogsArgs),
    Hits(HitsArgs),
}

/// A wrapper struct for the command to run on the eslogs corpus.
#[derive(Debug, clap::Parser)]
pub struct EsLogsArgs {
    #[command(subcommand)]
    pub command: EsLogsCommand,
}

/// A wrapper struct for the command to run on the hits corpus.
#[derive(Debug, clap::Parser)]
pub struct HitsArgs {
    #[command(subcommand)]
    pub command: HitsCommand,
}

/// Wrapper for pg_analytics benchmark subcommands.
#[derive(Debug, clap::Parser)]
pub struct PgaBenchArgs {
    #[command(subcommand)]
    pub command: PgaBenchCommand,
}

/// Runs pg_analytics benchmark sub-commands
#[derive(Debug, Clone, clap::Subcommand)]
pub enum PgaBenchCommand {
    ParquetRunAll,
}

/// The command to run on the eslogs corpus.
#[derive(Debug, clap::Subcommand)]
pub enum EsLogsCommand {
    /// Generate the eslogs corpus, inserting into a Postgres table.
    Generate {
        /// Starting seed for random generation.
        #[arg(long, short, default_value_t = 1)]
        seed: u64,
        /// Total number of events to generate per file.
        /// Defaults to a file size of 100MB.
        #[arg(long, short, default_value_t = 118891)]
        events: u64,
        /// Postgres table name to insert into.
        #[arg(short, long, default_value = DEFAULT_BENCH_ESLOGS_TABLE)]
        table: String,
        /// Postgres database url to connect to.
        #[arg(short, long, env = "DATABASE_URL")]
        url: String,
    },
    BuildSearchIndex {
        /// Postgres table name to index.
        #[arg(short, long, default_value = DEFAULT_BENCH_ESLOGS_TABLE)]
        table: String,
        /// Postgres table name to index.
        #[arg(short, long, default_value = DEFAULT_BENCH_ESLOGS_INDEX_NAME)]
        index: String,
        /// Postgres database url to connect to.
        #[arg(short, long, env = "DATABASE_URL")]
        url: String,
    },
    QuerySearchIndex {
        /// Postgres index name to query.
        #[arg(short, long, default_value = DEFAULT_BENCH_ESLOGS_INDEX_NAME)]
        index: String,
        /// Query to run.
        #[arg(short, long, default_value = "message:flame")]
        query: String,
        /// Limit results to return.
        #[arg(short, long, default_value_t = 1)]
        limit: u64,
        /// Postgres database url to connect to.
        #[arg(short, long, env = "DATABASE_URL")]
        url: String,
    },
    BuildGinIndex {
        /// Postgres table name to index.
        #[arg(short, long, default_value = DEFAULT_BENCH_ESLOGS_TABLE)]
        table: String,
        /// Postgres table name to index.
        #[arg(short, long, default_value = DEFAULT_BENCH_ESLOGS_INDEX_NAME)]
        index: String,
        /// Postgres database url to connect to.
        #[arg(short, long, env = "DATABASE_URL")]
        url: String,
    },
    QueryGinIndex {
        /// Postgres index name to query.
        #[arg(short, long, default_value = DEFAULT_BENCH_ESLOGS_TABLE)]
        table: String,
        /// Query to run.
        #[arg(short, long, default_value = "flame")]
        query: String,
        /// Limit results to return.
        #[arg(short, long, default_value_t = 1)]
        limit: u64,
        /// Postgres database url to connect to.
        #[arg(short, long, env = "DATABASE_URL")]
        url: String,
    },
    BuildParquetTable {
        /// Postgres table name to build from.
        #[arg(short, long, default_value = DEFAULT_BENCH_ESLOGS_TABLE)]
        table: String,
        /// Postgres database url to connect to.
        #[arg(short, long, env = "DATABASE_URL")]
        url: String,
    },
    CountParquetTable {
        /// Postgres table name to build from.
        #[arg(short, long, default_value = DEFAULT_BENCH_ESLOGS_TABLE)]
        table: String,
        /// Postgres database url to connect to.
        #[arg(short, long, env = "DATABASE_URL")]
        url: String,
    },
    BuildElasticIndex {
        /// Postgres table name to build from.
        #[arg(short, long, default_value = DEFAULT_BENCH_ESLOGS_TABLE)]
        table: String,
        /// Postgres database url to connect to.
        #[arg(short, long, env = "DATABASE_URL")]
        url: String,
        /// Elastic index url to connect to.
        /// Should contain the index name as a path subcomponent.
        #[arg(short, long)]
        elastic_url: String,
    },
    QueryElasticIndex {
        /// Index field to match on.
        #[arg(short, long, default_value = "message")]
        field: String,
        /// Search term in index field to match on.
        #[arg(short, long, default_value = "flame")]
        term: String,
        /// Elastic index url to connect to.
        /// Should contain the index name as a path subcomponent.
        #[arg(short, long)]
        elastic_url: String,
    },
}
/// The command to run on the hits corpus.
#[derive(Debug, clap::Subcommand)]
pub enum HitsCommand {
    /// Generate the hits corpus, inserting into a Postgres table.
    Run {
        /// Workload to benchmark, defaults to a file size of 100MB.
        /// - 'single' Runs the full ClickBench benchmark against a single Parquet file
        /// - 'partitioned' Runs the full ClickBench benchmark against one hundred partitioned Parquet files
        #[arg(long, short, default_value = "single")]
        workload: String,
        /// Postgres database url to connect to.
        #[arg(short, long, env = "DATABASE_URL")]
        url: String,
        /// Use the full dataset or a smaller version?
        #[arg(short, long, default_value_t = false)]
        full: bool,
    },
}

impl Cli {
    pub fn parse_args() -> Self {
        let args: Vec<String> = env::args().collect();
        tracing::info!("Collected args: {:#?}", args);

        tracing::trace!("Entering parse_args method");

        // Parse all arguments without skipping
        let parsed = Self::parse();
        tracing::debug!("Parsed result: {:#?}", parsed);
        parsed
    }
}

impl Default for Cli {
    fn default() -> Self {
        Self::parse_args()
    }
}
