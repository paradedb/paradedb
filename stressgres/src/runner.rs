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

use crate::sqlscanner::StatementDestination;
use crate::suite::{Job, Server, ServerStyle, Suite};
use anyhow::{anyhow, Result};
use cursive_core::style::{BaseColor, Color};
use parking_lot::{Mutex, RwLock};
use postgres::error::SqlState;
use postgres::types::ToSql;
use postgres::Row;
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use sysinfo::{Pid, ProcessesToUpdate, System};

type PostgresResult<T> = std::result::Result<T, Arc<postgres::Error>>;

pub type BackendPid = i32;

/// A thread handle paired with its corresponding JobRunner for error reporting
struct ThreadHandle {
    handle: JoinHandle<std::result::Result<(), Arc<anyhow::Error>>>,
    runner: Arc<JobRunner>,
}

pub struct Conn {
    server_lookup: Arc<HashMap<String, Server>>,
    clients: Vec<(postgres::Client, Server, BackendPid)>,
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
    transactions: Vec<(postgres::Transaction<'a>, &'a Server, BackendPid)>,
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

    pub fn open(suite: &Suite, job: &Job) -> Result<Self> {
        let clients = job
            .destinations()
            .iter()
            .map(|destination| match destination {
                StatementDestination::DefaultServer => {
                    let server = suite.default_server();
                    Ok(vec![(Conn::_open_conn(server, job)?, server.clone())])
                }
                StatementDestination::SpecificServers(server_names) => server_names
                    .iter()
                    .map(|name| {
                        let server = suite
                            .server(name)
                            .ok_or_else(|| anyhow!("No such server named `{name}`"))?;
                        Ok((Conn::_open_conn(server, job)?, server.clone()))
                    })
                    .collect::<Result<Vec<_>>>(),
                StatementDestination::AllServers => suite
                    .all_servers()
                    .map(|server| Ok((Conn::_open_conn(server, job)?, server.clone())))
                    .collect::<Result<Vec<_>>>(),
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .map(|(mut conn, server)| {
                let pid: BackendPid = conn.query_one("SELECT pg_backend_pid();", &[])?.get(0);
                Ok((conn, server, pid))
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            server_lookup: suite.server_lookup(),
            clients,
        })
    }

    pub fn first_client(&mut self) -> &mut postgres::Client {
        &mut self.clients[0].0
    }

    pub fn transaction(&mut self) -> Result<MultiTransaction<'_>> {
        Ok(MultiTransaction {
            server_lookup: self.server_lookup.clone(),
            transactions: self
                .clients
                .iter_mut()
                .map(|(client, ref server, pid)| {
                    client
                        .transaction()
                        .map(|xact| (xact, server, *pid))
                        .map_err(Arc::new)
                })
                .collect::<PostgresResult<Vec<_>>>()?,
        })
    }

    pub fn clients(&mut self) -> impl Iterator<Item = &mut postgres::Client> {
        self.clients.iter_mut().map(|(client, ..)| client)
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
            let result = client.query(&query, params).map_err(Arc::new);
            let duration = start.elapsed();
            results.push((result, duration, ConnInfo::new(id, server, *pid)));
        }

        results
    }

    pub fn execute(
        &mut self,
        query: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Vec<PostgresResult<u64>> {
        let mut results = Vec::new();
        for client in self.clients() {
            results.push(client.execute(query, params).map_err(Arc::new));
        }
        results
    }

    #[doc(hidden)]
    pub fn _open_conn(server: &Server, job: &Job) -> Result<postgres::Client> {
        let sanitized = job.title().replace(|c: char| !c.is_alphanumeric(), "_");
        let connstr = match server.style {
            // For ServerStyle::With, the connection string is expected to already
            // include the application_name if needed, so it is not appended here.
            ServerStyle::With { .. } => server.connstr(),
            _ => format!("{} application_name={}", server.connstr(), sanitized),
        };
        let mut conn = postgres::Client::connect(&connstr, {
            use openssl::ssl::{SslConnector, SslMethod};
            use postgres_openssl::MakeTlsConnector;
            let mut builder =
                SslConnector::builder(SslMethod::tls()).expect("ssl builder should not fail");
            builder.set_verify(openssl::ssl::SslVerifyMode::NONE);
            MakeTlsConnector::new(builder.build())
        })?;

        for query in job.on_connect() {
            conn.execute(query.sql, &[])?;
        }

        Ok(conn)
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

        for (id, (xact, server, pid)) in self.transactions.iter_mut().enumerate() {
            let start = Instant::now();
            results.push((
                xact.query(&query, params).map_err(Arc::new),
                start.elapsed(),
                ConnInfo::new(id, server, *pid),
            ));
        }

        results
    }

    pub fn commit(self) -> Result<()> {
        for (xact, ..) in self.transactions {
            xact.commit()?;
        }
        Ok(())
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

/// Manages the entire suite: runs setup once, spawns each job's thread, optionally a monitor job,
/// and finally runs teardown when finished.
pub struct SuiteRunner {
    suite: Arc<Suite>,
    pgver: String,
    alive: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    handles: Mutex<Vec<ThreadHandle>>,
    runners: Vec<Arc<JobRunner>>,
    have_error: Arc<AtomicBool>,
    first_error_duration_bits: Arc<AtomicU64>,

    monitors: Vec<Arc<JobRunner>>,
    sys: Arc<RwLock<System>>,
}

impl SuiteRunner {
    /// Create a new SuiteRunner, run the `setup` job, then spawn threads for each job + monitor.
    pub fn new(suite: Suite, paused: bool) -> Result<Arc<Self>> {
        let suite = Arc::new(suite);
        let mut runner = Self {
            suite: suite.clone(),
            pgver: String::from("<unknown>"),
            alive: Arc::new(AtomicBool::new(true)),
            paused: Arc::new(AtomicBool::new(paused)),
            handles: Default::default(),
            runners: Default::default(),
            have_error: Arc::new(AtomicBool::new(false)),
            first_error_duration_bits: Default::default(),

            monitors: Default::default(),
            sys: Arc::new(RwLock::new(System::new_all())),
        };

        runner.pgver = Conn::open(&suite, &Job::default())?
            .first_client()
            .query_one("SELECT version()", &[])?
            .get(0);

        runner.init()?;
        let suite_runner = Arc::new(runner);

        // refresh sysinfo stats very frequently
        {
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

    /// Perform setup job, optionally create a monitor job, and spawn all main job threads.
    fn init(&mut self) -> Result<()> {
        // 1. Run setup job (always present)
        for server in self.suite.all_servers() {
            let mut setup_job = server.setup.clone();
            eprintln!(
                "Running setup job for {} on {}",
                server.name,
                server.connstr()
            );
            setup_job.destinations = vec![StatementDestination::SpecificServers(vec![server
                .name
                .clone()])];

            let setup_runner = JobRunner::new(
                self.suite.clone(),
                setup_job,
                false,
                self.alive.clone(),
                self.sys.clone(),
            )?;
            setup_runner.run(&mut Conn::open(&self.suite, &setup_runner.job)?)?;
        }

        // setup and start the monitors
        for server in self.suite.all_servers() {
            let mut monitor_job = server.monitor.clone();
            monitor_job.destinations = vec![StatementDestination::SpecificServers(vec![server
                .name
                .clone()])];

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

        // The actual thread loop
        let handle = {
            let job_runner = job_runner.clone();
            std::thread::spawn(move || {
                match Self::job_runner_worker(paused, refresh_ms, &job_runner) {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        job_runner.running.store(false, Ordering::Relaxed);
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
        job_runner: &Arc<JobRunner>,
    ) -> Result<(), anyhow::Error> {
        *job_runner.runtime_stats.write() = Conn::make_infos(&job_runner.suite, &job_runner.job)
            .into_iter()
            .map(|info| (info, RuntimeStats::default()))
            .collect();

        let mut conn = if job_runner.job.atomic_connection {
            None
        } else {
            Some(Conn::open(&job_runner.suite, &job_runner.job)?)
        };

        let alive = job_runner.alive.clone();
        while alive.load(Ordering::Relaxed) {
            // If paused, sleep until unpaused
            while paused.load(Ordering::Relaxed) && alive.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(100));
            }
            if !alive.load(Ordering::Relaxed) {
                break;
            }

            let conn = match &mut conn {
                Some(conn) => conn,
                None => &mut Conn::open(&job_runner.suite, &job_runner.job)?,
            };
            job_runner.run(conn)?;

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

    /// Ends all job threads
    pub fn terminate(&self) {
        self.alive.store(false, Ordering::Relaxed);
        self.paused.store(false, Ordering::Relaxed);
        for runner in self.runners() {
            runner.cancel_query();
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

    pub fn wait_for_finish(&self) -> Result<Vec<Arc<anyhow::Error>>> {
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

        for server in &mut self.suite.all_servers() {
            let mut teardown_job = server.teardown.clone();
            teardown_job.destinations = vec![StatementDestination::SpecificServers(vec![server
                .name
                .clone()])];

            let teardown_runner = JobRunner::new(
                self.suite.clone(),
                teardown_job,
                false,
                self.alive.clone(),
                self.sys.clone(),
            )?;
            teardown_runner.run(&mut Conn::open(&self.suite, &teardown_runner.job)?)?;
        }

        for job in &self.runners {
            let job_errors = job.collect_errors().into_iter().filter(|e| {
                e.source()
                    .and_then(|e| e.downcast_ref::<postgres::Error>())
                    .map(is_ignorable_error)
                    .unwrap_or(false)
            });

            // Add job title to each error from this job
            for error in job_errors {
                let job_title = job.title();
                let error_with_title = Arc::new(anyhow!("Job '{}' error: {}", job_title, error));
                all_errors.push(error_with_title);
            }
        }

        if !all_errors.is_empty() {
            self.have_error.store(true, Ordering::Relaxed);
        }

        Ok(all_errors)
    }
}

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
        if !tps.is_normal() {
            0.0
        } else {
            tps
        }
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

/// A single job’s runtime state.
pub struct JobRunner {
    suite: Arc<Suite>,
    index: Index,
    job: Job,
    is_monitor: bool,
    alive: Arc<AtomicBool>,
    running: AtomicBool,
    paused: AtomicBool,
    worker_error: RwLock<Option<Arc<anyhow::Error>>>,

    runtime_stats: RwLock<BTreeMap<ConnInfo, RuntimeStats>>,

    sys: Arc<RwLock<System>>,
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
            running: AtomicBool::new(false),
            paused: AtomicBool::new(false),
            worker_error: Default::default(),
            runtime_stats: Default::default(),
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
            if self.running() {
                if let Some((cpu_usage, mem_usage)) = sys_info(conninfo.pid(), &self.sys.read()) {
                    if cpu_usage > 0.0 {
                        stats.cpu_usage = cpu_usage;
                    }
                    stats.mem_usage = mem_usage;
                }
            }
        }
    }

    /// Is the job currently running a query?
    pub fn running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
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

    pub fn collect_errors(&self) -> Vec<Arc<anyhow::Error>> {
        self.worker_error
            .read()
            .clone()
            .into_iter()
            .chain(self.runtime_stats.read().iter().filter_map(|(_, stats)| {
                if stats.assert_error.is_some() {
                    Some(Arc::new(anyhow!(stats.assert_error.clone().unwrap())))
                } else if stats.results.is_err() {
                    let postgres_error = Clone::clone(stats.results.as_ref().err().unwrap());
                    let msg = if let Some(db_error) = postgres_error.as_db_error() {
                        format!(
                            "{} (SQLState: {})",
                            db_error.message(),
                            db_error.code().code()
                        )
                    } else {
                        postgres_error.to_string()
                    };
                    Some(Arc::new(anyhow!(msg)))
                } else {
                    None
                }
            }))
            .collect()
    }

    pub fn worker_error(&self) -> Option<Arc<anyhow::Error>> {
        self.worker_error.read().clone()
    }

    /// A best efforts attempt to cancel the query that this Job might currently be running.
    ///
    /// No errors are reported, even if the attempt to cancel failed in some generally unexpected way
    pub fn cancel_query(&self) {
        if self.running() {
            // open a new connection to everywhere the job is configured to connect to
            if let Ok(mut conn) = Conn::open(&self.suite, &self.job) {
                for conninfo in &mut self.runtime_stats.read().keys() {
                    // this is a best efforts to cancel the pid.
                    conn.execute("SELECT pg_cancel_backend($1);", &[&conninfo.pid()]);
                }
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

        let statements = self.job.sql();

        let mut final_result: Option<Vec<(PostgresResult<Vec<Row>>, Duration, ConnInfo)>> = None;

        // If first statement is "BEGIN", treat subsequent statements until "COMMIT"
        // inside a transaction.
        self.running.store(true, Ordering::Relaxed);
        if statements
            .first()
            .map(|s| s.sql.to_ascii_lowercase().starts_with("begin"))
            .unwrap_or(false)
        {
            let mut tx = conn.transaction()?;
            for stmt in &statements[1..] {
                let sql = stmt.sql.to_ascii_lowercase();
                if sql.starts_with("commit") {
                    tx.commit()?;
                    tx = conn.transaction()?;
                    continue;
                } else if sql.starts_with("abort") || sql.starts_with("rollback") {
                    drop(tx);
                    tx = conn.transaction()?;
                    continue;
                }

                final_result = Some(tx.query(stmt.sql, &[]));
                if final_result
                    .as_ref()
                    .unwrap()
                    .iter()
                    .any(|(results, ..)| results.is_err())
                {
                    break;
                }
            }
            tx.commit()?;
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
        self.running.store(false, Ordering::Relaxed);

        let mut last_error = None;
        if let Some(final_result) = final_result {
            for (mut rows, duration, conninfo) in final_result {
                let mut assert_error = None;
                if rows.is_err() {
                    rows = rows.inspect_err(|e| {
                        if !is_ignorable_error(e) {
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
                        *occ.key().meta.write() = conninfo.meta.read().clone();
                        occ.get_mut().update(duration, rows, assert_error.clone());
                    }
                }

                if let Some(assert_error) = assert_error {
                    return Err(anyhow!(assert_error));
                }
            }

            if let Some(last_error) = last_error {
                let msg = if let Some(db_error) = last_error.as_db_error() {
                    format!(
                        "{} (SQLState: {})",
                        db_error.message(),
                        db_error.code().code()
                    )
                } else {
                    last_error.to_string()
                };
                return Err(anyhow!(msg));
            }
        }

        Ok(())
    }
}

fn is_ignorable_error(e: &postgres::Error) -> bool {
    // user cancel request
    e.as_db_error()
        .map(|dberror| dberror.code() == &SqlState::from_code("57014"))
        .unwrap_or_default()
}
