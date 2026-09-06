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

use crate::sqlscanner::{ScannedStatement, SqlStatementScanner, StatementDestination};
use pgrx_pg_config::{PgConfig, Pgrx};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU16, Ordering};

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
                    .expect("is pgrx configured with the requested Postgres version?");
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
    V17,
    #[default]
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
                "Invalid PostgreSQL version: {}. Expected one of 'pg15', 'pg16', 'pg17', or 'pg18'",
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

    /// Optional phased setup supplied by an external workload definition.
    #[serde(default)]
    pub setup_template: Option<String>,

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
    /// A logical destination resolved by the topology's `[routes]` table.
    #[serde(default)]
    pub route: Option<String>,
    /// Text which must occur in the verbose plan for `sql` before the job starts.
    /// A string is accepted for the common case; an array can assert multiple plan nodes.
    #[serde(default, deserialize_with = "deserialize_string_or_vec")]
    pub sql_plan_contains: Vec<String>,
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
    Ok(destinations_from_names(names.unwrap()))
}

fn destinations_from_names(names: Vec<String>) -> Vec<StatementDestination> {
    names
        .into_iter()
        .map(|name| match name.to_lowercase().as_str() {
            "default" => StatementDestination::DefaultServer,
            "all" => StatementDestination::AllServers,
            _ => StatementDestination::SpecificServers(vec![name]),
        })
        .collect()
}

fn deserialize_string_or_vec<'de, D>(d: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrVec {
        String(String),
        Vec(Vec<String>),
    }

    Ok(match StringOrVec::deserialize(d)? {
        StringOrVec::String(value) => vec![value],
        StringOrVec::Vec(values) => values,
    })
}

impl Default for Job {
    fn default() -> Self {
        Self {
            title: None,
            on_connect: None,
            sql: "".to_string(),
            route: None,
            sql_plan_contains: vec![],
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

#[cfg(test)]
mod tests {
    use super::Job;

    #[test]
    fn sql_plan_contains_accepts_a_string_or_array() {
        let one: Job = toml::from_str(
            r#"
            sql = "SELECT 1"
            sql_plan_contains = "Result"
            "#,
        )
        .unwrap();
        assert_eq!(one.sql_plan_contains, ["Result"]);

        let many: Job = toml::from_str(
            r#"
            sql = "SELECT 1"
            sql_plan_contains = ["Result", "Output"]
            "#,
        )
        .unwrap();
        assert_eq!(many.sql_plan_contains, ["Result", "Output"]);
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

    /// Workload definition to compose with this topology, relative to this file.
    #[serde(default)]
    pub workload: Option<PathBuf>,

    /// Topology definition to compose with this suite, relative to this file.
    #[serde(default)]
    pub topology: Option<PathBuf>,

    /// Features supplied by a topology or required by a workload.
    #[serde(default)]
    pub capabilities: Vec<String>,

    #[serde(default)]
    pub requires: Vec<String>,

    /// Phased, reusable server setup snippets supplied by a workload.
    #[serde(default)]
    pub setup_templates: HashMap<String, SetupTemplate>,

    /// Maps logical workload routes to concrete destinations and an interval scale.
    #[serde(default)]
    pub routes: HashMap<String, Route>,

    /// The list of jobs to run as part of the suite.
    #[serde(default)]
    pub jobs: Vec<Job>,

    /// The list of servers (Postgres instances) involved in the suite.
    #[serde(default, deserialize_with = "validate_server_list")]
    #[serde(rename = "server")]
    pub servers: Vec<Server>,

    /// A list of error message substrings that should be ignored during execution and termination.
    #[serde(default)]
    pub ignore_errors: Vec<String>,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct SetupTemplate {
    #[serde(default)]
    pub before: String,
    #[serde(default)]
    pub after: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Route {
    pub destinations: Vec<String>,
    #[serde(default = "default_route_refresh_scale")]
    pub refresh_scale: f64,
}

fn default_route_refresh_scale() -> f64 {
    1.0
}

impl SuiteDefinition {
    /// Combine schema/query definitions from a workload with concrete topology settings.
    pub fn compose_workload(&mut self, mut workload: SuiteDefinition) -> anyhow::Result<()> {
        anyhow::ensure!(
            workload.workload.is_none(),
            "nested workload definitions are not supported"
        );
        anyhow::ensure!(
            workload.servers.is_empty(),
            "a workload definition must not declare concrete servers"
        );
        for required in &workload.requires {
            anyhow::ensure!(
                self.capabilities.contains(required),
                "workload requires topology capability `{required}`"
            );
        }

        for (name, template) in workload.setup_templates.drain() {
            anyhow::ensure!(
                self.setup_templates
                    .insert(name.clone(), template)
                    .is_none(),
                "duplicate setup template `{name}`"
            );
        }

        for server in &mut self.servers {
            let Some(template_name) = &server.setup_template else {
                continue;
            };
            let template = self.setup_templates.get(template_name).ok_or_else(|| {
                anyhow::anyhow!(
                    "server `{}` references unknown setup template `{template_name}`",
                    server.name
                )
            })?;
            server.setup.sql = [
                template.before.trim(),
                server.setup.sql.trim(),
                template.after.trim(),
            ]
            .into_iter()
            .filter(|sql| !sql.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        }

        workload.jobs.append(&mut self.jobs);
        self.jobs = workload.jobs;
        self.ignore_errors.splice(0..0, workload.ignore_errors);

        for job in &mut self.jobs {
            if job.route.is_none() {
                job.route = Some(if job.is_select() { "read" } else { "write" }.to_owned());
            }
            let Some(route_name) = &job.route else {
                continue;
            };
            let route = self.routes.get(route_name).ok_or_else(|| {
                anyhow::anyhow!(
                    "job `{}` references unknown route `{route_name}`",
                    job.title()
                )
            })?;
            anyhow::ensure!(
                route.refresh_scale.is_finite() && route.refresh_scale > 0.0,
                "route `{route_name}` has invalid refresh_scale {}",
                route.refresh_scale
            );
            job.destinations = destinations_from_names(route.destinations.clone());
            job.refresh_ms = ((job.refresh_ms as f64) * route.refresh_scale)
                .round()
                .max(1.0) as usize;
        }

        Ok(())
    }
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
