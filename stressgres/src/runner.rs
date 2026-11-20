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

use crate::fault_tolerance::{GraceWindow, is_transient_connection_error, tolerate_transient};
use crate::sqlscanner::StatementDestination;
use crate::suite::{Job, Server, ServerStyle, Suite};
use anyhow::{Result, anyhow};
use cursive_core::style::{BaseColor, Color};
use parking_lot::{Mutex, RwLock};
use postgres::Row;
use postgres::error::SqlState;
use postgres::types::ToSql;
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, AtomicUsize, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use sysinfo::{Pid, ProcessesToUpdate, System};
use tokio::runtime::{Builder, Runtime};
use tokio::task::AbortHandle;

const CANCELLATION_TIMEOUT: Duration = Duration::from_secs(5);

type PostgresResult<T> = std::result::Result<T, Arc<postgres::Error>>;

pub type BackendPid = i32;

fn format_postgres_error(error: &postgres::Error) -> String {
    let Some(db_error) = error.as_db_error() else {
        return error.to_string();
    };

    let mut message = format!(
        "{} (SQLState: {})",
        db_error.message(),
        db_error.code().code()
    );

    if let Some(detail) = db_error.detail() {
        message.push_str("\nDETAIL: ");
        message.push_str(detail);
    }
    if let Some(hint) = db_error.hint() {
        message.push_str("\nHINT: ");
        message.push_str(hint);
    }
    if let Some(where_) = db_error.where_() {
        message.push_str("\nWHERE: ");
        message.push_str(where_);
    }

    let location = match (db_error.file(), db_error.line(), db_error.routine()) {
        (Some(file), Some(line), Some(routine)) => Some(format!("{file}:{line} ({routine})")),
        (Some(file), Some(line), None) => Some(format!("{file}:{line}")),
        (Some(file), None, Some(routine)) => Some(format!("{file} ({routine})")),
        (Some(file), None, None) => Some(file.to_string()),
        (None, Some(line), Some(routine)) => Some(format!("line {line} ({routine})")),
        (None, Some(line), None) => Some(format!("line {line}")),
        (None, None, Some(routine)) => Some(routine.to_string()),
        (None, None, None) => None,
    };
    if let Some(location) = location {
        message.push_str("\nLOCATION: ");
        message.push_str(&location);
    }

    message
}

/// A thread handle paired with its corresponding JobRunner for error reporting
struct ThreadHandle {
    handle: JoinHandle<std::result::Result<(), Arc<anyhow::Error>>>,
    runner: Arc<JobRunner>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct ConnGeneration(u64);

pub struct Conn {
    generation: ConnGeneration,
    server_lookup: Arc<HashMap<String, Server>>,
    clients: Vec<(WorkerClient, Server, BackendPid)>,
}

#[derive(Debug, Clone)]
pub struct ConnInfo {
    pub id: usize,
    meta: Arc<RwLock<(String, Server, BackendPid)>>,
}

impl Eq for ConnInfo {}
impl PartialEq for ConnInfo {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Ord for ConnInfo {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}
impl PartialOrd for ConnInfo {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl ConnInfo {
    pub fn new(id: usize, server: &Server, pid: BackendPid) -> Self {
        let name = format!("{} (port={})", server.name, server.style.port());
        Self {
            id,
            meta: Arc::new(RwLock::new((name, server.clone(), pid))),
        }
    }

    pub fn name(&self) -> String {
        self.meta.read().0.clone()
    }

    pub fn server(&self) -> Server {
        self.meta.read().1.clone()
    }

    pub fn pid(&self) -> BackendPid {
        self.meta.read().2
    }

    pub fn color(&self) -> Color {
        Color::Light(BaseColor::Green)
    }
}

pub struct MultiTransaction<'a> {
    server_lookup: Arc<HashMap<String, Server>>,
    transactions: Vec<(
        tokio_postgres::Transaction<'a>,
        &'a ConnectionIo,
        &'a Server,
        BackendPid,
    )>,
}

impl Conn {
    pub fn make_infos(suite: &Suite, job: &Job) -> Vec<ConnInfo> {
        job.destinations()
            .iter()
            .map(|destination| match destination {
                StatementDestination::DefaultServer => {
                    let server = suite.default_server();
                    vec![ConnInfo::new(0, server, 0)]
                }
                StatementDestination::SpecificServers(server_names) => server_names
                    .iter()
                    .map(|name| {
                        let server = suite
                            .server(name)
                            .unwrap_or_else(|| panic!("No such server named `{name}`"));
                        ConnInfo::new(0, server, 0)
                    })
                    .collect::<Vec<_>>(),
                StatementDestination::AllServers => suite
                    .all_servers()
                    .map(|server| ConnInfo::new(0, server, 0))
                    .collect::<Vec<_>>(),
            })
            .collect::<Vec<_>>()
            .into_iter()
            .flatten()
            .enumerate()
            .map(|(id, mut conninfo)| {
                conninfo.id = id;
                conninfo
            })
            .collect::<Vec<_>>()
    }

    fn open(suite: &Suite, job: &Job, policy: &IoPolicy) -> Result<Self> {
        let clients = job
            .destinations()
            .iter()
            .map(|destination| match destination {
                StatementDestination::DefaultServer => {
                    let server = suite.default_server();
                    Ok(vec![(
                        Conn::open_client(server, job, policy)?,
                        server.clone(),
                    )])
                }
                StatementDestination::SpecificServers(server_names) => server_names
                    .iter()
                    .map(|name| {
                        let server = suite
                            .server(name)
                            .ok_or_else(|| anyhow!("No such server named `{name}`"))?;
                        Ok((Conn::open_client(server, job, policy)?, server.clone()))
                    })
                    .collect::<Result<Vec<_>>>(),
                StatementDestination::AllServers => suite
                    .all_servers()
                    .map(|server| Ok((Conn::open_client(server, job, policy)?, server.clone())))
                    .collect::<Result<Vec<_>>>(),
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .map(|(conn, server)| {
                let pid: BackendPid = conn.query_one("SELECT pg_backend_pid();", &[])?.get(0);
                Ok((conn, server, pid))
            })
            .collect::<Result<Vec<_>>>()?;

        static NEXT_GENERATION: AtomicU64 = AtomicU64::new(1);
        Ok(Self {
            generation: ConnGeneration(NEXT_GENERATION.fetch_add(1, Ordering::Relaxed)),
            server_lookup: suite.server_lookup(),
            clients,
        })
    }

    pub(crate) fn first_client(&self) -> &WorkerClient {
        &self.clients[0].0
    }

    pub fn transaction(&mut self) -> std::result::Result<MultiTransaction<'_>, postgres::Error> {
        let mut transaction = MultiTransaction {
            server_lookup: self.server_lookup.clone(),
            transactions: Vec::with_capacity(self.clients.len()),
        };
        for (client, server, pid) in &mut self.clients {
            let xact = client.io.run(client.client.transaction())?;
            transaction
                .transactions
                .push((xact, &client.io, server, *pid));
        }
        Ok(transaction)
    }

    fn rollback_best_effort(&mut self) {
        for (client, ..) in &mut self.clients {
            let _ = client.io.run(client.client.batch_execute("ROLLBACK"));
        }
    }

    pub fn query(
        &mut self,
        query: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Vec<(PostgresResult<Vec<Row>>, Duration, ConnInfo)> {
        let query = do_query_replacements(query, &self.server_lookup);
        let mut results = Vec::new();

        for (id, (client, server, pid)) in self.clients.iter_mut().enumerate() {
            let start = Instant::now();
            let result = client
                .io
                .run(client.client.query(&query, params))
                .map_err(Arc::new);
            let duration = start.elapsed();
            results.push((result, duration, ConnInfo::new(id, server, *pid)));
        }

        results
    }

    fn tls_connector() -> postgres_openssl::MakeTlsConnector {
        use openssl::ssl::{SslConnector, SslMethod};
        let mut builder =
            SslConnector::builder(SslMethod::tls()).expect("ssl builder should not fail");
        builder.set_verify(openssl::ssl::SslVerifyMode::NONE);
        postgres_openssl::MakeTlsConnector::new(builder.build())
    }

    #[doc(hidden)]
    pub(crate) fn _open_conn(server: &Server, job: &Job) -> Result<WorkerClient> {
        Self::open_client(server, job, &IoPolicy::default())
    }

    fn open_client(server: &Server, job: &Job, policy: &IoPolicy) -> Result<WorkerClient> {
        let sanitized = job.title().replace(|c: char| !c.is_alphanumeric(), "_");
        let connstr = match server.style {
            // For ServerStyle::With, the connection string is expected to already
            // include the application_name if needed, so it is not appended here.
            ServerStyle::With { .. } => server.connstr(),
            _ => format!("{} application_name={}", server.connstr(), sanitized),
        };
        let conn = WorkerClient::connect(&connstr, policy)?;

        for query in job.on_connect() {
            conn.io.run(conn.client.execute(query.sql, &[]))?;
        }

        if !job.sql_plan_contains.is_empty() {
            let statements = job.sql();
            anyhow::ensure!(
                statements.len() == 1,
                "job `{}` uses `sql_plan_contains`, but its `sql` contains {} statements; exactly one is required",
                job.title(),
                statements.len()
            );
            let explain = format!("EXPLAIN (VERBOSE) {}", statements[0].sql);
            let plan = conn
                .io
                .run(conn.client.query(explain.as_str(), &[]))?
                .into_iter()
                .map(|row| row.get::<_, String>(0))
                .collect::<Vec<_>>()
                .join("\n");
            for expected in &job.sql_plan_contains {
                anyhow::ensure!(
                    plan.to_lowercase().contains(&expected.to_lowercase()),
                    "plan assertion failed for job `{}`: expected `{expected}` in plan for `{}`. Actual plan:\n{plan}",
                    job.title(),
                    statements[0].sql
                );
            }
        }

        Ok(conn)
    }
}

// Shared by the suite and every connection, including connections still starting up.
#[derive(Clone, Default)]
struct IoPolicy {
    operation_timeout: Option<Duration>,
    shutdown: Arc<RwLock<Option<Instant>>>,
    interrupt_shutdown: bool,
}

impl IoPolicy {
    async fn wait<T>(&self, operation: impl Future<Output = T>) -> Option<T> {
        if !self.interrupt_shutdown && self.operation_timeout.is_none() {
            return Some(operation.await);
        }

        let started = Instant::now();
        let mut operation = std::pin::pin!(operation);
        loop {
            let remaining = [
                self.operation_timeout
                    .map(|timeout| timeout.saturating_sub(started.elapsed())),
                self.shutdown
                    .read()
                    .map(|deadline| deadline.saturating_duration_since(Instant::now())),
            ]
            .into_iter()
            .flatten()
            .min();
            let delay = remaining
                .unwrap_or(Duration::MAX)
                .min(Duration::from_millis(100));
            // Timeout polls the operation first, so a ready SQL response still wins.
            if let Ok(result) = tokio::time::timeout(delay, operation.as_mut()).await {
                return Some(result);
            }
            if remaining.is_some_and(|remaining| remaining <= delay) {
                return None;
            }
        }
    }
}

struct ConnectionIo {
    runtime: Runtime,
    connection: AbortHandle,
    policy: IoPolicy,
}

impl ConnectionIo {
    fn run<T>(
        &self,
        operation: impl Future<Output = Result<T, postgres::Error>>,
    ) -> Result<T, postgres::Error> {
        self.runtime.block_on(async {
            let mut operation = std::pin::pin!(operation);
            if let Some(result) = self.policy.wait(operation.as_mut()).await {
                return result;
            }
            eprintln!("stressgres: database operation exceeded its deadline, closing connection (outcome may be unknown)");
            self.connection.abort();
            operation.await
        })
    }
}

pub(crate) struct WorkerClient {
    client: tokio_postgres::Client,
    io: ConnectionIo,
}

impl WorkerClient {
    fn connect(connstr: &str, policy: &IoPolicy) -> Result<Self> {
        let runtime = Builder::new_current_thread().enable_all().build()?;
        let connstr = tokio_connstr(connstr);
        let (client, connection) = runtime
            .block_on(policy.wait(tokio_postgres::connect(&connstr, Conn::tls_connector())))
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "database connection exceeded its deadline",
                )
            })??;
        let connection = runtime.spawn(connection).abort_handle();
        Ok(Self {
            client,
            io: ConnectionIo {
                runtime,
                connection,
                policy: policy.clone(),
            },
        })
    }

    fn query_one(
        &self,
        query: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Row, postgres::Error> {
        self.io.run(self.client.query_one(query, params))
    }
}

impl MultiTransaction<'_> {
    pub fn query(
        &mut self,
        query: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Vec<(PostgresResult<Vec<Row>>, Duration, ConnInfo)> {
        let query = do_query_replacements(query, &self.server_lookup);
        let mut results = Vec::new();

        for (id, (xact, io, server, pid)) in self.transactions.iter_mut().enumerate() {
            let start = Instant::now();
            results.push((
                io.run(xact.query(&query, params)).map_err(Arc::new),
                start.elapsed(),
                ConnInfo::new(id, server, *pid),
            ));
        }

        results
    }

    pub fn commit(mut self) -> std::result::Result<(), postgres::Error> {
        let mut first_err = None;
        for (xact, io, ..) in self.transactions.drain(..) {
            if first_err.is_some() {
                let _ = io.run(xact.rollback());
            } else if let Err(e) = io.run(xact.commit()) {
                if is_transient_connection_error(&e) {
                    eprintln!(
                        "stressgres: lost the COMMIT response, transaction outcome may be unknown"
                    );
                }
                first_err = Some(e);
            }
        }
        match first_err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}

impl Drop for MultiTransaction<'_> {
    fn drop(&mut self) {
        for (xact, io, ..) in self.transactions.drain(..) {
            let _ = io.run(xact.rollback());
        }
    }
}

fn do_query_replacements(query: &str, server_lookup: &Arc<HashMap<String, Server>>) -> String {
    let mut query = query.to_owned();
    for (name, server) in server_lookup.iter() {
        // replace connection string tokens in the query
        let key = format!("@{}_CONNSTR@", name);
        query = query.replace(&key, &server.connstr());
    }
    query
}

/// Adapts a connection string for our tokio-postgres client.
///
/// Connection strings are written with libpq's canonical parameter names so the same
/// string works both here and when it is inlined verbatim into SQL the server parses with
/// libpq (e.g. `CREATE SUBSCRIPTION ... CONNECTION '...'`, via [`do_query_replacements`]).
/// The two spellings agree on every keyword except the keepalive probe count, which libpq
/// calls `keepalives_count` and tokio-postgres calls `keepalives_retries`; tokio-postgres
/// rejects the libpq name outright. Translate just that one so the client accepts the
/// libpq-canonical string. Names never used in-tree stay untranslated on purpose.
fn tokio_connstr(connstr: &str) -> String {
    connstr.replace("keepalives_count", "keepalives_retries")
}

/// Which portion of the suite lifecycle a [`SuiteRunner`] executes.
///
/// The schema build and the workload can run in separate processes so setup stays off the
/// fault path: one process builds the schema before fault injection begins, and another runs
/// the workload against it under faults.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SetupMode {
    /// Build the schema (the `setup` job) and stop: no workload, no teardown.
    SetupOnly,
    /// Skip the `setup` job and teardown; connect to a schema a prior setup run built and run
    /// the workload only.
    SkipSetup,
    /// The whole lifecycle: setup, workload, teardown. For the TUI, `auto`, and local runs.
    Full,
}

/// Manages the entire suite: runs setup once, spawns each job's thread, optionally a monitor job,
/// and finally runs teardown when finished.
pub struct SuiteRunner {
    suite: Arc<Suite>,
    setup_mode: SetupMode,
    pgver: String,
    alive: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    handles: Mutex<Vec<ThreadHandle>>,
    runners: Vec<Arc<JobRunner>>,
    have_error: Arc<AtomicBool>,
    first_error_duration_bits: Arc<AtomicU64>,
    reconnect_grace: GraceWindow,
    io_policy: IoPolicy,

    monitors: Vec<Arc<JobRunner>>,
    sys: Arc<RwLock<System>>,
}

/// Run `op` against a live [`Conn`], tolerating transient connectivity faults.
///
/// Open connections lazily and discard them on error. A new connection alone does
/// not reset the grace period: the workload must complete an iteration.
fn with_reconnect<T>(
    alive: &AtomicBool,
    grace: &GraceWindow,
    conn: &mut Option<Conn>,
    suite: &Suite,
    job: &Job,
    io_policy: &IoPolicy,
    mut op: impl FnMut(&mut Conn) -> Result<T>,
) -> Result<Option<T>> {
    tolerate_transient(alive, grace, || {
        if conn.is_none() {
            *conn = Some(Conn::open(suite, job, io_policy)?);
        }
        if io_policy.shutdown.read().is_some() {
            *conn = None;
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "worker is shutting down",
            )
            .into());
        }
        let result = op(conn.as_mut().unwrap());
        if result.is_err() {
            *conn = None;
        }
        result
    })
}

impl SuiteRunner {
    /// [`with_reconnect`] for a one-shot job (version probe, setup, teardown): opens a
    /// throwaway connection and fills in the runner's `alive` / grace / suite, so the
    /// caller only supplies the job and what to do with the connection.
    fn run_once<T>(&self, job: &Job, op: impl FnMut(&mut Conn) -> Result<T>) -> Result<Option<T>> {
        // Setup and teardown use their own I/O budget. The worker cutoff only
        // applies to the workload, and teardown starts after the workers join.
        let io_policy = IoPolicy {
            operation_timeout: self.io_policy.operation_timeout,
            ..IoPolicy::default()
        };
        let mut conn = None;
        with_reconnect(
            &self.alive,
            &self.reconnect_grace,
            &mut conn,
            &self.suite,
            job,
            &io_policy,
            op,
        )
    }

    /// Create a new SuiteRunner in the given [`SetupMode`], run whatever startup that mode
    /// needs (setup and/or the version probe), then spawn a thread per job and monitor.
    ///
    /// Antithesis builds the schema before fault injection. Its `SkipSetup` driver
    /// starts workers without waiting for a version probe or schema setup.
    pub fn new(
        suite: Suite,
        paused: bool,
        reconnect_grace: GraceWindow,
        operation_timeout: Option<Duration>,
        setup_mode: SetupMode,
    ) -> Result<Arc<Self>> {
        let suite = Arc::new(suite);
        let mut runner = Self {
            suite: suite.clone(),
            setup_mode,
            pgver: String::from("<unknown>"),
            alive: Arc::new(AtomicBool::new(true)),
            paused: Arc::new(AtomicBool::new(paused)),
            handles: Default::default(),
            runners: Default::default(),
            have_error: Arc::new(AtomicBool::new(false)),
            first_error_duration_bits: Default::default(),
            io_policy: IoPolicy {
                operation_timeout,
                interrupt_shutdown: !reconnect_grace.current().is_zero(),
                ..IoPolicy::default()
            },
            reconnect_grace,

            monitors: Default::default(),
            sys: Arc::new(RwLock::new(System::new_all()?)),
        };

        // Probe the server version (informational). Skip it under `SkipSetup`: that mode runs
        // under faults, and a blocking probe would ride the long reconnect grace before the
        // runtime clock starts and pin the driver. Setup runs fault-free, so the probe
        // is safe under `SetupOnly` / `Full`.
        if setup_mode != SetupMode::SkipSetup {
            let default_job = Job::default();
            if let Some(version) = runner.run_once(&default_job, |conn| {
                let version: String = conn
                    .first_client()
                    .query_one("SELECT version()", &[])?
                    .get(0);
                Ok(version)
            })? {
                runner.pgver = version;
            }
        }

        runner.init()?;
        let suite_runner = Arc::new(runner);

        // Refresh sysinfo stats very frequently. A `SetupOnly` run spawned no workers or
        // monitors and exits right after this, so skip the refresh loop entirely.
        if suite_runner.setup_mode != SetupMode::SetupOnly {
            let alive = suite_runner.alive.clone();
            let sys = suite_runner.sys.clone();
            let suite_runner = suite_runner.clone();
            std::thread::spawn(move || {
                while alive.load(Ordering::Relaxed) {
                    if suite_runner.any_running() {
                        sys.write().refresh_processes(ProcessesToUpdate::All, true);

                        suite_runner
                            .runners()
                            .chain(suite_runner.monitor_runners())
                            .for_each(|job_runner| job_runner.update_sysinfo_stats());
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            });
        }

        Ok(suite_runner)
    }

    /// Run what the [`SetupMode`] calls for: the setup job (unless `SkipSetup`), then the
    /// monitors and job threads (unless `SetupOnly`).
    fn init(&mut self) -> Result<()> {
        // Run the setup job, unless this process is running the workload against a schema a
        // prior setup run already built (`SkipSetup`).
        if self.setup_mode != SetupMode::SkipSetup {
            for server in self.suite.all_servers() {
                let mut setup_job = server.setup.clone();
                eprintln!(
                    "Running setup job for {} on {}",
                    server.name,
                    server.connstr()
                );
                setup_job.destinations = vec![StatementDestination::SpecificServers(vec![
                    server.name.clone(),
                ])];

                let setup_runner = JobRunner::new(
                    self.suite.clone(),
                    setup_job,
                    false,
                    self.alive.clone(),
                    self.sys.clone(),
                )?;
                // Run setup on a fresh connection, replaying it in full if a transient
                // fault interrupts it. Replay is safe because setup is idempotent (every
                // statement is DROP ... IF EXISTS / CREATE OR REPLACE), so re-running from
                // the top rebuilds state no matter how far a partial attempt got — not
                // because it runs in one transaction (it can't: it issues ALTER SYSTEM /
                // pg_reload_conf, which are disallowed inside a transaction block).
                if self
                    .run_once(&setup_runner.job, |conn| setup_runner.run(conn))?
                    .is_none()
                {
                    // `alive` went false before setup finished; stop before spawning workers.
                    return Ok(());
                }
            }
        }

        // `SetupOnly` builds the schema and stops here; the workload runs later in a
        // separate process.
        if self.setup_mode == SetupMode::SetupOnly {
            return Ok(());
        }

        // setup and start the monitors
        for server in self.suite.all_servers() {
            let mut monitor_job = server.monitor.clone();
            monitor_job.destinations = vec![StatementDestination::SpecificServers(vec![
                server.name.clone(),
            ])];

            let monitor_runner = JobRunner::new(
                self.suite.clone(),
                monitor_job,
                true,
                self.alive.clone(),
                self.sys.clone(),
            )?;
            let paused_always_false = Arc::new(AtomicBool::new(false));
            let (mrunner, mhandle) = self.make_runner_thread(monitor_runner, paused_always_false);

            self.monitors.push(mrunner.clone());
            self.handles.lock().push(ThreadHandle {
                handle: mhandle,
                runner: mrunner,
            });
        }

        // 3. For each main job, spawn a thread
        for job in self.suite.jobs() {
            let jrunner = JobRunner::new(
                self.suite.clone(),
                job.clone(),
                false,
                self.alive.clone(),
                self.sys.clone(),
            )?;
            let (runner_arc, handle) = self.make_runner_thread(jrunner, self.paused.clone());
            self.runners.push(runner_arc.clone());
            self.handles.lock().push(ThreadHandle {
                handle,
                runner: runner_arc,
            });
        }

        // Give them a moment to start up
        std::thread::sleep(Duration::from_millis(1000));
        Ok(())
    }

    /// Helper to create a background thread that calls `JobRunner::run` repeatedly.
    fn make_runner_thread(
        &self,
        job_runner: JobRunner,
        paused: Arc<AtomicBool>,
    ) -> (
        Arc<JobRunner>,
        JoinHandle<std::result::Result<(), Arc<anyhow::Error>>>,
    ) {
        let start_time = Instant::now();
        let first_error_duration_bits = self.first_error_duration_bits.clone();
        let kill_suite = {
            let a = self.alive.clone();
            move || {
                a.store(false, Ordering::SeqCst);
            }
        };

        let job_runner = Arc::new(job_runner);
        let refresh_ms = Duration::from_millis(job_runner.job.refresh_ms as u64);
        let reconnect_grace = self.reconnect_grace.clone();
        let io_policy = self.io_policy.clone();

        // The actual thread loop
        job_runner.state.store(WorkerState::Active);
        let handle = {
            let job_runner = job_runner.clone();
            std::thread::spawn(move || {
                let _worker = WorkerGuard(&job_runner.state);
                match Self::job_runner_worker(
                    paused,
                    refresh_ms,
                    reconnect_grace,
                    io_policy,
                    &job_runner,
                ) {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        kill_suite();
                        let e = Arc::new(e);
                        if !job_runner.errored() {
                            *job_runner.worker_error.write() = Some(e.clone());
                        }
                        first_error_duration_bits.store(
                            start_time.elapsed().as_secs_f64().to_bits(),
                            Ordering::Relaxed,
                        );

                        Err(e)
                    }
                }
            })
        };

        (job_runner, handle)
    }

    fn job_runner_worker(
        paused: Arc<AtomicBool>,
        refresh_ms: Duration,
        reconnect_grace: GraceWindow,
        io_policy: IoPolicy,
        job_runner: &Arc<JobRunner>,
    ) -> Result<(), anyhow::Error> {
        *job_runner.runtime_stats.write() = Conn::make_infos(&job_runner.suite, &job_runner.job)
            .into_iter()
            .map(|info| (info, RuntimeStats::default()))
            .collect();

        let alive = job_runner.alive.clone();

        // A normal job keeps a persistent connection; an `atomic_connection` job
        // opens a fresh one each iteration. Either way it is (re)opened lazily by
        // `with_reconnect`, so a dropped socket just reconnects.
        let mut conn: Option<Conn> = None;

        while alive.load(Ordering::Relaxed) {
            // If paused, sleep until unpaused
            while paused.load(Ordering::Relaxed) && alive.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(100));
            }
            if !alive.load(Ordering::Relaxed) {
                break;
            }

            // An `atomic_connection` job opens a fresh connection each iteration.
            // Dropping it here rather than after `run` keeps it open across the refresh
            // sleep and closes it before the next iteration reopens it.
            if job_runner.job.atomic_connection {
                conn = None;
            }

            // Run one iteration, riding out transient faults. `with_reconnect` drops
            // the (possibly poisoned) connection on error so the next attempt
            // reconnects. If a response was lost, the next iteration may repeat work
            // that PostgreSQL already committed.
            let outcome = with_reconnect(
                &alive,
                &reconnect_grace,
                &mut conn,
                &job_runner.suite,
                &job_runner.job,
                &io_policy,
                |conn| job_runner.run(conn),
            )?;
            if outcome.is_none() {
                break; // `alive` went false while waiting out a fault
            }

            if refresh_ms.as_millis() <= 1000 {
                // if the refresh interval is less than a second, just sleep for that amount of time
                std::thread::sleep(refresh_ms);
            } else {
                // otherwise, sleep in short spurts to allow for detecting "alive" state changes
                // while trying not to sleep too much longer than the refresh interval
                let start = Instant::now();
                while start.elapsed() < refresh_ms {
                    if !alive.load(Ordering::Relaxed) {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
            }
        }

        Ok(())
    }

    pub fn pgver(&self) -> &str {
        &self.pgver
    }

    pub fn suite(&self) -> &Suite {
        &self.suite
    }

    pub fn monitor_runners(&self) -> impl Iterator<Item = Arc<JobRunner>> + '_ {
        self.monitors.iter().cloned()
    }

    pub fn runners(&self) -> impl Iterator<Item = Arc<JobRunner>> + '_ {
        self.runners.iter().cloned()
    }

    pub fn name(&self) -> String {
        self.suite.name()
    }

    /// Return whether the suite has encountered an error in any job
    pub fn errored(&self) -> bool {
        self.have_error.load(Ordering::Relaxed)
    }

    /// How long did this job run before it first had an error
    pub fn first_error_duration(&self) -> Option<Duration> {
        let bits = self.first_error_duration_bits.load(Ordering::Relaxed);
        if bits == 0 {
            return None;
        }
        Some(Duration::from_secs_f64(f64::from_bits(bits)))
    }

    /// Any are jobs currently running a query?
    pub fn any_running(&self) -> bool {
        self.runners
            .iter()
            .chain(self.monitors.iter())
            .any(|job| job.running())
    }

    /// Whether the suite is still active (no fatal errors and not forcibly terminated)
    pub fn alive(&self) -> bool {
        self.alive.load(Ordering::Relaxed)
    }

    /// Ends all job threads, retrying native cancellation for up to 60 seconds.
    pub fn terminate(&self) {
        self.terminate_with_timeout(Duration::from_secs(60));
    }

    /// Stop scheduling work and try native cancellation. Fault-tolerant workers
    /// close any remaining connections after a short cancellation window.
    pub fn terminate_with_timeout(&self, timeout: Duration) {
        let start = Instant::now();
        let cancel_window = if self.io_policy.interrupt_shutdown {
            let window = timeout.min(CANCELLATION_TIMEOUT);
            self.io_policy
                .shutdown
                .write()
                .get_or_insert(start + window);
            window
        } else {
            timeout
        };
        self.alive.store(false, Ordering::Release);
        self.paused.store(false, Ordering::Release);
        // A failed cancellation connection must not prevent other jobs from being canceled
        // or stop the caller from enforcing the shutdown deadline.
        for runner in self.runners().chain(self.monitor_runners()) {
            if !runner.worker_active() {
                continue;
            }
            std::thread::spawn(move || {
                while start.elapsed() < cancel_window && runner.worker_active() {
                    if runner.running() {
                        runner.cancel_query();
                    }
                    let sleep_start = Instant::now();
                    while sleep_start.elapsed() < Duration::from_secs(1) && runner.worker_active() {
                        std::thread::sleep(Duration::from_millis(50));
                    }
                }
            });
        }
    }

    /// Is the suite paused? (applies only to non-monitor jobs)
    pub fn paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    /// Toggle paused <-> running for non-monitor jobs
    pub fn toggle_pause(&self) {
        let new_val = !self.paused();
        self.paused.store(new_val, Ordering::Relaxed);
    }

    pub fn wait_for_finish(&self) -> Vec<Arc<anyhow::Error>> {
        // wait on main
        let mut all_errors = vec![];
        for thread_handle in self.handles.lock().drain(..) {
            match thread_handle.handle.join() {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    let job_title = thread_handle.runner.title();
                    let error_with_title = Arc::new(anyhow!("Job '{}' failed: {}", job_title, e));
                    all_errors.push(error_with_title);
                }
                Err(e) => {
                    let job_title = thread_handle.runner.title();
                    let error_with_title =
                        Arc::new(anyhow!("Job '{}' panicked: {:?}", job_title, e));
                    all_errors.push(error_with_title);
                }
            }
        }

        // Only a `Full` run tears down. A `SkipSetup` workload connects to a schema that setup
        // built in a prior run and that later runs may still use, so it leaves it in place.
        // `SetupOnly` never reaches here; it returns before the workload starts.
        if self.setup_mode == SetupMode::Full {
            for server in &mut self.suite.all_servers() {
                let mut teardown_job = server.teardown.clone();
                teardown_job.destinations = vec![StatementDestination::SpecificServers(vec![
                    server.name.clone(),
                ])];

                let result = JobRunner::new(
                    self.suite.clone(),
                    teardown_job,
                    false,
                    self.alive.clone(),
                    self.sys.clone(),
                )
                .and_then(|runner| self.run_once(&runner.job, |conn| runner.run(conn)));
                if let Err(error) = result {
                    all_errors.push(Arc::new(
                        error.context(format!("Teardown for '{}' failed", server.name)),
                    ));
                }
            }
        }

        if !all_errors.is_empty() {
            self.have_error.store(true, Ordering::Relaxed);
        }

        all_errors
    }
}

pub const HARDCODED_IGNORE_ERRORS: &[&str] = &[
    "failed to merge: User requested cancel",
    "Merge cancelled",
    "canceling statement due to conflict with recovery",
    "(SQLState: 57014)", // query_canceled
];

type Index = usize;

#[derive(Debug, Clone)]
pub struct RuntimeStats {
    pub count: usize,
    cumulative_duration: Duration,
    pub cpu_usage: f32,
    pub mem_usage: u64,
    pub results: std::result::Result<Vec<Row>, Arc<postgres::Error>>,
    pub assert_error: Option<String>,
}

impl Default for RuntimeStats {
    fn default() -> Self {
        Self {
            count: 0,
            cumulative_duration: Default::default(),
            cpu_usage: 0.0,
            mem_usage: 0,
            results: Ok(Vec::new()),
            assert_error: None,
        }
    }
}

impl RuntimeStats {
    pub fn tps(&self) -> f64 {
        let tps = self.count as f64 / self.cumulative_duration.as_secs_f64();
        if !tps.is_normal() { 0.0 } else { tps }
    }

    pub fn update(
        &mut self,
        duration: Duration,
        results: PostgresResult<Vec<Row>>,
        assert_error: Option<String>,
    ) {
        self.count += 1;
        self.cumulative_duration += duration;
        self.results = results;
        self.assert_error = assert_error;
    }
}

/// Lifecycle state of a [`JobRunner`]'s worker thread.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WorkerState {
    /// Worker thread has not been spawned or has exited.
    Stopped = 0,
    /// Worker thread is alive (connecting, reconnecting, or sleeping between iterations).
    Active = 1,
    /// Worker thread is actively executing SQL statements.
    RunningQuery = 2,
}

struct AtomicWorkerState(AtomicU8);

impl AtomicWorkerState {
    fn new(state: WorkerState) -> Self {
        Self(AtomicU8::new(state as u8))
    }

    fn load(&self) -> WorkerState {
        match self.0.load(Ordering::Acquire) {
            0 => WorkerState::Stopped,
            1 => WorkerState::Active,
            2 => WorkerState::RunningQuery,
            other => unreachable!("invalid WorkerState discriminant: {other}"),
        }
    }

    fn store(&self, state: WorkerState) {
        self.0.store(state as u8, Ordering::Release);
    }

    fn swap(&self, state: WorkerState) -> WorkerState {
        match self.0.swap(state as u8, Ordering::AcqRel) {
            0 => WorkerState::Stopped,
            1 => WorkerState::Active,
            2 => WorkerState::RunningQuery,
            other => unreachable!("invalid WorkerState discriminant: {other}"),
        }
    }
}

struct WorkerGuard<'a>(&'a AtomicWorkerState);

impl Drop for WorkerGuard<'_> {
    fn drop(&mut self) {
        self.0.store(WorkerState::Stopped);
    }
}

struct QueryGuard<'a> {
    state: &'a AtomicWorkerState,
    prev: WorkerState,
}

impl<'a> QueryGuard<'a> {
    fn enter(state: &'a AtomicWorkerState) -> Self {
        let prev = state.swap(WorkerState::RunningQuery);
        Self { state, prev }
    }
}

impl Drop for QueryGuard<'_> {
    fn drop(&mut self) {
        self.state.store(self.prev);
    }
}

#[derive(Clone)]
struct CancelTarget {
    server_name: String,
    endpoint: String,
    pid: BackendPid,
    token: tokio_postgres::CancelToken,
}

// Log only the configured hosts and ports, never connection credentials or cancel keys.
fn connection_endpoint(connstr: &str) -> String {
    match tokio_connstr(connstr).parse::<postgres::Config>() {
        Ok(config) => format!("{:?}:{:?}", config.get_hosts(), config.get_ports()),
        Err(_) => "unavailable".to_owned(),
    }
}

/// A single job’s runtime state.
pub struct JobRunner {
    suite: Arc<Suite>,
    index: Index,
    job: Job,
    is_monitor: bool,
    alive: Arc<AtomicBool>,
    state: AtomicWorkerState,
    paused: AtomicBool,
    worker_error: RwLock<Option<Arc<anyhow::Error>>>,

    last_conn_generation: AtomicU64,
    runtime_stats: RwLock<BTreeMap<ConnInfo, RuntimeStats>>,
    cancel_targets: RwLock<Vec<CancelTarget>>,

    sys: Arc<RwLock<System>>,
}

fn postgres_anyhow(error: postgres::Error) -> anyhow::Error {
    if error.as_db_error().is_some() {
        let msg = format_postgres_error(&error);
        anyhow::Error::new(error).context(msg)
    } else {
        anyhow::Error::new(error)
    }
}

impl JobRunner {
    /// Create a new runner for the given job.
    pub fn new(
        suite: Arc<Suite>,
        job: Job,
        is_monitor: bool,
        alive: Arc<AtomicBool>,
        sys: Arc<RwLock<System>>,
    ) -> Result<Self> {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        Ok(Self {
            suite: suite.clone(),
            index: COUNTER.fetch_add(1, Ordering::Relaxed),
            job,
            is_monitor,
            alive,
            state: AtomicWorkerState::new(WorkerState::Stopped),
            paused: AtomicBool::new(false),
            worker_error: Default::default(),
            last_conn_generation: AtomicU64::new(0),
            runtime_stats: Default::default(),
            cancel_targets: Default::default(),
            sys,
        })
    }

    pub fn view_id(&self, conninfo: &ConnInfo, label: &str) -> String {
        format!(
            "{}:{}-{}-{}",
            label,
            conninfo.id,
            conninfo.name(),
            self.index
        )
    }

    /// Get the underlying `Job`.
    pub fn job(&self) -> &Job {
        &self.job
    }

    /// Returns true if this is the Monitor job
    pub fn is_monitor(&self) -> bool {
        self.is_monitor
    }

    pub fn is_select(&self) -> bool {
        self.job.is_select()
    }

    /// The job's (friendly) title.
    pub fn title(&self) -> String {
        self.job.title()
    }

    pub fn runtime_stats(&self) -> BTreeMap<ConnInfo, RuntimeStats> {
        self.runtime_stats.read().clone()
    }

    /// Calculate the CPU usage of the connected database processes, by pid.
    ///
    /// The CPU usage is a percentage and is calculated in real-time when
    /// this function is called, but only if this [`JobRunner`] is actively running
    /// a query at this moment.
    ///
    /// The CPU usage is stored internally and the last-known value will be returned in the
    /// cases where the job isn't actively running a query.
    fn update_sysinfo_stats(&self) {
        #[inline(always)]
        fn sys_info(pid: BackendPid, sys: &System) -> Option<(f32, u64)> {
            let process = sys.process(Pid::from_u32(pid as u32))?;
            Some((process.cpu_usage(), process.memory()))
        }

        for (conninfo, stats) in &mut self.runtime_stats.write().iter_mut() {
            if self.running()
                && let Some((cpu_usage, mem_usage)) = sys_info(conninfo.pid(), &self.sys.read())
            {
                if cpu_usage > 0.0 {
                    stats.cpu_usage = cpu_usage;
                }
                stats.mem_usage = mem_usage;
            }
        }
    }

    /// Current lifecycle state of this job's worker thread.
    pub fn worker_state(&self) -> WorkerState {
        self.state.load()
    }

    pub fn log_shutdown_diagnostics(&self) {
        let targets = self.cancel_targets.read();
        eprintln!(
            "shutdown worker job={} state={:?} connection_generation={} cancel_targets={}",
            self.title(),
            self.worker_state(),
            self.last_conn_generation.load(Ordering::Relaxed),
            targets.len()
        );
        for target in targets.iter() {
            eprintln!(
                "shutdown connection job={} server={} configured_endpoint={} pid={}",
                self.title(),
                target.server_name,
                target.endpoint,
                target.pid
            );
        }
    }

    /// Is this job's worker thread still running?
    pub fn worker_active(&self) -> bool {
        self.worker_state() != WorkerState::Stopped
    }

    /// Is the job currently running a query?
    pub fn running(&self) -> bool {
        self.worker_state() == WorkerState::RunningQuery
    }

    /// Is the job paused?
    pub fn paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    /// Did this job encounter an error?
    pub fn errored(&self) -> bool {
        self.worker_error.read().is_some()
            || self
                .runtime_stats
                .read()
                .iter()
                .any(|(_, stats)| stats.results.is_err() || stats.assert_error.is_some())
    }

    /// Did this job encounter an error?
    pub fn connection_errored(&self, conninfo: &ConnInfo) -> bool {
        self.runtime_stats
            .read()
            .iter()
            .filter(|(this_conninfo, _)| *this_conninfo == conninfo)
            .any(|(_, stats)| stats.results.is_err() || stats.assert_error.is_some())
    }

    pub fn worker_error(&self) -> Option<Arc<anyhow::Error>> {
        self.worker_error.read().clone()
    }

    /// Best-effort cancellation of the job's active queries.
    pub fn cancel_query(&self) {
        if !self.running() {
            return;
        }
        // Tokens retain the original connection's endpoint and cancellation key.
        // Unlike a SQL session, a cancel request can be sent during smart shutdown.
        let targets = self.cancel_targets.read().clone();
        let runtime = match Builder::new_current_thread().enable_all().build() {
            Ok(runtime) => runtime,
            Err(error) => {
                eprintln!("could not start cancellation runtime: {error}");
                return;
            }
        };
        for target in targets {
            let started = Instant::now();
            eprintln!(
                "sending cancellation job={} state={:?} server={} configured_endpoint={} pid={}",
                self.title(),
                self.worker_state(),
                target.server_name,
                target.endpoint,
                target.pid
            );
            let result = runtime.block_on(async {
                tokio::time::timeout(
                    CANCELLATION_TIMEOUT,
                    target.token.cancel_query(Conn::tls_connector()),
                )
                .await?
                .map_err(anyhow::Error::new)
            });
            if let Err(error) = result {
                eprintln!(
                    "could not send cancellation job={} state={:?} server={} configured_endpoint={} pid={} elapsed_ms={}: {error:#}",
                    self.title(),
                    self.worker_state(),
                    target.server_name,
                    target.endpoint,
                    target.pid,
                    started.elapsed().as_millis(),
                );
            }
        }
    }

    /// Toggle the Job's paused state
    pub fn toggle_pause(&self) {
        let new_val = !self.paused();
        self.paused.store(new_val, Ordering::Relaxed);
    }

    /// Execute this job once using the provided connection.
    pub fn run(&self, conn: &mut Conn) -> Result<()> {
        while self.paused() {
            if !self.alive.load(Ordering::Relaxed) {
                return Ok(());
            }

            std::thread::yield_now();
        }

        // Refresh cancellation tokens and backend IDs whenever the connection is opened or
        // replaced after a reconnect, before the first query on it can block.
        if self
            .last_conn_generation
            .swap(conn.generation.0, Ordering::Relaxed)
            != conn.generation.0
        {
            *self.cancel_targets.write() = conn
                .clients
                .iter()
                .map(|(client, server, pid)| CancelTarget {
                    server_name: server.name.clone(),
                    endpoint: connection_endpoint(&server.connstr()),
                    pid: *pid,
                    token: client.client.cancel_token(),
                })
                .collect();

            let mut stats = self.runtime_stats.write();
            for (id, (_, server, pid)) in conn.clients.iter().enumerate() {
                let info = ConnInfo::new(id, server, *pid);
                if let Some((existing, _)) = stats.get_key_value(&info) {
                    *existing.meta.write() = info.meta.read().clone();
                } else {
                    stats.insert(info, RuntimeStats::default());
                }
            }
        }

        let statements = self.job.sql();

        let mut final_result: Option<Vec<(PostgresResult<Vec<Row>>, Duration, ConnInfo)>> = None;

        // If first statement is "BEGIN", treat subsequent statements until "COMMIT"
        // inside a transaction.
        let _running = QueryGuard::enter(&self.state);
        if statements
            .first()
            .map(|s| s.sql.to_ascii_lowercase().starts_with("begin"))
            .unwrap_or(false)
        {
            let mut idx = 0;
            while idx < statements.len() {
                if statements[idx]
                    .sql
                    .to_ascii_lowercase()
                    .starts_with("begin")
                {
                    idx += 1;
                    if idx >= statements.len() {
                        break;
                    }
                }
                let commit_res = {
                    let mut tx = match conn.transaction() {
                        Ok(tx) => tx,
                        Err(e) if is_ignorable_error(&e, self.suite.ignore_errors()) => {
                            return Ok(());
                        }
                        Err(e) => return Err(postgres_anyhow(e)),
                    };
                    let mut should_commit = true;
                    while idx < statements.len() {
                        let stmt = &statements[idx];
                        idx += 1;
                        let sql = stmt.sql.to_ascii_lowercase();
                        if sql.starts_with("commit") {
                            break;
                        } else if sql.starts_with("abort") || sql.starts_with("rollback") {
                            should_commit = false;
                            break;
                        }

                        let results = tx.query(stmt.sql, &[]);
                        let had_err = results.iter().any(|(r, ..)| r.is_err());
                        final_result = Some(results);
                        if had_err {
                            should_commit = false;
                            idx = statements.len();
                            break;
                        }
                    }
                    if should_commit {
                        tx.commit()
                    } else {
                        drop(tx);
                        Ok(())
                    }
                };
                if let Err(e) = commit_res {
                    if is_ignorable_error(&e, self.suite.ignore_errors()) {
                        conn.rollback_best_effort();
                        return Ok(());
                    }
                    return Err(postgres_anyhow(e));
                }
                while idx < statements.len() {
                    let sql = statements[idx].sql.to_ascii_lowercase();
                    if sql.starts_with("commit")
                        || sql.starts_with("abort")
                        || sql.starts_with("rollback")
                    {
                        idx += 1;
                    } else {
                        break;
                    }
                }
            }
        } else {
            for stmt in statements {
                final_result = Some(conn.query(stmt.sql, &[]));
                if final_result
                    .as_ref()
                    .unwrap()
                    .iter()
                    .any(|(results, ..)| results.is_err())
                {
                    break;
                }
            }
        }
        drop(_running);

        let mut last_error = None;
        if let Some(final_result) = final_result {
            for (mut rows, duration, conninfo) in final_result {
                let mut assert_error = None;
                if rows.is_err() {
                    rows = rows.inspect_err(|e| {
                        if !is_ignorable_error(e, self.suite.ignore_errors())
                            && last_error
                                .as_ref()
                                .is_none_or(|previous: &Arc<postgres::Error>| {
                                    is_transient_connection_error(previous)
                                })
                        {
                            last_error = Some(Clone::clone(e));
                        }
                    });
                } else if let Some(assert) = &self.job.assert {
                    let tmp = rows.clone()?;
                    if let Some(first_row) = tmp.first() {
                        let value = first_row.get::<_, i64>(0);
                        if value != i64::from_str(assert)? {
                            assert_error = Some(format!(
                                "Job assertion failed: expected {assert} but got {value}"
                            ));
                        }
                    } else {
                        assert_error =
                            Some("cannot evaluate job `assert` due to no rows returned".to_owned());
                    }
                }

                let mut runtime_stats = self.runtime_stats.write();
                match runtime_stats.entry(conninfo.clone()) {
                    Entry::Vacant(vac) => {
                        let mut new_stats = RuntimeStats::default();
                        new_stats.update(duration, rows, assert_error.clone());
                        vac.insert(new_stats);
                    }
                    Entry::Occupied(mut occ) => {
                        occ.get_mut().update(duration, rows, assert_error.clone());
                    }
                }

                if let Some(assert_error) = assert_error {
                    return Err(anyhow!(assert_error));
                }
            }

            if let Some(last_error) = last_error {
                let msg = format_postgres_error(&last_error);
                return Err(anyhow::Error::new(last_error).context(msg));
            }
        }

        Ok(())
    }
}

fn is_ignorable_error(e: &postgres::Error, ignore_patterns: &[String]) -> bool {
    // user cancel request
    if e.as_db_error()
        .map(|dberror| dberror.code() == &SqlState::from_code("57014"))
        .unwrap_or_default()
    {
        return true;
    }

    let error_string = e.to_string();
    for pattern in HARDCODED_IGNORE_ERRORS {
        if error_string.contains(pattern) {
            return true;
        }
    }
    for pattern in ignore_patterns {
        if error_string.contains(pattern) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::{
        AtomicWorkerState, IoPolicy, QueryGuard, WorkerGuard, WorkerState, connection_endpoint,
        tokio_connstr,
    };
    use std::time::{Duration, Instant};

    #[test]
    fn io_deadline_preserves_a_ready_response_and_bounds_a_stalled_response() {
        let policy = IoPolicy {
            operation_timeout: Some(Duration::from_millis(10)),
            ..IoPolicy::default()
        };
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        runtime.block_on(async {
            assert_eq!(
                tokio::time::timeout(
                    Duration::from_secs(2),
                    policy.wait(std::future::pending::<()>()),
                )
                .await
                .unwrap(),
                None
            );
            *policy.shutdown.write() = Some(Instant::now());
            assert_eq!(
                policy.wait(std::future::ready("SQL response")).await,
                Some("SQL response")
            );
        });
    }

    #[test]
    fn shutdown_endpoint_omits_credentials_and_options() {
        for connstr in [
            "postgresql://private_user:private_password@localhost:5433/db?options=-csecret=value",
            "host=localhost port=5433 user=private_user password=private_password options=-csecret=value",
        ] {
            let endpoint = connection_endpoint(connstr);
            assert!(endpoint.contains("localhost"), "{endpoint}");
            assert!(endpoint.contains("5433"), "{endpoint}");
            assert!(!endpoint.contains("private"), "{endpoint}");
            assert!(!endpoint.contains("secret"), "{endpoint}");
        }
    }

    #[test]
    fn worker_and_query_guards_restore_state_across_returns_and_panics() {
        let state = AtomicWorkerState::new(WorkerState::Stopped);
        assert_eq!(state.load(), WorkerState::Stopped);

        // One-shot runners (setup/teardown) transition Stopped -> RunningQuery -> Stopped.
        {
            let _query = QueryGuard::enter(&state);
            assert_eq!(state.load(), WorkerState::RunningQuery);
        }
        assert_eq!(state.load(), WorkerState::Stopped);

        // Spawned workers transition Active -> RunningQuery -> Active, and unwind to Stopped.
        state.store(WorkerState::Active);
        let unwind_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _worker = WorkerGuard(&state);
            {
                let _query = QueryGuard::enter(&state);
                assert_eq!(state.load(), WorkerState::RunningQuery);
            }
            assert_eq!(state.load(), WorkerState::Active);

            let _query = QueryGuard::enter(&state);
            panic!("simulated failure inside query");
        }));
        assert!(unwind_result.is_err());
        assert_eq!(state.load(), WorkerState::Stopped);
    }

    #[test]
    fn tokio_connstr_translates_only_the_keepalive_probe_count() {
        // libpq's keepalives_count becomes tokio-postgres's keepalives_retries; every
        // other keyword (including the identically-spelled keepalives_* and
        // tcp_user_timeout) is left untouched.
        assert_eq!(
            tokio_connstr(
                "postgresql://h:5432/db?connect_timeout=5&keepalives=1&keepalives_idle=5\
                 &keepalives_interval=2&keepalives_count=3&tcp_user_timeout=15"
            ),
            "postgresql://h:5432/db?connect_timeout=5&keepalives=1&keepalives_idle=5\
             &keepalives_interval=2&keepalives_retries=3&tcp_user_timeout=15"
        );
        // A string that never mentions the probe count is returned verbatim.
        assert_eq!(
            tokio_connstr("host=localhost port=5432 dbname=stressgres"),
            "host=localhost port=5432 dbname=stressgres"
        );
    }
}

#[cfg(test)]
mod shutdown_test;
