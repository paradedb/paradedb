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

use crate::fault_tolerance::GraceWindow;
use crate::suite::PgVersion;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use std::time::Duration;

/// Reconnect-grace options, shared by the `ui` and `headless` subcommands.
#[derive(Debug, Args)]
pub struct ReconnectGraceArgs {
    /// How long (in milliseconds) to tolerate one continuous transient database fault
    /// (dropped/refused sockets, server restarting) by reconnecting before failing the
    /// run. The window restarts after a successful reconnect. Defaults to 0, i.e. any
    /// error fails the run immediately.
    ///
    /// Under fault injection set this longer than `--runtime`, so a connectivity fault
    /// can never fail the run, and pair it with `--reconnect-grace-file`: any window
    /// shorter than the run is one the fault injector can outlast, so liveness has to be
    /// asserted while faults are healed rather than guessed at while they are active.
    #[arg(long, default_value = "0")]
    pub reconnect_grace: u64,

    /// Path to a file whose contents (a count of milliseconds) override
    /// `--reconnect-grace` for as long as it exists, re-read on every failed attempt.
    ///
    /// This is how an external supervisor narrows the window at runtime. Under Antithesis
    /// the `anytime_recovery_liveness` command heals every fault and then writes a shorter
    /// window here, so the run fails only if the database was provably reachable and the
    /// workload still could not make progress.
    #[arg(long)]
    pub reconnect_grace_file: Option<PathBuf>,
}

impl ReconnectGraceArgs {
    /// The grace window these options describe.
    pub fn window(&self) -> GraceWindow {
        let baseline = Duration::from_millis(self.reconnect_grace);
        match self.reconnect_grace_file.clone() {
            Some(file) => GraceWindow::pokeable(baseline, file),
            None => GraceWindow::fixed(baseline),
        }
    }
}

/// Stress testing for ParadeDB/PostgreSQL
#[derive(Debug, Parser)]
#[command(version, about = "Stress testing for ParadeDB/PostgreSQL")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// Stressgres subcommands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run a stress test suite interactively via a TUI.
    Ui(UiArgs),

    /// Run a stress test suite in headless mode (log metrics to stdout or a file).
    Headless(HeadlessArgs),

    /// Parse a log file and generate metric graphs from the contained data
    Graph(GraphArgs),

    /// Parse a log file and generate an aggregated CSV report of contained data
    Csv(CsvArgs),

    /// Given the location of a `pg_config` automatically spin up an transient Postgres cluster (or two, if logical replication is enabled)
    Auto(AutoArgs),
}

/// Arguments for running the suite in interactive (TUI) mode.
#[derive(Debug, Args)]
pub struct UiArgs {
    /// Path to the .toml suite file.
    #[arg(value_name = "SUITE_PATH")]
    pub suite_path: PathBuf,

    /// Start in paused mode.
    #[arg(long, default_value = "false")]
    pub paused: bool,

    /// PostgreSQL version to use (pg15, pg16, pg17, or pg18).
    #[arg(long, default_value = "pg18")]
    pub pgversion: Option<PgVersion>,

    #[command(flatten)]
    pub grace: ReconnectGraceArgs,
}

/// Arguments for running the suite in headless mode.
#[derive(Debug, Args)]
pub struct HeadlessArgs {
    /// Path to the .toml suite file.
    #[arg(value_name = "SUITE_PATH")]
    pub suite_path: PathBuf,
    /// Logging interval (in milliseconds).
    #[arg(long, default_value = "10")]
    pub log_interval_ms: u64,
    /// Optional: if provided, logs are written to this file instead of stdout.
    #[arg(long)]
    pub log_file: Option<PathBuf>,
    /// Runtime (in milliseconds)
    #[arg(long, default_value = "600000")]
    pub runtime: u128,
    /// PostgreSQL version to use (pg15, pg16, pg17, or pg18).
    #[arg(long, default_value = "pg18")]
    pub pgversion: Option<PgVersion>,
    /// Build the schema (run the `setup` job) and exit, running no workload. For an
    /// Antithesis `first_` command, which runs fault-free before any driver commands.
    #[arg(long, conflicts_with = "skip_setup")]
    pub setup_only: bool,
    /// Skip the `setup` job and teardown; connect to a schema a prior `first_` built and run
    /// the workload only. For an Antithesis `singleton_driver_` command.
    #[arg(long)]
    pub skip_setup: bool,
    #[command(flatten)]
    pub grace: ReconnectGraceArgs,
}

/// Arguments for parsing a log file and generating the desired charts.
#[derive(Debug, Args)]
pub struct GraphArgs {
    /// Path to the log file produced by a headless run.
    pub log_path: PathBuf,

    /// Output image file prefix (e.g. "output").
    /// We will create: output_tps.png, output_block_count.png, output_segment_count.png
    #[arg(default_value = "output.png")]
    pub output: String,
}

/// Arguments for parsing a log file and generating an aggregated CSV report
#[derive(Debug, Args)]
pub struct CsvArgs {
    /// Path to the log file produced by a headless run.
    pub log_path: PathBuf,

    /// Output image file prefix (e.g. "output").
    /// We will create: output_tps.png, output_block_count.png, output_segment_count.png
    #[arg(default_value = "output.csv")]
    pub output: String,
}

#[derive(Debug, Args)]
pub struct AutoArgs {
    #[arg(value_name = "PG_CONFIG")]
    pub pg_config: PathBuf,

    #[arg(value_name = "SUITE_PATH")]
    pub suite_path: PathBuf,

    #[arg(value_name = "PG_DATA")]
    pub pg_data_base: PathBuf,

    /// Path to the log file produced by a headless run.
    #[arg(long)]
    pub log_path: Option<PathBuf>,

    #[arg(long, default_value = "600000")]
    pub runtime: u64,
}
