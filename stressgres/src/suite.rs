// Copyright (c) 2023-2025 ParadeDB, Inc.
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

use crate::sqlscanner::{ScannedStatement, SqlStatementScanner, StatementDestination};
use pgrx_pg_config::{PgConfig, Pgrx};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;

// See https://users.rust-lang.org/t/concatenate-two-static-str/33993/4
#[macro_export]
macro_rules! physical_replication_slot_name {
    () => {
        "physical_wal_receiver_1"
    };
}

#[derive(Serialize, Debug, Clone, Deserialize)]
pub enum PgConfigStyle {
    Pgrx(PgVersion),
    Env,
    Path(PathBuf),
}

impl Default for PgConfigStyle {
    fn default() -> Self {
        PgConfigStyle::Pgrx(PgVersion::default())
    }
}

impl PgConfigStyle {
    pub fn pg_config(&self, port: Option<u16>) -> PgConfig {
        match self {
            PgConfigStyle::Pgrx(version) => {
                let pgrx = Pgrx::from_config().expect("is pgrx configured?");
                let base_pg_config = pgrx
                    .get(&version.to_string())
                    .expect("is pgrx configured with Postgres v17?");
                PgConfig::new(
                    base_pg_config.path().unwrap(),
                    port.unwrap_or_else(default_port),
                    0,
                )
            }
            PgConfigStyle::Env => {
                let base_pg_config = PgConfig::from_path();
                PgConfig::new(
                    base_pg_config.path().unwrap(),
                    port.unwrap_or_else(default_port),
                    0,
                )
            }
            PgConfigStyle::Path(path) => {
                PgConfig::new(path.clone(), port.unwrap_or_else(default_port), 0)
            }
        }
    }
}

#[derive(Serialize, Default, Debug, Clone, Deserialize)]
pub enum PostgresqlConf {
    #[default]
    Normal,
    Publisher,
    Subscriber,
    WalReceiver,
    Custom(String),
}

impl PostgresqlConf {
    pub fn lines(&self) -> impl Iterator<Item = &str> + '_ {
        match self {
            PostgresqlConf::Normal => vec![],
            PostgresqlConf::Publisher => {
                vec!["wal_level=logical"]
            }
            PostgresqlConf::Subscriber => {
                vec!["wal_level=replica", "max_wal_senders=4"]
            }
            PostgresqlConf::WalReceiver => {
                vec![
                    "hot_standby=on",
                    "hot_standby_feedback=on",
                    concat!("primary_slot_name=", physical_replication_slot_name!()),
                ]
            }
            PostgresqlConf::Custom(s) => s.lines().collect::<Vec<_>>(),
        }
        .into_iter()
        .chain(vec![
            "shared_preload_libraries=pg_search",
            "log_line_prefix=%m [%p] [%x] [%a] ",
            "log_error_verbosity=verbose",
            "max_wal_size=8GB",
        ])
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub enum PgVersion {
    V15,
    V16,
    #[default]
    V17,
    V18,
}

impl fmt::Display for PgVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PgVersion::V15 => write!(f, "pg15"),
            PgVersion::V16 => write!(f, "pg16"),
            PgVersion::V17 => write!(f, "pg17"),
            PgVersion::V18 => write!(f, "pg18"),
        }
    }
}

impl FromStr for PgVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pg15" | "15" => Ok(PgVersion::V15),
            "pg16" | "16" => Ok(PgVersion::V16),
            "pg17" | "17" => Ok(PgVersion::V17),
            "pg18" | "18" => Ok(PgVersion::V18),
            _ => Err(format!(
                "Invalid PostgreSQL version: {}. Expected 'pg17' or 'pg18'",
                s
            )),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerStyle {
    Pgrx(PgVersion),
    FromPath,
    Automatic {
        #[serde(default)]
        pg_config: PgConfigStyle,
        #[serde(default = "default_port")]
        port: u16,
        log_path: Option<PathBuf>,
        pgdata: Option<PathBuf>,
        #[serde(default)]
        postgresql_conf: PostgresqlConf,
    },
    With {
        connection_string: String,
    },
}

impl Default for ServerStyle {
    fn default() -> Self {
        ServerStyle::Pgrx(PgVersion::default())
    }
}

impl ServerStyle {
    pub fn port(&self) -> u16 {
        match self {
            ServerStyle::Pgrx(version) => PgConfigStyle::Pgrx(version.clone())
                .pg_config(None)
                .port()
                .expect("`pgrx` should be installed"),
            ServerStyle::FromPath => PgConfigStyle::Env
                .pg_config(None)
                .port()
                .expect("`pg_config` not found"),
            ServerStyle::Automatic {
                port, pg_config, ..
            } => pg_config
                .pg_config(Some(*port))
                .port()
                .expect("`pg_config` not found"),
            ServerStyle::With { connection_string } => {
                let url = url::Url::parse(connection_string).expect("invalid connection string");
                url.port_or_known_default()
                    .expect("no port found in connection string")
            }
        }
    }

    pub fn connstr(&self) -> String {
        match self {
            ServerStyle::Pgrx(_) => {
                format!("host=localhost port={} dbname=stressgres", self.port())
            }
            ServerStyle::FromPath => {
                format!("host=localhost port={} dbname=stressgres", self.port())
            }
            ServerStyle::Automatic { .. } => {
                format!("host=localhost port={} dbname=stressgres", self.port())
            }
            ServerStyle::With { connection_string } => connection_string.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Server {
    #[serde(default)]
    pub default: bool,

    #[serde(deserialize_with = "validate_server_name")]
    pub name: String,

    #[serde(default)]
    pub style: ServerStyle,

    pub setup: Job,
    pub teardown: Job,
    pub monitor: Job,
}

fn validate_server_name<'de, D>(d: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    if s.chars().any(|c| !c.is_ascii_alphanumeric() && c != '_') {
        Err(serde::de::Error::custom(format!(
            "invalid server name `{s}`.  Only `[a-zA-Z0-9_]` are supported"
        )))
    } else {
        Ok(s)
    }
}

fn default_port() -> u16 {
    static LAST_PORT: AtomicU16 = AtomicU16::new(55500);
    LAST_PORT.fetch_add(1, Ordering::Relaxed)
}

/// A single job in the suite.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Job {
    pub title: Option<String>,
    pub on_connect: Option<String>,
    pub sql: String,
    pub assert: Option<String>,
    pub window_height: Option<usize>,
    pub cancel_keycode: Option<char>,
    pub pause_keycode: Option<char>,
    pub cancel_every: Option<f64>,

    #[serde(default)]
    pub atomic_connection: bool,

    /// measured in milliseconds
    #[serde(default = "default_refresh")]
    pub refresh_ms: usize,

    /// If true, log `tps=...`.
    #[serde(default = "default_log_tps")]
    pub log_tps: bool,

    /// Arbitrary column names (e.g. block_count, segment_count) to include in the logs
    #[serde(default)]
    pub log_columns: Vec<String>,

    #[serde(default)]
    #[serde(deserialize_with = "deserialize_destinations")]
    pub destinations: Vec<StatementDestination>,
}

fn deserialize_destinations<'de, D>(d: D) -> Result<Vec<StatementDestination>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let names = Option::<Vec<String>>::deserialize(d)?;
    if names.is_none() {
        return Ok(vec![StatementDestination::DefaultServer]);
    }
    let mut destinations: Vec<StatementDestination> = Vec::new();
    for name in names.unwrap() {
        destinations.push(match name.to_lowercase().as_str() {
            "default" => StatementDestination::DefaultServer,
            "all" => StatementDestination::AllServers,
            _ => StatementDestination::SpecificServers(vec![name]),
        });
    }
    Ok(destinations)
}

impl Default for Job {
    fn default() -> Self {
        Self {
            title: None,
            on_connect: None,
            sql: "".to_string(),
            assert: None,
            window_height: None,
            cancel_keycode: None,
            pause_keycode: None,
            cancel_every: None,
            atomic_connection: false,
            refresh_ms: 0,
            log_tps: false,
            log_columns: vec![],
            destinations: vec![],
        }
    }
}

impl Job {
    pub fn destinations(&self) -> Vec<StatementDestination> {
        if self.destinations.is_empty() {
            vec![StatementDestination::DefaultServer]
        } else {
            self.destinations.clone()
        }
    }
}

fn default_refresh() -> usize {
    1000
}

fn default_log_tps() -> bool {
    true
}

/// A full suite of jobs, plus optional name, setup, teardown, monitor.
#[derive(Deserialize, Debug)]
pub struct SuiteDefinition {
    /// The file path to the suite definition.
    #[serde(skip_serializing)]
    pub path: Option<PathBuf>,

    /// The display name of the suite.
    pub name: Option<String>,

    /// The list of jobs to run as part of the suite.
    pub jobs: Vec<Job>,

    /// The list of servers (Postgres instances) involved in the suite.
    #[serde(deserialize_with = "validate_server_list")]
    #[serde(rename = "server")]
    pub servers: Vec<Server>,

    /// A list of error message substrings that should be ignored during execution and termination.
    #[serde(default)]
    pub ignore_errors: Vec<String>,
}

pub struct Suite {
    definition: SuiteDefinition,
    server_lookup: Arc<HashMap<String, Server>>,
}

#[rustfmt::skip]
fn validate_server_list<'de, D>(d: D) -> Result<Vec<Server>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let mut servers = Vec::<Server>::deserialize(d)?;
    if !servers.is_empty() {
        let mut found_default = false;
        for server in &servers {
            if server.default {
                if found_default {
                    return Err(serde::de::Error::custom("cannot have multiple default servers"));
                }
                found_default = true;
            }
        }
        if !found_default {
            servers[0].default = true;
        }
    }

    Ok(servers)
}

impl Server {
    pub fn connstr(&self) -> String {
        self.style.connstr()
    }

    pub fn is_subscriber(&self) -> bool {
        matches!(
            self.style,
            ServerStyle::Automatic {
                postgresql_conf: PostgresqlConf::Subscriber,
                ..
            }
        )
    }

    pub fn port(&self) -> u16 {
        self.style.port()
    }
}

impl Suite {
    pub fn new(definition: SuiteDefinition) -> Self {
        let server_lookup = definition
            .servers
            .iter()
            .map(|server| (server.name.clone(), server.clone()))
            .collect();

        Self {
            definition,
            server_lookup: Arc::new(server_lookup),
        }
    }

    pub fn name(&self) -> String {
        self.definition.name.clone().unwrap_or_else(|| {
            self.definition
                .path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "<no name>".to_string())
        })
    }

    pub fn ignore_errors(&self) -> &[String] {
        &self.definition.ignore_errors
    }

    pub fn jobs(&self) -> impl Iterator<Item = &Job> {
        self.definition.jobs.iter()
    }

    pub fn server(&self, name: &str) -> Option<&Server> {
        self.server_lookup.get(name)
    }

    pub fn all_servers(&self) -> impl Iterator<Item = &Server> {
        self.definition.servers.iter()
    }

    pub fn server_lookup(&self) -> Arc<HashMap<String, Server>> {
        self.server_lookup.clone()
    }

    pub fn default_server(&self) -> &Server {
        for server in &self.definition.servers {
            if server.default {
                return server;
            }
        }
        unreachable!("there should be a `[[server]]` configuration with `default = true`")
    }
}

impl Job {
    /// Return the user-provided or derived job title.
    pub fn title(&self) -> String {
        if let Some(t) = &self.title {
            return t.trim().to_string();
        }
        // If no title was given, derive from the first statement
        let statements = self.sql();
        if statements.is_empty() {
            "<no sql>".to_string()
        } else {
            statements[0].sql.trim().to_string()
        }
    }

    pub fn is_select(&self) -> bool {
        self.sql()
            .last()
            .map(|stmt| {
                stmt.sql.to_ascii_uppercase().starts_with("SELECT")
                    || stmt.sql.to_ascii_uppercase().starts_with("EXPLAIN")
            })
            .unwrap_or_default()
    }

    /// Return the parsed statements for this job to run when the connection is first opened
    pub fn on_connect(&self) -> Vec<ScannedStatement<'_>> {
        if let Some(on_connect) = &self.on_connect {
            SqlStatementScanner::new(on_connect)
                .into_iter()
                .map(|mut st| {
                    st.sql = st.sql.trim();
                    st
                })
                .filter(|st| !st.sql.is_empty())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Return the parsed statements for this job.
    pub fn sql(&self) -> Vec<ScannedStatement<'_>> {
        SqlStatementScanner::new(&self.sql)
            .into_iter()
            .map(|mut st| {
                st.sql = st.sql.trim();
                st
            })
            .filter(|st| !st.sql.is_empty())
            .collect()
    }
}
