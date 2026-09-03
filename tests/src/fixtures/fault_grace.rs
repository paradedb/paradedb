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

//! Fault tolerance for qgen under Antithesis.
//!
//! Retries are unbounded by default: any finite window is one the fault injector can outlast, so
//! a transient fault must never fail the run on its own. Liveness is asserted from the outside —
//! the `anytime_recovery_liveness` command heals every fault, then writes a poke file declaring
//! when faults resume and how quickly qgen must make progress. Only while that declaration is in
//! force can failing to make progress fail the run, which no fault schedule can fake.
//!
//! The unpause deadline also makes a server-side `statement_timeout` (57014) attributable:
//! Antithesis pauses threads, so any fixed timeout can be exceeded while faults are active. Only
//! when an attempt ran start to finish inside one declared pause is "the query could not finish
//! in time on a healthy database" a real finding.

use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use lockfree_object_pool::MutexObjectPool;
use sqlx::PgConnection;

/// What kind of fault-induced failure occurred.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransientKind {
    /// The connection died. Retrying works once the fault heals, on a fresh connection.
    ConnectionLost,
    /// The server cancelled the statement: the database answered, on a still-usable connection.
    QueryCanceled,
}

/// Classify an error as fault-induced. Connection-level errors are ridden out by reconnecting; a
/// server-side `statement_timeout` (57014) is meaningful only relative to the fault schedule.
/// Serialization failures and deadlocks are excluded: with one instance and one session per test
/// they would be real findings. Always `None` under a plain `cargo test`, which keeps every error
/// a hard failure.
pub fn classify_transient(e: &sqlx::Error) -> Option<TransientKind> {
    if !cfg!(feature = "dst") {
        return None;
    }
    match e {
        sqlx::Error::Io(_)
        | sqlx::Error::Protocol(_)
        | sqlx::Error::WorkerCrashed
        | sqlx::Error::PoolClosed
        | sqlx::Error::PoolTimedOut => Some(TransientKind::ConnectionLost),
        sqlx::Error::Database(db) => {
            let code = db.code().unwrap_or_default();
            if dst::is_connection_lost_sqlstate(&code) {
                Some(TransientKind::ConnectionLost)
            } else if code == "57014" {
                Some(TransientKind::QueryCanceled)
            } else {
                None
            }
        }
        _ => None,
    }
}

const INITIAL_BACKOFF: Duration = Duration::from_millis(250);
const MAX_BACKOFF: Duration = Duration::from_secs(5);

/// A declared fault pause, parsed from the poke file: `<unpause_epoch_ms> <window_ms>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Pause {
    /// When faults resume (epoch ms). `u64::MAX` for the env-var baseline, which never expires.
    unpause_at_ms: u64,
    /// How quickly qgen must make progress while the pause is in force.
    window: Duration,
}

/// Where pause declarations come from: `PARADEDB_QGEN_RECONNECT_GRACE_FILE` (the poke channel)
/// and `PARADEDB_QGEN_RECONNECT_GRACE_MS` (local-testing baseline, ignored if the file is set).
#[derive(Clone, Debug, Default)]
pub(crate) struct PauseSource {
    file: Option<PathBuf>,
    baseline: Option<Duration>,
}

impl PauseSource {
    fn from_env() -> Self {
        Self {
            file: std::env::var("PARADEDB_QGEN_RECONNECT_GRACE_FILE")
                .ok()
                .filter(|p| !p.is_empty())
                .map(PathBuf::from),
            baseline: std::env::var("PARADEDB_QGEN_RECONNECT_GRACE_MS")
                .ok()
                .and_then(|ms| ms.trim().parse::<u64>().ok())
                .map(Duration::from_millis),
        }
    }

    /// The pause in force right now, if any. Re-read on every use. A missing, malformed, or
    /// expired file is no pause: a supervisor that has not run, a half-written file, or a file
    /// stranded by a killed supervisor must not be able to fail the run.
    fn current(&self) -> Option<Pause> {
        if let Some(file) = self.file.as_ref() {
            let contents = std::fs::read_to_string(file).ok()?;
            let pause = Self::parse(&contents).or_else(|| {
                eprintln!(
                    "qgen: ignoring malformed grace file {}: {:?}",
                    file.display(),
                    contents.trim()
                );
                None
            })?;
            return (now_ms() < pause.unpause_at_ms).then_some(pause);
        }
        self.baseline.map(|window| Pause {
            unpause_at_ms: u64::MAX,
            window,
        })
    }

    fn parse(contents: &str) -> Option<Pause> {
        let mut fields = contents.split_whitespace();
        let unpause_at_ms = fields.next()?.parse::<u64>().ok()?;
        let window = Duration::from_millis(fields.next()?.parse::<u64>().ok()?);
        fields.next().is_none().then_some(Pause {
            unpause_at_ms,
            window,
        })
    }
}

pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn pause_source() -> &'static PauseSource {
    static SOURCE: OnceLock<PauseSource> = OnceLock::new();
    SOURCE.get_or_init(PauseSource::from_env)
}

/// The pause state when an attempt began. A statement timeout is a finding only if the whole
/// attempt ran inside one declared pause; two pokes never share a deadline, so "same deadline at
/// start and at failure" pins the attempt inside a single episode. Anything else — faults active,
/// faults resumed mid-attempt, the attempt spanning two pauses — may have been slowed by faults
/// and must be retried instead.
struct PauseWitness(Option<Pause>);

impl PauseWitness {
    fn take(source: &PauseSource) -> Self {
        Self(source.current())
    }

    fn still_paused(&self, source: &PauseSource) -> bool {
        match (self.0, source.current()) {
            (Some(at_start), Some(now)) => at_start.unpause_at_ms == now.unpause_at_ms,
            _ => false,
        }
    }
}

/// Why [`retry_transient`] gave up.
pub enum RetryError {
    /// The statement timed out while faults were paused for the whole attempt: the query cannot
    /// finish inside `statement_timeout` on a healthy database.
    TimedOutUnderPause(sqlx::Error),
    /// A pause's recovery window elapsed without progress: a liveness failure.
    GraceExpired(String),
}

/// One attempt of a retryable operation: either it resolved (the resolution may itself be a hard
/// failure — that is the caller's business), or it hit a classified transient fault.
pub enum Attempt<T> {
    Done(T),
    Transient(TransientKind, sqlx::Error),
}

/// Adapts a raw SQL result into an [`Attempt`]: transient errors retry, everything else — success
/// or hard error — resolves to the caller.
pub fn sql_attempt<T>(result: Result<T, sqlx::Error>) -> Attempt<Result<T, sqlx::Error>> {
    match result {
        Ok(value) => Attempt::Done(Ok(value)),
        Err(e) => match classify_transient(&e) {
            Some(kind) => Attempt::Transient(kind, e),
            None => Attempt::Done(Err(e)),
        },
    }
}

/// Runs `op` against pooled connections, retrying transient faults until it resolves.
///
/// A lost connection is never returned to the pool (the next `pull` builds a live one); a
/// cancelled statement keeps its connection, which is still usable. Retries are bounded only
/// while a pause is declared — see the module doc.
pub fn retry_transient<T>(
    pool: &MutexObjectPool<PgConnection>,
    what: &str,
    mut op: impl FnMut(&mut PgConnection) -> Attempt<T>,
) -> Result<T, RetryError> {
    let mut retry = FaultRetry::default();
    loop {
        let witness = PauseWitness::take(pause_source());
        let mut conn = pool.pull();
        let attempt = op(&mut conn);
        observe_schedule();
        let (kind, err) = match attempt {
            Attempt::Done(value) => return Ok(value),
            Attempt::Transient(kind, err) => (kind, err),
        };
        match kind {
            TransientKind::ConnectionLost => {
                // The peer is gone; leak the connection rather than let `Drop` return it to the
                // pool, where a later `pull` would hand it back out.
                std::mem::forget(conn);
            }
            TransientKind::QueryCanceled => {
                if witness.still_paused(pause_source()) {
                    return Err(RetryError::TimedOutUnderPause(err));
                }
            }
        }
        if let Err(reason) = retry.record(what, &err) {
            return Err(RetryError::GraceExpired(reason));
        }
    }
}

/// Counters describing the fault schedule this timeline actually experienced, so its shape can be
/// asserted rather than trusted.
static LAST_PAUSED: AtomicBool = AtomicBool::new(false);
static QUIET_PERIODS_COMPLETED: AtomicUsize = AtomicUsize::new(0);
static FAULTS_BEFORE_ANY_QUIET: AtomicUsize = AtomicUsize::new(0);

/// Samples the pause state after every attempt — not just failing ones, since a fault-free window
/// is exactly when nothing fails — and asserts the schedule alternates: faults, a quiet window,
/// faults again.
fn observe_schedule() {
    let paused = pause_source().current().is_some();
    let was_paused = LAST_PAUSED.swap(paused, Ordering::Relaxed);

    if paused && !was_paused {
        dst::assert_reachable!("qgen: the recovery-liveness command paused faults");
    }
    if !paused && was_paused {
        QUIET_PERIODS_COMPLETED.fetch_add(1, Ordering::Relaxed);
        dst::assert_reachable!("qgen: returned to unbounded retry after a fault-free window");
    }

    // Faults must not be paused everywhere: some case has to complete under uninterrupted chaos.
    dst::assert_sometimes!(
        QUIET_PERIODS_COMPLETED.load(Ordering::Relaxed) == 0
            && FAULTS_BEFORE_ANY_QUIET.load(Ordering::Relaxed) > 0,
        "qgen: a case completed while faults had never been paused",
        &serde_json::json!({})
    );
}

fn note_fault() {
    if QUIET_PERIODS_COMPLETED.load(Ordering::Relaxed) == 0 {
        FAULTS_BEFORE_ANY_QUIET.fetch_add(1, Ordering::Relaxed);
    } else if FAULTS_BEFORE_ANY_QUIET.load(Ordering::Relaxed) > 0 {
        dst::assert_reachable!("qgen: faults resumed after surviving a fault-free window");
    }
}

/// Tracks one continuous transient fault against the declared pause. The pause state is re-read
/// on every failure and the clock restarts whenever it changes, so a poke landing mid-fault
/// starts a fresh recovery window: it asks "can you recover within the window from *now*".
#[derive(Default)]
pub(crate) struct FaultRetry {
    down_since: Option<std::time::Instant>,
    last_pause: Option<Option<Pause>>,
    backoff: Option<Duration>,
}

impl FaultRetry {
    /// Records a transient failure of `what`, then sleeps so the caller can retry. Returns `Err`
    /// only once a declared pause's window has elapsed continuously — i.e. only while faults are
    /// healed.
    pub(crate) fn record(&mut self, what: &str, err: &dyn std::fmt::Display) -> Result<(), String> {
        let pause = pause_source().current();
        note_fault();

        // A poke landing mid-fault is the only path where the liveness check can actually fail;
        // assert it happens at all.
        dst::assert_sometimes!(
            self.last_pause
                .is_some_and(|last| pause_narrowed(last, pause)),
            "qgen: a recovery poke narrowed the grace window during an active database fault",
            &serde_json::json!({})
        );

        if self.last_pause != Some(pause) {
            self.down_since = None;
            self.backoff = None;
        }
        self.last_pause = Some(pause);

        let down_since = *self.down_since.get_or_insert_with(std::time::Instant::now);
        if let Some(pause) = pause
            && down_since.elapsed() >= pause.window
        {
            return Err(format!(
                "{what}: no progress for {:?}, past the {:?} grace window \
                 (faults were healed, so this is a liveness failure): {err}",
                down_since.elapsed(),
                pause.window
            ));
        }

        dst::assert_reachable!("qgen: retried a transient database fault");
        eprintln!("{what}: transient database fault, retrying: {err}");
        let backoff = self.backoff.unwrap_or(INITIAL_BACKOFF);
        std::thread::sleep(backoff);
        self.backoff = Some((backoff * 2).min(MAX_BACKOFF));
        Ok(())
    }
}

/// Whether `now` is a strictly tighter constraint than `last`. No pause is the loosest there is.
fn pause_narrowed(last: Option<Pause>, now: Option<Pause>) -> bool {
    match (last, now) {
        (None, Some(_)) => true,
        (Some(last), Some(now)) => now.window < last.window,
        (_, None) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file_source(path: PathBuf) -> PauseSource {
        PauseSource {
            file: Some(path),
            baseline: None,
        }
    }

    #[test]
    fn absent_file_means_no_pause() {
        let s = file_source(PathBuf::from("/nonexistent/qgen-grace"));
        assert_eq!(s.current(), None);
    }

    #[test]
    fn no_file_and_no_baseline_is_unbounded() {
        assert_eq!(PauseSource::default().current(), None);
    }

    #[test]
    fn baseline_is_a_pause_that_never_expires() {
        let s = PauseSource {
            file: None,
            baseline: Some(Duration::from_secs(7)),
        };
        let p = s.current().unwrap();
        assert_eq!(p.window, Duration::from_secs(7));
        assert_eq!(p.unpause_at_ms, u64::MAX);
    }

    #[test]
    fn file_declares_deadline_and_window() {
        let path = std::env::temp_dir().join("qgen-grace-test-valid");
        let future = now_ms() + 60_000;
        std::fs::write(&path, format!("{future} 1500\n")).unwrap();
        let p = file_source(path.clone()).current().unwrap();
        assert_eq!(p.unpause_at_ms, future);
        assert_eq!(p.window, Duration::from_millis(1500));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn expired_file_is_no_pause() {
        let path = std::env::temp_dir().join("qgen-grace-test-expired");
        std::fs::write(&path, format!("{} 1500\n", now_ms().saturating_sub(1))).unwrap();
        assert_eq!(file_source(path.clone()).current(), None);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn malformed_file_is_no_pause() {
        let path = std::env::temp_dir().join("qgen-grace-test-malformed");
        for junk in ["not-a-number", "12345", "1 2 3"] {
            std::fs::write(&path, junk).unwrap();
            assert_eq!(file_source(path.clone()).current(), None, "junk: {junk}");
        }
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn timeout_after_unpause_is_not_attributed() {
        // An attempt starts inside a pause, but its statement timeout fires after faults resume.
        let path = std::env::temp_dir().join("qgen-grace-test-mid-attempt");
        std::fs::write(&path, format!("{} 120000\n", now_ms() + 60_000)).unwrap();
        let source = file_source(path.clone());
        let witness = PauseWitness::take(&source);
        assert!(witness.0.is_some(), "attempt started under a pause");
        std::fs::write(&path, format!("{} 120000\n", now_ms().saturating_sub(1))).unwrap();
        assert!(!witness.still_paused(&source));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn timeout_spanning_two_pauses_is_not_attributed() {
        // An attempt that starts in pause A and fails in pause B was exposed to faults in between.
        let path = std::env::temp_dir().join("qgen-grace-test-two-pokes");
        let source = file_source(path.clone());
        std::fs::write(&path, format!("{} 120000\n", now_ms() + 60_000)).unwrap();
        let witness = PauseWitness::take(&source);
        std::fs::write(&path, format!("{} 120000\n", now_ms() + 90_000)).unwrap();
        assert!(!witness.still_paused(&source));
        std::fs::remove_file(&path).ok();
    }
}
