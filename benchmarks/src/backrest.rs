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

use anyhow::{bail, Context, Result};
use clap::{ArgAction, Parser};
use serde_json::Value;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Parser)]
pub struct SnapshotHeapArgs {
    #[command(flatten)]
    options: PgBackRestOptions,
}

#[derive(Parser)]
pub struct RestoreHeapArgs {
    #[command(flatten)]
    options: PgBackRestOptions,

    /// Restore without pgBackRest's --delta option.
    #[arg(long = "no-delta", action = ArgAction::SetFalse, default_value_t = true)]
    delta: bool,
}

#[derive(Parser)]
struct PgBackRestOptions {
    /// Benchmark dataset name.
    #[arg(long, default_value = "stackoverflow")]
    dataset: String,

    /// Size label for the snapshot, e.g. "100k", "1m", or "20m".
    #[arg(long)]
    size: String,

    /// PostgreSQL data directory to snapshot or restore. Defaults to PGDATA when set.
    #[arg(long, env = "PGDATA")]
    pgdata: Option<PathBuf>,

    /// Existing pgBackRest config. When set, repo and pgdata options are ignored.
    /// The stanza must still be provided with --stanza.
    #[arg(long)]
    config: Option<PathBuf>,

    /// pgBackRest stanza name.
    #[arg(long)]
    stanza: Option<String>,

    /// S3 bucket for generated pgBackRest configs.
    #[arg(long)]
    repo_bucket: Option<String>,

    /// Path prefix inside the S3 bucket. The dataset and size are appended.
    #[arg(long)]
    repo_path_prefix: Option<String>,

    /// S3 region for generated pgBackRest configs.
    #[arg(long)]
    repo_region: Option<String>,

    /// S3 endpoint for generated pgBackRest configs.
    #[arg(long)]
    repo_endpoint: Option<String>,

    /// S3 credential mode for generated pgBackRest configs.
    #[arg(long, value_parser = ["shared", "auto", "web-id"])]
    repo_s3_key_type: Option<String>,

    /// Max pgBackRest worker processes. Defaults to available CPUs.
    #[arg(long)]
    process_max: Option<usize>,
}

struct PreparedConfig {
    path: PathBuf,
    stanza: String,
}

struct AwsCredentials {
    access_key_id: String,
    secret_access_key: String,
    session_token: Option<String>,
}

struct GeneratedConfig<'a> {
    pgdata: &'a Path,
    repo_bucket: &'a str,
    repo_endpoint: &'a str,
    repo_path: String,
    repo_region: &'a str,
    repo_s3_key_type: &'a str,
    stanza: &'a str,
    log_path: PathBuf,
    spool_path: PathBuf,
    process_max: usize,
    credentials: Option<AwsCredentials>,
}

pub fn run_snapshot_heap(args: SnapshotHeapArgs) -> Result<()> {
    let config = prepare_config(&args.options)?;
    run_pgbackrest(&config, &["--no-online", "stanza-create"])?;
    run_pgbackrest(&config, &["--type=full", "--no-online", "backup"])?;
    run_pgbackrest(&config, &["info"])?;
    Ok(())
}

pub fn run_restore_heap(args: RestoreHeapArgs) -> Result<()> {
    let config = prepare_config(&args.options)?;
    let backup_count = backup_count(&config)?;
    if backup_count == 0 {
        bail!(
            "No pgBackRest snapshot found for '{}' ({})",
            args.options.dataset,
            args.options.size
        );
    }
    println!(
        "Found {backup_count} snapshot backup(s) for '{}' ({}).",
        args.options.dataset, args.options.size
    );

    let mut restore_args = Vec::new();
    if args.delta {
        restore_args.push("--delta");
    }
    restore_args.push("restore");
    run_pgbackrest(&config, &restore_args)?;
    Ok(())
}

fn prepare_config(options: &PgBackRestOptions) -> Result<PreparedConfig> {
    let stanza = options
        .stanza
        .as_deref()
        .with_context(|| "Provide --stanza")?;
    if let Some(config) = &options.config {
        return Ok(PreparedConfig {
            path: config.clone(),
            stanza: stanza.to_string(),
        });
    }

    let pgdata = options
        .pgdata
        .as_deref()
        .with_context(|| "Provide --pgdata, set PGDATA, or pass --config")?;
    let temp_dir = env::temp_dir();
    let log_path = temp_dir.join("pgbackrest-log");
    let spool_path = temp_dir.join("pgbackrest-spool");
    fs::create_dir_all(&log_path)
        .with_context(|| format!("Failed to create '{}'", log_path.display()))?;
    fs::create_dir_all(&spool_path)
        .with_context(|| format!("Failed to create '{}'", spool_path.display()))?;

    let config_path = temp_dir.join(format!(
        "pgbackrest-{}-{}-{}.conf",
        safe_path_component(&options.dataset),
        safe_path_component(&options.size),
        std::process::id()
    ));
    let repo_bucket = options
        .repo_bucket
        .as_deref()
        .with_context(|| "Provide --repo-bucket or pass --config")?;
    let repo_path_prefix = options
        .repo_path_prefix
        .as_deref()
        .with_context(|| "Provide --repo-path-prefix or pass --config")?;
    let repo_region = options
        .repo_region
        .as_deref()
        .with_context(|| "Provide --repo-region or pass --config")?;
    let repo_endpoint = options
        .repo_endpoint
        .as_deref()
        .with_context(|| "Provide --repo-endpoint or pass --config")?;
    let repo_s3_key_type = options
        .repo_s3_key_type
        .as_deref()
        .with_context(|| "Provide --repo-s3-key-type or pass --config")?;
    let repo_path = repo_path(repo_path_prefix, &options.dataset, &options.size);
    let credentials = match repo_s3_key_type {
        "shared" => Some(load_aws_credentials()?),
        "auto" | "web-id" => None,
        _ => unreachable!("clap validates repo_s3_key_type"),
    };
    let process_max = options.process_max.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    });

    let config = render_config(&GeneratedConfig {
        pgdata,
        repo_bucket,
        repo_endpoint,
        repo_path,
        repo_region,
        repo_s3_key_type,
        stanza,
        log_path,
        spool_path,
        process_max,
        credentials,
    });
    fs::write(&config_path, config)
        .with_context(|| format!("Failed to write '{}'", config_path.display()))?;

    Ok(PreparedConfig {
        path: config_path,
        stanza: stanza.to_string(),
    })
}

fn load_aws_credentials() -> Result<AwsCredentials> {
    let access_key_id = env::var("AWS_ACCESS_KEY_ID")
        .with_context(|| "Set AWS_ACCESS_KEY_ID/AWS_SECRET_ACCESS_KEY or pass --config")?;
    let secret_access_key = env::var("AWS_SECRET_ACCESS_KEY")
        .with_context(|| "Set AWS_ACCESS_KEY_ID/AWS_SECRET_ACCESS_KEY or pass --config")?;
    let session_token = env::var("AWS_SESSION_TOKEN").ok();
    Ok(AwsCredentials {
        access_key_id,
        secret_access_key,
        session_token,
    })
}

fn render_config(config: &GeneratedConfig<'_>) -> String {
    let mut rendered = format!(
        "[global]\n\
         repo1-type=s3\n\
         repo1-s3-bucket={repo_bucket}\n\
         repo1-s3-region={repo_region}\n\
         repo1-s3-endpoint={repo_endpoint}\n",
        repo_bucket = config.repo_bucket,
        repo_region = config.repo_region,
        repo_endpoint = config.repo_endpoint,
    );

    match &config.credentials {
        Some(credentials) => {
            rendered.push_str(&format!(
                "repo1-s3-key={}\nrepo1-s3-key-secret={}\n",
                credentials.access_key_id, credentials.secret_access_key
            ));
            if let Some(token) = &credentials.session_token {
                rendered.push_str(&format!("repo1-s3-token={token}\n"));
            }
        }
        None => {
            rendered.push_str(&format!("repo1-s3-key-type={}\n", config.repo_s3_key_type));
        }
    }

    rendered.push_str(&format!(
        "repo1-path={repo_path}\n\
         repo1-cipher-type=none\n\
         repo1-retention-full=1\n\
         repo1-bundle=y\n\
         log-path={log_path}\n\
         spool-path={spool_path}\n\
         log-level-console=info\n\
         start-fast=y\n\
         process-max={process_max}\n\
         compress-type=lz4\n\
         \n\
         [{stanza}]\n\
         pg1-path={pgdata}\n",
        repo_path = config.repo_path,
        log_path = config.log_path.display(),
        spool_path = config.spool_path.display(),
        process_max = config.process_max,
        stanza = config.stanza,
        pgdata = config.pgdata.display(),
    ));

    rendered
}

fn repo_path(prefix: &str, dataset: &str, size: &str) -> String {
    let prefix = prefix.trim_end_matches('/');
    if prefix.is_empty() {
        format!("/{dataset}/{size}")
    } else {
        format!("{prefix}/{dataset}/{size}")
    }
}

fn safe_path_component(component: &str) -> String {
    component
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

fn backup_count(config: &PreparedConfig) -> Result<usize> {
    let output = pgbackrest_command(config)
        .arg("--output=json")
        .arg("info")
        .output()
        .with_context(|| "Failed to execute pgbackrest info")?;
    if !output.status.success() {
        bail!(
            "Failed to inspect pgBackRest repository: {}",
            command_output(&output)
        );
    }

    let info: Value = serde_json::from_slice(&output.stdout)
        .with_context(|| "Failed to parse pgbackrest info JSON")?;
    let Some(stanzas) = info.as_array() else {
        return Ok(0);
    };
    Ok(stanzas
        .iter()
        .map(|stanza| {
            stanza
                .get("backup")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0)
        })
        .sum())
}

fn run_pgbackrest(config: &PreparedConfig, args: &[&str]) -> Result<()> {
    let status = pgbackrest_command(config)
        .args(args)
        .status()
        .with_context(|| "Failed to execute pgbackrest")?;
    if !status.success() {
        bail!("pgbackrest {} failed", args.join(" "));
    }
    Ok(())
}

fn pgbackrest_command(config: &PreparedConfig) -> Command {
    let mut command = Command::new("pgbackrest");
    command
        .arg(format!("--config={}", config.path.display()))
        .arg(format!("--stanza={}", config.stanza));
    command
}

fn command_output(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !stderr.is_empty() {
        return stderr;
    }
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}
