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

#![allow(clippy::mutable_key_type)]
#![allow(clippy::type_complexity)]
mod auto;
mod cli;
mod csv;
mod graph;
mod headless;
mod metrics;
mod runner;
mod sqlscanner;
mod suite;
mod table_helper;
mod tui;

use crate::auto::{setup_server, ServerHandler};
use crate::cli::{AutoArgs, Cli, Command};
use crate::runner::SuiteRunner;
use crate::suite::{PgConfigStyle, PgVersion, ServerStyle, Suite, SuiteDefinition};
use anyhow::Context;
use clap::Parser;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use shutdown_hooks::add_shutdown_hook;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsLine {
    pub duration: Duration,
    pub job_title: String,
    pub server_name: String,
    pub metrics: serde_json::Map<String, serde_json::Value>,
}

/// Main entry point using subcommands.
fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        // When using the "ui" subcommand.
        Command::Ui(args) => {
            let suite = load_suite(&args.suite_path, args.pgversion, None).with_context(|| {
                format!("Failed to load suite file: {}", args.suite_path.display())
            })?;
            let suite_runner = SuiteRunner::new(
                suite,
                args.paused,
                Duration::from_millis(args.reconnect_grace_ms),
            )?;
            tui::run(suite_runner)?;
        }

        // When using the "headless" subcommand.
        Command::Headless(args) => {
            let suite = load_suite(&args.suite_path, args.pgversion, None).with_context(|| {
                format!("Failed to load suite file: {}", args.suite_path.display())
            })?;
            let suite_runner = SuiteRunner::new(
                suite,
                false,
                Duration::from_millis(args.reconnect_grace_ms),
            )?;
            let mut log_file = args.log_file.clone();
            if let Some(path) = log_file.as_ref() {
                if path.display().to_string() == "-" {
                    log_file = None
                }
            }
            headless::run(
                suite_runner,
                log_file,
                args.log_interval_ms,
                Some(args.runtime),
            )?;
        }

        // When using the "auto" subcommand: spin up a throwaway Postgres cluster
        // (or two, for logical replication) from the given `pg_config` and run the
        // suite headless against it.
        Command::Auto(args) => {
            let suite = load_suite(&args.suite_path, None, Some(&args)).with_context(|| {
                format!("Failed to load suite file: {}", args.suite_path.display())
            })?;
            let suite_runner = SuiteRunner::new(suite, false)?;
            headless::run(
                suite_runner,
                args.log_path.clone(),
                1000,
                Some(args.runtime as u128),
            )?;
        }

        // When using the "graph" subcommand.
        Command::Graph(graph_args) => {
            graph::run(&graph_args)?;
        }

        Command::Csv(csv_args) => {
            csv::run(&csv_args)?;
        }
    }

    Ok(())
}

/// Loads the Suite (TOML) from the provided path.
///
/// When `auto` is provided (the `auto` subcommand), every `Automatic` server is
/// pointed at the supplied `pg_config` binary and given a data directory under the
/// supplied base path, so a suite can be run against an arbitrary Postgres build
/// without editing its TOML.
fn load_suite<P: AsRef<Path>>(
    path: P,
    pgversion: Option<PgVersion>,
    auto: Option<&AutoArgs>,
) -> anyhow::Result<Suite> {
    eprintln!("Loading Suite: {}", path.as_ref().display());
    let file = std::fs::read_to_string(path.as_ref())?;
    let mut definition = toml::from_str::<SuiteDefinition>(&file)?;
    definition.path = Some(path.as_ref().to_path_buf());

    // Override server configurations with the provided pgversion if specified
    if let Some(version) = pgversion {
        for server in &mut definition.servers {
            match &mut server.style {
                ServerStyle::Pgrx(_) => {
                    server.style = ServerStyle::Pgrx(version.clone());
                }
                ServerStyle::Automatic { pg_config, .. } => {
                    if let PgConfigStyle::Pgrx(_) = pg_config {
                        *pg_config = PgConfigStyle::Pgrx(version.clone());
                    }
                }
                _ => {
                    // For other server styles, we don't override
                }
            }
        }
    }

    // Override every server to use the `auto`-provided pg_config binary and a data
    // directory under the requested base path.
    if let Some(auto) = auto {
        std::fs::create_dir_all(&auto.pg_data_base).with_context(|| {
            format!(
                "Failed to create data directory base {}",
                auto.pg_data_base.display()
            )
        })?;
        for server in &mut definition.servers {
            let name = server.name.clone();
            match &mut server.style {
                ServerStyle::Automatic {
                    pg_config,
                    pgdata,
                    log_path,
                    ..
                } => {
                    *pg_config = PgConfigStyle::Path(auto.pg_config.clone());
                    *pgdata = Some(auto.pg_data_base.join(format!("{name}.data")));
                    *log_path = Some(auto.pg_data_base.join(format!("{name}.log")));
                }
                style => anyhow::bail!(
                    "`stressgres auto` requires `[server.style.Automatic]` servers, \
                     but server `{name}` is {style:?}"
                ),
            }
        }
    }

    eprintln!("{definition:#?}");

    static RUNNING_POSTGRES_INSTANCES: Mutex<Option<Vec<ServerHandler>>> =
        Mutex::new(Some(Vec::new()));

    extern "C" fn shutdown_hook() {
        for handler in RUNNING_POSTGRES_INSTANCES
            .lock()
            .take()
            .into_iter()
            .flatten()
        {
            eprintln!("Shutting down Postgres, pid={:?}", handler.pid());
            handler.kill();
        }
    }
    add_shutdown_hook(shutdown_hook);

    definition.servers.iter().for_each(|server| {
        let server_handle = setup_server(server, &definition.servers).expect("setup_server failed");
        if !matches!(server.style, ServerStyle::With { .. }) {
            RUNNING_POSTGRES_INSTANCES
                .lock()
                .as_mut()
                .unwrap()
                .push(server_handle);
        }
    });

    Ok(Suite::new(definition))
}
