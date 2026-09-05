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
mod fault_tolerance;
mod graph;
mod headless;
mod metrics;
mod runner;
mod sqlscanner;
mod suite;
mod table_helper;
mod tui;

use crate::auto::{ServerHandler, setup_server};
use crate::cli::{AutoArgs, Cli, Command};
use crate::fault_tolerance::GraceWindow;
use crate::runner::{SetupMode, SuiteRunner};
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
    // Register the DST assertion catalog (see `dst`), so a never-hit reachability site is
    // reported rather than passing vacuously. A no-op outside the DST environment.
    dst::init();

    let cli = Cli::parse();

    match cli.command {
        // When using the "ui" subcommand.
        Command::Ui(args) => {
            let suite = load_suite(&args.suite_path, args.pgversion, None)?;
            let suite_runner =
                SuiteRunner::new(suite, args.paused, args.grace.window(), SetupMode::Full)?;
            tui::run(suite_runner)?;
        }

        // When using the "headless" subcommand.
        Command::Headless(args) => {
            let suite = load_suite(&args.suite_path, args.pgversion, None)?;
            let setup_mode = if args.setup_only {
                SetupMode::SetupOnly
            } else if args.skip_setup {
                SetupMode::SkipSetup
            } else {
                SetupMode::Full
            };
            let suite_runner = SuiteRunner::new(suite, false, args.grace.window(), setup_mode)?;
            // `--setup-only` has built the schema and is done; the workload runs later in a
            // separate `--skip-setup` process.
            if setup_mode == SetupMode::SetupOnly {
                eprintln!("stressgres: setup complete, exiting without a workload");
                return Ok(());
            }
            let mut log_file = args.log_file.clone();
            if let Some(path) = log_file.as_ref()
                && path.display().to_string() == "-"
            {
                log_file = None
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
            let suite = load_suite(&args.suite_path, None, Some(&args))?;
            // `auto` is a local-dev command with no fault injection, so fail fast
            // (grace 0) rather than tolerating transient connectivity faults.
            let suite_runner = SuiteRunner::new(
                suite,
                false,
                GraceWindow::fixed(Duration::ZERO),
                SetupMode::Full,
            )?;
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

/// Loads the Suite (TOML) from the provided path, tagging any failure with the file path.
fn load_suite<P: AsRef<Path>>(
    path: P,
    pgversion: Option<PgVersion>,
    auto: Option<&AutoArgs>,
) -> anyhow::Result<Suite> {
    let path = path.as_ref();
    load_suite_inner(path, pgversion, auto)
        .with_context(|| format!("Failed to load suite file: {}", path.display()))
}

/// When `auto` is provided (the `auto` subcommand), every `Automatic` server is
/// pointed at the supplied `pg_config` binary and given a data directory under the
/// supplied base path, so a suite can be run against an arbitrary Postgres build
/// without editing its TOML.
fn load_suite_inner(
    path: &Path,
    pgversion: Option<PgVersion>,
    auto: Option<&AutoArgs>,
) -> anyhow::Result<Suite> {
    eprintln!("Loading Suite: {}", path.display());
    let mut definition = load_suite_definition(path)?;

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

fn load_suite_definition(path: &Path) -> anyhow::Result<SuiteDefinition> {
    // Resolve symlinks before relative workload/topology references. Antithesis
    // publishes the selected suite through `/tmp/stressgres-workload.toml`.
    let source_path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let source_dir = source_path.parent().unwrap_or_else(|| Path::new("."));
    let file = std::fs::read_to_string(&source_path)?;
    let mut definition = toml::from_str::<SuiteDefinition>(&file)?;
    if let Some(topology_path) = definition.topology.clone() {
        let topology_path = source_dir.join(topology_path);
        let topology_file = std::fs::read_to_string(&topology_path)
            .with_context(|| format!("Failed to read topology: {}", topology_path.display()))?;
        let mut topology = toml::from_str::<SuiteDefinition>(&topology_file)
            .with_context(|| format!("Failed to parse topology: {}", topology_path.display()))?;
        anyhow::ensure!(
            topology.topology.is_none() && topology.workload.is_none(),
            "nested topology/workload references are not supported"
        );
        topology.workload = definition.workload.take();
        topology.name = definition.name.take().or(topology.name);
        topology.path = definition.path.take();
        definition = topology;
    }
    if let Some(workload_path) = definition.workload.clone() {
        let workload_path = source_dir.join(workload_path);
        let workload_file = std::fs::read_to_string(&workload_path).with_context(|| {
            format!(
                "Failed to read workload definition: {}",
                workload_path.display()
            )
        })?;
        let workload = toml::from_str::<SuiteDefinition>(&workload_file).with_context(|| {
            format!(
                "Failed to parse workload definition: {}",
                workload_path.display()
            )
        })?;
        definition.compose_workload(workload)?;
    }
    definition.path = Some(path.to_path_buf());
    Ok(definition)
}

#[cfg(test)]
mod suite_file_tests {
    use super::load_suite_definition;
    use crate::sqlscanner::StatementDestination;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn all_bundled_suites_parse() {
        let suites_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("suites");
        let mut suite_paths = fs::read_dir(&suites_dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "toml")
            })
            .collect::<Vec<_>>();
        suite_paths.sort();

        for path in suite_paths {
            load_suite_definition(&path)
                .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()));

            let suite_name = path.file_stem().unwrap().to_string_lossy();
            let antithesis_entrypoint = suites_dir
                .join("antithesis")
                .join(format!("first_{suite_name}.sh"));
            assert!(
                antithesis_entrypoint.is_file(),
                "missing Antithesis entrypoint for {}: {}",
                path.display(),
                antithesis_entrypoint.display()
            );
        }
    }

    #[test]
    fn workload_is_composed_with_topology_routes_and_phased_setup() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("suites/logical-replication-mixed-workload.toml");
        let definition = load_suite_definition(&path).unwrap();

        assert_eq!(definition.jobs.len(), 19);
        assert!(
            definition
                .jobs
                .iter()
                .all(|job| !job.destinations.is_empty())
        );

        let subscriber = definition
            .servers
            .iter()
            .find(|server| server.name == "Subscriber")
            .unwrap();
        let create_table = subscriber.setup.sql.find("CREATE TABLE test").unwrap();
        let subscription = subscriber.setup.sql.find("CREATE SUBSCRIPTION").unwrap();
        let create_index = subscriber.setup.sql.find("CREATE INDEX idxtest").unwrap();
        assert!(create_table < subscription && subscription < create_index);

        let multi_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("suites/logical-replication-mixed-workload-multi-subscriber.toml");
        let multi = load_suite_definition(&multi_path).unwrap();
        let top_k = multi
            .jobs
            .iter()
            .find(|job| job.title.as_deref() == Some("Key-ordered Top K Base Scan"))
            .unwrap();
        assert_eq!(top_k.refresh_ms, 10);
        assert_eq!(top_k.destinations.len(), 2);
    }

    #[test]
    fn all_workload_topology_matrix_entries_compose() {
        let matrix_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("suites/matrix");
        let mut paths = fs::read_dir(&matrix_dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        paths.sort();
        assert_eq!(paths.len(), 18);

        for path in paths {
            let definition = load_suite_definition(&path)
                .unwrap_or_else(|error| panic!("failed to compose {}: {error}", path.display()));
            assert!(!definition.jobs.is_empty());
            assert!(!definition.servers.is_empty());
            assert!(
                definition
                    .jobs
                    .iter()
                    .all(|job| !job.destinations.is_empty())
            );

            if path.file_name().unwrap() == "wide-table--logical.toml" {
                let update = definition
                    .jobs
                    .iter()
                    .find(|job| job.title.as_deref() == Some("Single Update"))
                    .unwrap();
                assert_eq!(
                    update.destinations,
                    [StatementDestination::SpecificServers(vec![
                        "Publisher".to_owned()
                    ])]
                );
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn composed_suite_resolves_references_through_a_symlink() {
        use std::os::unix::fs::symlink;

        let suite = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("suites/background-merge.toml");
        let temp = tempfile::tempdir().unwrap();
        let link = temp.path().join("selected-suite.toml");
        symlink(suite, &link).unwrap();

        let definition = load_suite_definition(&link).unwrap();
        assert_eq!(definition.servers.len(), 1);
        assert!(!definition.jobs.is_empty());
    }
}
