use crate::runner::Conn;
use crate::suite::{Job, PostgresqlConf, Server, ServerStyle};
use anyhow::anyhow;
use pgrx_pg_config::PgConfig;
use std::ffi::OsStr;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{ChildStderr, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug)]
struct InnerChild {
    log_path: PathBuf,
    pid: libc::pid_t,
    _stdin: ChildStdin,
    _stdout: ChildStdout,
    _stderr: ChildStderr,
}

#[derive(Debug)]
pub struct ServerHandler {
    child: Option<InnerChild>,
}

impl ServerHandler {
    pub fn pid(&self) -> Option<libc::pid_t> {
        self.child.as_ref().map(|inner| inner.pid)
    }

    pub fn kill(self) {
        if let Some(child) = self.child {
            unsafe {
                let pid = child.pid;
                drop(child);
                libc::kill(pid as libc::pid_t, libc::SIGTERM);
                std::thread::sleep(std::time::Duration::from_millis(333));
                libc::kill(pid as libc::pid_t, libc::SIGKILL);
            }
        }
    }

    pub fn log_contains(&self, msg: &str) -> std::io::Result<bool> {
        match &self.child {
            None => Ok(false),
            Some(inner) => {
                let logfile = std::fs::read_to_string(&inner.log_path)?;
                Ok(logfile.contains(msg))
            }
        }
    }
}

pub fn setup_server(server: &Server, other_servers: &Vec<Server>) -> anyhow::Result<ServerHandler> {
    if let ServerStyle::Automatic {
        pg_config,
        port,
        log_path,
        pgdata,
        postgresql_conf,
    } = &server.style
    {
        let pg_config = pg_config.pg_config(Some(*port));

        let pg_data = pgdata.clone().unwrap_or_else(|| {
            PathBuf::from("/")
                .join("tmp")
                .join("stressgres")
                .join(format!("{}.data", server.name))
        });
        let log_path = log_path.clone().unwrap_or_else(|| {
            PathBuf::from("/")
                .join("tmp")
                .join("stressgres")
                .join(format!("{}.log", server.name))
        });

        let set_opts = postgresql_conf
            .lines()
            .flat_map(|s| s.lines().map(|line| ["-c", line].into_iter()))
            .flatten()
            .collect::<Vec<_>>();

        std::fs::remove_file(&log_path).ok();
        std::fs::remove_dir_all(&pg_data).ok();

        let is_wal_reciver = matches!(postgresql_conf, PostgresqlConf::WalReceiver);

        if is_wal_reciver {
            let mut subscriber_server = None;
            for server in other_servers {
                if server.is_subscriber() {
                    subscriber_server = Some(server);
                    break;
                }
            }
            let source_server = subscriber_server
                .expect("To use WalReceiver a Subscriber must already be configured");

            let wait = Arc::new(AtomicBool::new(true));
            let wait_clone = wait.clone();
            let source_server_clone = source_server.clone();
            std::thread::spawn(move || {
                while wait_clone.load(Ordering::Relaxed) {
                    std::thread::yield_now();
                }
                std::thread::sleep(std::time::Duration::from_millis(333));
                let job = Job {
                    on_connect: Some(format!(
                        r#"
                            SELECT pg_create_physical_replication_slot('{}');
                            CHECKPOINT;
                            "#,
                        crate::physical_replication_slot_name!()
                    )),
                    ..Default::default()
                };
                Conn::_open_conn(&source_server_clone, &job).expect(
                    "Failed to create replication slot on the Subscriber while creating WalReceiver",
                );
            });
            pg_basebackup(source_server.port(), &pg_config, &pg_data, &wait)?;
        } else {
            initdb(&pg_config, &pg_data)?;
        }

        let server_handler = start_pg(&pg_config, &pg_data, &log_path, &set_opts)?;
        let start = std::time::Instant::now();

        loop {
            if server_handler.log_contains("ready to accept connections")?
                || server_handler.log_contains("started streaming WAL from primary")?
            {
                break;
            } else if server_handler.log_contains("database system is shut down")? {
                panic!("{} shut down unexpectedly", server.name);
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
            if start.elapsed() > std::time::Duration::from_secs(60) {
                panic!("Postgres took too long to start for:\n{server:#?}");
            }
        }

        if !is_wal_reciver {
            createdb(&pg_config, "stressgres", &pg_data)?;
        }

        return Ok(server_handler);
    }

    Ok(ServerHandler { child: None })
}

fn initdb<P: AsRef<Path>>(pg_config: &PgConfig, pg_data: P) -> anyhow::Result<()> {
    let mut command = make_command(
        pg_config,
        &pg_data,
        pg_config.initdb_path().map_err(|e| anyhow!(e))?,
    )?;
    command.arg(pg_data.as_ref());
    command.args([
        "--set",
        &format!("port={}", pg_config.port().map_err(|e| anyhow!(e))?),
    ]);
    eprintln!("{command:?}");
    let status = command.spawn()?.wait()?;
    if !status.success() {
        panic!("exited with status code: {status}");
    }

    Ok(())
}

fn createdb<P: AsRef<Path>>(pg_config: &PgConfig, dbname: &str, pg_data: P) -> anyhow::Result<()> {
    let mut command = make_command(
        pg_config,
        &pg_data,
        pg_config.createdb_path().map_err(|e| anyhow!(e))?,
    )?;
    command.args(["-h", "localhost", dbname]);

    eprintln!("{command:?}");
    let status = command.spawn()?.wait()?;
    if !status.success() {
        panic!("exited with status code: {status}");
    }

    Ok(())
}

fn start_pg<P: AsRef<Path>>(
    pg_config: &PgConfig,
    pg_data: P,
    log_path: &Path,
    args: &[impl AsRef<OsStr>],
) -> anyhow::Result<ServerHandler> {
    let mut command = make_command(
        pg_config,
        &pg_data,
        pg_config.postmaster_path().map_err(|e| anyhow!(e))?,
    )?;
    command.args(["-c", "unix_socket_directories=/tmp"]);
    command.args([
        "-c",
        &format!(
            "port={}",
            pg_config.port().expect("should be able to get port number")
        ),
        "-c",
        "logging_collector=true",
        "-c",
        &format!("log_directory={}", log_path.parent().unwrap().display()),
        "-c",
        &format!(
            "log_filename={}",
            log_path.file_name().unwrap().to_str().unwrap()
        ),
        "-c",
        &format!(
            "max_worker_processes={}",
            std::thread::available_parallelism()?.get()
        ),
    ]);
    command.args(args);
    command.stdin(Stdio::piped());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    eprintln!("{command:?}");
    let mut child = command.spawn()?;
    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let pid = child.id();
    std::thread::spawn(move || child.wait());

    let mut ready = false;
    let buf = BufReader::new(&mut stderr);
    for line in buf.lines() {
        let line = line?;
        eprintln!("{line}");
        if line.contains("Future log output will appear") {
            ready = true;
            break;
        }
    }
    if !ready {
        panic!("failed to start pg");
    }

    Ok(ServerHandler {
        child: Some(InnerChild {
            log_path: log_path.to_path_buf(),
            pid: pid as libc::pid_t,
            _stdin: stdin,
            _stdout: stdout,
            _stderr: stderr,
        }),
    })
}

fn pg_basebackup<P: AsRef<Path>>(
    source_port: u16,
    target_pg_config: &PgConfig,
    target_pg_data: P,
    wait_signal: &AtomicBool,
) -> anyhow::Result<()> {
    let pg_basebackup = target_pg_config
        .bin_dir()
        .expect("should have a PgConfig::bin_dir()")
        .join("pg_basebackup");
    let mut command = make_command(target_pg_config, &target_pg_data, pg_basebackup)?;
    /*
    $pg_basebackup
            --pgdata $target_tempdir_path
            --host localhost
            --port $source_port
            --username $source_username
            -Fp -Xs -P -R
         */
    command.args(["--pgdata", target_pg_data.as_ref().to_str().unwrap()]);
    // command.args(&["--username", "replicator"]);
    command.args(["--host", "localhost"]);
    command.args(["--port", &source_port.to_string()]);
    command.args(["-Fp", "-Xs", "-P", "-R"]);

    eprintln!("{command:?}");

    let mut child = command.spawn()?;
    wait_signal.store(false, Ordering::Relaxed);
    let status = child.wait()?;
    if !status.success() {
        panic!("exited with status code: {status}");
    }

    Ok(())
}

fn make_command<P: AsRef<Path>, B: AsRef<Path>>(
    pg_config: &PgConfig,
    pg_data: P,
    bin: B,
) -> anyhow::Result<Command> {
    let mut command = Command::new(bin.as_ref());
    command.env(
        "PGPORT",
        pg_config.port().map_err(|e| anyhow!(e))?.to_string(),
    );
    command.env("PGDATA", pg_data.as_ref());
    Ok(command)
}
