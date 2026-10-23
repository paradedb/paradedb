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

use postgres::{Client, Error, NoTls, Row, types::FromSqlOwned};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, Once};
use std::thread::{JoinHandle, sleep, spawn};
use std::time::{Duration, Instant};

/// Keep other schedules' write transactions out of VACUUM's snapshot horizon.
static SCHEDULE: Mutex<()> = Mutex::new(());

fn connect() -> Client {
    try_connect().expect("test server must be reachable")
}

/// `None` while the server is down, as it is for a moment after a backend crash.
fn try_connect() -> Option<Client> {
    let mut client = if let Ok(url) = std::env::var("PG_SEARCH_TEST_DATABASE_URL") {
        Client::connect(&url, NoTls).ok()?
    } else {
        static INITIALIZE: Once = Once::new();
        INITIALIZE.call_once(|| {
            pgrx_tests::run_test(
                "bootstrap_test_server",
                None,
                vec!["shared_preload_libraries='pg_search'"],
            )
            .unwrap();
        });
        pgrx_tests::client().ok()?.0
    };
    client.batch_execute("SET statement_timeout='15s'").ok()?;
    Some(client)
}

/// Waits through crash recovery for a working connection.
fn reconnect() -> Client {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        if let Some(client) = try_connect() {
            return client;
        }
        assert!(
            Instant::now() < deadline,
            "server did not come back after a crash"
        );
        sleep(Duration::from_millis(200));
    }
}

fn wait_until(description: &str, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !condition() {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {description}"
        );
        sleep(Duration::from_millis(10));
    }
}

struct Actor {
    pid: i32,
    ready: i64,
    gate: i64,
    handle: JoinHandle<(Client, Result<Vec<Row>, Error>)>,
}

struct BackgroundOutputPause {
    client: Client,
    database: String,
}

impl BackgroundOutputPause {
    fn new(ready: i64, gate: i64) -> Self {
        let mut client = connect();
        let database: String = client
            .query_one("SELECT current_database()", &[])
            .unwrap()
            .get(0);
        let database = format!("\"{}\"", database.replace('"', "\"\""));
        let mut pause = Self { client, database };
        pause
            .client
            .batch_execute(&format!(
                "ALTER DATABASE {} SET pg_search.test_output_ready='{ready}';
                 ALTER DATABASE {} SET pg_search.test_output_gate='{gate}'",
                pause.database, pause.database
            ))
            .unwrap();
        pause
    }
}

impl Drop for BackgroundOutputPause {
    fn drop(&mut self) {
        // Always attempted: a database-level setting left behind by a failed test would pause
        // every later writer in the database.
        let result = self.client.batch_execute(&format!(
            "ALTER DATABASE {} RESET pg_search.test_output_ready;
             ALTER DATABASE {} RESET pg_search.test_output_gate",
            self.database, self.database
        ));
        if !std::thread::panicking() {
            result.unwrap();
        }
    }
}

impl Actor {
    fn finish(self) -> (Client, Result<Vec<Row>, Error>) {
        self.handle.join().unwrap()
    }
}

struct Fixture {
    controller: Client,
    mutable: bool,
    table: &'static str,
    index: String,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            self.controller
                .batch_execute(&format!(
                    "SELECT pg_advisory_unlock_all(); DROP TABLE {}",
                    self.table
                ))
                .unwrap();
        }
    }
}

impl Fixture {
    fn new(table: &'static str, mutable: bool) -> Self {
        let mut controller = connect();
        controller
            .batch_execute("CREATE EXTENSION IF NOT EXISTS pg_search CASCADE")
            .unwrap();
        let index = format!("{table}_idx");
        let options = if mutable {
            "mutable_segment_rows=200, target_segment_count=1, background_layer_sizes='0'"
        } else {
            "mutable_segment_rows=0, target_segment_count=1, layer_sizes='0', background_layer_sizes='0'"
        };
        controller
            .batch_execute(&format!(
                "DROP TABLE IF EXISTS {table}; CREATE TABLE {table}(id bigint, body text);
             CREATE INDEX {index} ON {table} USING paradedb(id,body) WITH ({options})"
            ))
            .unwrap();
        Self {
            controller,
            mutable,
            table,
            index,
        }
    }

    fn segments(&mut self) {
        let rows = if self.mutable { 100 } else { 250 };
        self.segments_with_rows(rows);
    }

    fn segments_with_rows(&mut self, rows: i32) {
        let body = if self.mutable {
            "'alpha beta'"
        } else {
            "'alpha beta ' || md5(id::text)"
        };
        for batch in 0..4 {
            self.controller
                .batch_execute(&format!(
                    "INSERT INTO {} SELECT id, {body} FROM generate_series({}, {}) id",
                    self.table,
                    batch * rows + 1,
                    (batch + 1) * rows
                ))
                .unwrap();
        }
        self.controller
            .batch_execute(&format!("DELETE FROM {} WHERE id % 2=0", self.table))
            .unwrap();
    }

    fn value<T: FromSqlOwned>(&mut self, query: &str) -> T {
        self.controller.query_one(query, &[]).unwrap().get(0)
    }

    fn actor(&mut self, phase: &str, query: String) -> Actor {
        self.actor_with_timeout(phase, query, "15s")
    }

    fn actor_with_timeout(&mut self, phase: &str, query: String, timeout: &str) -> Actor {
        let (ready, gate) = self.pause_keys();
        let mut client = connect();
        client
            .batch_execute(&format!(
                "SET pg_search.test_{phase}_ready='{ready}';
             SET pg_search.test_{phase}_gate='{gate}'; SET statement_timeout='{timeout}'"
            ))
            .unwrap();
        let pid = client
            .query_one("SELECT pg_backend_pid()", &[])
            .unwrap()
            .get(0);
        let handle = spawn(move || {
            let result = client.query(&query, &[]);
            (client, result)
        });
        Actor {
            pid,
            ready,
            gate,
            handle,
        }
    }

    fn pause_keys(&mut self) -> (i64, i64) {
        // Small keys cannot collide with pg_search's own advisory keys, which set the upper
        // 32 bits (see VACUUM_SIGNAL_KEY_BASE and MERGE_SLOT_KEY_BASE).
        static NEXT_KEY: AtomicI64 = AtomicI64::new(884000);
        let ready = NEXT_KEY.fetch_add(2, Ordering::Relaxed);
        let gate = ready + 1;
        self.controller
            .query_one("SELECT pg_advisory_lock($1)", &[&gate])
            .unwrap();
        (ready, gate)
    }

    fn stage(&mut self, actor: &Actor) {
        self.stage_key(actor.pid, actor.ready);
    }

    fn stage_key(&mut self, pid: i32, ready: i64) {
        wait_until(&format!("backend {pid} reaching ready key {ready}"), || {
            self.controller
                .query_one(
                    "SELECT EXISTS(SELECT FROM pg_locks WHERE pid=$1 AND locktype='advisory'
             AND objid::bigint=$2 AND granted)",
                    &[&pid, &ready],
                )
                .unwrap()
                .get(0)
        });
    }

    fn waiting(&mut self, actor: &Actor, event: &str) {
        wait_until(&format!("backend {} waiting on {event}", actor.pid), || {
            self.controller
                .query_opt(
                    "SELECT coalesce(wait_event,'') FROM pg_stat_activity WHERE pid=$1",
                    &[&actor.pid],
                )
                .unwrap()
                .is_some_and(|row| row.get::<_, String>(0) == event)
        });
    }

    fn release(&mut self, actor: &Actor) {
        self.controller
            .query_one("SELECT pg_advisory_unlock($1)", &[&actor.gate])
            .unwrap();
    }

    fn fsm_counts(&mut self) -> (i64, i64) {
        let row = self
            .controller
            .query_one(
                &format!(
                    "SELECT count(*), count(DISTINCT free_blockno) FROM paradedb.fsm_info('{}')",
                    self.index
                ),
                &[],
            )
            .unwrap();
        (row.get(0), row.get(1))
    }

    fn unaccounted_blocks(&mut self) -> i64 {
        self.value(&format!(
            "SELECT paradedb.unaccounted_blocks('{}')",
            self.index
        ))
    }

    fn pid_gone(&mut self, pid: i32) {
        wait_until(&format!("backend {pid} exit"), || {
            self.value::<i64>(&format!(
                "SELECT count(*) FROM pg_stat_activity WHERE pid={pid}"
            )) == 0
        });
    }

    fn cancel(&mut self, actor: &Actor, terminate: bool) {
        self.cancel_pid(actor.pid, terminate);
    }

    fn cancel_pid(&mut self, pid: i32, terminate: bool) {
        let function = if terminate {
            "pg_terminate_backend"
        } else {
            "pg_cancel_backend"
        };
        assert!(
            self.controller
                .query_one(&format!("SELECT {function}($1)"), &[&pid])
                .unwrap()
                .get::<_, bool>(0)
        );
    }

    fn assert_rows(&mut self, expected: i64) {
        assert_eq!(
            self.value::<i64>(&format!("SELECT count(*) FROM {}", self.table)),
            expected
        );
        assert_eq!(
            self.value::<i64>(&format!(
                "SELECT count(*) FROM {} WHERE body @@@ 'alpha'",
                self.table
            )),
            expected
        );
    }
}

fn assert_error(result: Result<Vec<Row>, Error>, code: &str) {
    let error = result.unwrap_err();
    assert_eq!(error.as_db_error().unwrap().code().code(), code, "{error}");
}

fn assert_vacuum_error(actor: Actor) {
    let (mut client, result) = actor.finish();
    assert_error(result, "57014");
    assert_eq!(
        client
            .query_one(
                "SELECT count(*) FROM pg_locks WHERE pid=pg_backend_pid() AND locktype='advisory'",
                &[]
            )
            .unwrap()
            .get::<_, i64>(0),
        0
    );
}

#[test]
fn merger_error_and_termination_release_vacuum_barriers() {
    let _schedule = SCHEDULE.lock().unwrap_or_else(|error| error.into_inner());
    for terminate in [false, true] {
        let table = if terminate {
            "merge_cleanup_terminated_holder"
        } else {
            "merge_cleanup_failed_holder"
        };
        let mut fixture = Fixture::new(table, false);
        fixture.segments();
        let holder = fixture.actor(
            "merge",
            format!(
                "SELECT paradedb.test_hold_merger('{}',{})",
                fixture.index, !terminate
            ),
        );
        fixture.stage(&holder);
        let vacuum = fixture.actor("vacuum", format!("VACUUM {table}"));
        fixture.waiting(&vacuum, "BufferPin");
        if terminate {
            fixture.cancel(&holder, true);
        } else {
            fixture.release(&holder);
        }
        assert_error(holder.finish().1, if terminate { "57P01" } else { "XX000" });
        fixture.stage(&vacuum);
        fixture.release(&vacuum);
        vacuum.finish().1.unwrap();
        fixture.assert_rows(500);
    }
}

#[test]
fn merger_admission_and_vacuum_waits_remain_interruptible() {
    let _schedule = SCHEDULE.lock().unwrap_or_else(|error| error.into_inner());
    let mut fixture = Fixture::new("merge_cleanup_admission", false);
    fixture.segments();
    let holder = fixture.actor(
        "merge",
        format!(
            "SELECT paradedb.test_hold_merger('{}',false)",
            fixture.index
        ),
    );
    fixture.stage(&holder);
    let vacuum = fixture.actor("vacuum", format!("VACUUM {}", fixture.table));
    fixture.waiting(&vacuum, "BufferPin");
    let newcomer = fixture.actor(
        "merge",
        format!(
            "SELECT paradedb.test_hold_merger('{}',false)",
            fixture.index
        ),
    );
    fixture.stage(&newcomer);
    fixture.cancel(&newcomer, false);
    assert_error(newcomer.finish().1, "57014");
    fixture.cancel(&vacuum, false);
    assert_vacuum_error(vacuum);
    let timed = fixture.actor_with_timeout("vacuum", format!("VACUUM {}", fixture.table), "500ms");
    fixture.waiting(&timed, "BufferPin");
    assert_vacuum_error(timed);
    fixture.release(&holder);
    assert_eq!(holder.finish().1.unwrap()[0].get::<_, i32>(0), 0);
    let retry = fixture.actor("vacuum", format!("VACUUM {}", fixture.table));
    fixture.stage(&retry);
    fixture.release(&retry);
    retry.finish().1.unwrap();
    fixture.assert_rows(500);
}

#[test]
fn vacuum_exclusion_handoff_and_reader_barrier_are_preserved() {
    let _schedule = SCHEDULE.lock().unwrap_or_else(|error| error.into_inner());
    let mut fixture = Fixture::new("merge_cleanup_exclusions", false);
    fixture.segments();
    let vacuum = fixture.actor("vacuum", format!("VACUUM {}", fixture.table));
    fixture.stage(&vacuum);
    let merger = fixture.actor(
        "selection",
        format!("SELECT paradedb.test_merger_snapshot('{}')", fixture.index),
    );
    fixture.stage(&merger);
    fixture.release(&vacuum);
    wait_until("deletion metadata publication", || {
        fixture.value::<i64>(&format!(
            "SELECT paradedb.test_live_docs('{}')",
            fixture.index
        )) == 500
    });
    fixture.waiting(&vacuum, "BufferPin");
    assert!(fixture.value::<bool>(&format!(
        "SELECT paradedb.test_vacuum_active('{}')",
        fixture.index
    )));
    fixture.release(&merger);
    assert_eq!(merger.finish().1.unwrap()[0].get::<_, i64>(0), 0);
    vacuum.finish().1.unwrap();
    fixture.assert_rows(500);

    // Positive control: the same selection merges when no VACUUM list is active.
    let mut fixture = Fixture::new("merge_cleanup_no_exclusion", false);
    fixture.segments();
    assert!(
        fixture.value::<i64>(&format!(
            "SELECT paradedb.test_merger_snapshot('{}')",
            fixture.index
        )) > 0
    );
    fixture.assert_rows(500);

    let mut fixture = Fixture::new("merge_cleanup_reader", false);
    fixture.segments();
    let mut reader = connect();
    reader
        .batch_execute(&format!(
            "BEGIN; DECLARE reader CURSOR FOR SELECT id FROM {} WHERE id @@@ paradedb.all()",
            fixture.table
        ))
        .unwrap();
    assert_eq!(reader.query("FETCH 1 FROM reader", &[]).unwrap().len(), 1);
    let vacuum = fixture.actor("vacuum", format!("VACUUM {}", fixture.table));
    fixture.stage(&vacuum);
    fixture.release(&vacuum);
    fixture.waiting(&vacuum, "BufferPin");
    assert_eq!(
        fixture.value::<i64>(&format!(
            "SELECT paradedb.test_live_docs('{}')",
            fixture.index
        )),
        500
    );
    reader.batch_execute("COMMIT").unwrap();
    vacuum.finish().1.unwrap();

    let mut fixture = Fixture::new("merge_cleanup_setup", false);
    fixture.segments();
    let vacuum = fixture.actor("vacuum_setup", format!("VACUUM {}", fixture.table));
    fixture.stage(&vacuum);
    let merger = fixture.actor(
        "merge",
        format!(
            "SELECT paradedb.test_hold_merger('{}',false)",
            fixture.index
        ),
    );
    fixture.waiting(&merger, "BufferContent");
    fixture.release(&vacuum);
    fixture.stage(&merger);
    fixture.waiting(&vacuum, "BufferPin");
    assert!(fixture.value::<bool>(&format!(
        "SELECT paradedb.test_vacuum_active('{}')",
        fixture.index
    )));
    fixture.release(&merger);
    merger.finish().1.unwrap();
    vacuum.finish().1.unwrap();
}

#[test]
fn foreground_merging_does_not_queue_behind_waiting_vacuum() {
    let _schedule = SCHEDULE.lock().unwrap_or_else(|error| error.into_inner());
    for cancel in [false, true] {
        let table = if cancel {
            "merge_cleanup_foreground_cancel"
        } else {
            "merge_cleanup_foreground"
        };
        let mut fixture = Fixture::new(table, true);
        fixture.segments();
        let inserter = fixture.actor(
            "merge",
            format!("INSERT INTO {table} SELECT id, 'alpha beta' FROM generate_series(401,500) id"),
        );
        fixture.stage(&inserter);
        let vacuum = fixture.actor("vacuum", format!("VACUUM {table}"));
        fixture.waiting(&vacuum, "BufferPin");
        if cancel {
            fixture.cancel(&inserter, false);
            assert_error(inserter.finish().1, "57014");
        } else {
            connect()
                .batch_execute(&format!(
                    "INSERT INTO {table} SELECT id, 'alpha beta' FROM generate_series(501,600) id"
                ))
                .unwrap();
            let newcomer = fixture.actor(
                "merge",
                format!(
                    "INSERT INTO {table} SELECT id, 'alpha beta' FROM generate_series(601,700) id"
                ),
            );
            fixture.stage(&newcomer);
            fixture.release(&newcomer);
            newcomer.finish().1.unwrap();
            fixture.release(&inserter);
            inserter.finish().1.unwrap();
        }
        fixture.stage(&vacuum);
        fixture.release(&vacuum);
        vacuum.finish().1.unwrap();
        fixture.assert_rows(if cancel { 200 } else { 500 });
    }
}

#[test]
fn background_merge_cancellation_and_termination_reclaim_output() {
    let _schedule = SCHEDULE.lock().unwrap_or_else(|error| error.into_inner());
    for terminate in [false, true] {
        let table = if terminate {
            "merge_cleanup_background_terminate"
        } else {
            "merge_cleanup_background_cancel"
        };
        let mut fixture = Fixture::new(table, false);
        fixture.segments_with_rows(5000);
        let (ready, gate) = fixture.pause_keys();
        let settings = BackgroundOutputPause::new(ready, gate);
        fixture
            .controller
            .batch_execute(&format!(
                "ALTER INDEX {} SET(background_layer_sizes='1gb');
                 INSERT INTO {table} VALUES(20001,'alpha beta')",
                fixture.index
            ))
            .unwrap();
        let worker_query = format!(
            "SELECT pid FROM pg_locks JOIN pg_stat_activity USING(pid)
             WHERE locktype='advisory' AND objid::bigint={ready} AND granted
             AND backend_type LIKE 'background merger for %'"
        );
        wait_until("background merger writing output", || {
            !fixture
                .controller
                .query(&worker_query, &[])
                .unwrap()
                .is_empty()
        });
        let pid: i32 = fixture.value(&worker_query);
        let free_before: i64 = fixture.value(&format!(
            "SELECT count(*) FROM paradedb.fsm_info('{}')",
            fixture.index
        ));
        fixture.cancel_pid(pid, terminate);
        fixture.pid_gone(pid);
        drop(settings);
        if terminate {
            // FATAL does not unwind and the registry dies with the process: the output is left
            // to the next VACUUM.
            assert!(fixture.unaccounted_blocks() > 0);
            assert_eq!(fixture.fsm_counts().0, free_before);
            fixture
                .controller
                .batch_execute(&format!("VACUUM {table}"))
                .unwrap();
        }
        let (free, unique) = fixture.fsm_counts();
        assert!(free > free_before);
        assert_eq!(unique, free);
        assert_eq!(fixture.unaccounted_blocks(), 0);
        fixture.assert_rows(10001);
    }
}

#[test]
fn cancellation_and_termination_reclaim_reserved_output_pages() {
    let _schedule = SCHEDULE.lock().unwrap_or_else(|error| error.into_inner());
    for terminate in [false, true] {
        let table = if terminate {
            "merge_cleanup_output_terminate"
        } else {
            "merge_cleanup_output_cancel"
        };
        let mut fixture = Fixture::new(table, false);
        let blocks_before: i64 = fixture.value(&format!(
            "SELECT pg_relation_size('{}') / current_setting('block_size')::bigint",
            fixture.index
        ));
        let free_before: i64 = fixture.value(&format!(
            "SELECT count(*) FROM paradedb.fsm_info('{}')",
            fixture.index
        ));
        let writer = fixture.actor(
            "output",
            format!(
                "SELECT paradedb.test_write_unpublished('{}', {})",
                fixture.index,
                64 * 1024 * 1024
            ),
        );
        fixture.stage(&writer);
        let allocated: i64 = fixture.value::<i64>(&format!(
            "SELECT pg_relation_size('{}') / current_setting('block_size')::bigint",
            fixture.index
        )) - blocks_before;
        assert!(allocated > 2);
        let pid = writer.pid;
        fixture.cancel(&writer, terminate);
        let (mut client, result) = writer.finish();
        assert_error(result, if terminate { "57P01" } else { "57014" });
        if terminate {
            fixture.pid_gone(pid);
            // Left to VACUUM: written pages carry an aborted stamp, reserved ones are empty or
            // still show their previous owner, and nothing pins them any more.
            assert_eq!(fixture.unaccounted_blocks(), allocated);
            fixture
                .controller
                .batch_execute(&format!("VACUUM {table}"))
                .unwrap();
        } else {
            assert_eq!(
                client
                    .query_one(
                        &format!(
                            "SELECT paradedb.test_reclaim_unpublished('{}')",
                            fixture.index
                        ),
                        &[]
                    )
                    .unwrap()
                    .get::<_, i64>(0),
                1
            );
            assert_eq!(
                client
                    .query_one(
                        &format!(
                            "SELECT paradedb.test_reclaim_unpublished('{}')",
                            fixture.index
                        ),
                        &[]
                    )
                    .unwrap()
                    .get::<_, i64>(0),
                0
            );
        }
        let (free, unique): (i64, i64) = {
            let row = fixture.controller.query_one(&format!("SELECT count(*), count(DISTINCT free_blockno) FROM paradedb.fsm_info('{}')", fixture.index), &[]).unwrap();
            (row.get(0), row.get(1))
        };
        assert_eq!(free - free_before, allocated);
        assert_eq!(unique, free);
    }
}

/// Blocks a writer has reserved but not yet written still carry their previous stamps, so a
/// sweep that judged them by stamp alone would free them under the writer. It must not: they
/// are pinned, and the sweep claims only under a cleanup lock.
#[test]
fn sweep_leaves_a_writers_reserved_blocks_alone() {
    let _schedule = SCHEDULE.lock().unwrap_or_else(|error| error.into_inner());
    let table = "merge_cleanup_sweep_reserved";
    let mut fixture = Fixture::new(table, false);
    fixture.segments();
    // Merge and garbage-collect, so the FSM holds blocks whose stamps are old and committed.
    fixture
        .controller
        .batch_execute(&format!("VACUUM {table}"))
        .unwrap();
    let (free_before, _) = fixture.fsm_counts();
    assert!(free_before > 0, "fixture must leave free blocks to reuse");

    let writer = fixture.actor(
        "output",
        format!(
            "SELECT paradedb.test_write_unpublished('{}', {})",
            fixture.index,
            32 * 1024 * 1024
        ),
    );
    fixture.stage(&writer);
    fixture
        .controller
        .batch_execute(&format!("VACUUM {table}"))
        .unwrap();
    let (free, unique) = fixture.fsm_counts();
    assert_eq!(free, unique);

    fixture.cancel(&writer, false);
    let (mut client, result) = writer.finish();
    assert_error(result, "57014");
    client
        .query_one(
            &format!(
                "SELECT paradedb.test_reclaim_unpublished('{}')",
                fixture.index
            ),
            &[],
        )
        .unwrap();
    let (free, unique) = fixture.fsm_counts();
    assert_eq!(free, unique, "a block was returned twice");
    assert_eq!(fixture.unaccounted_blocks(), 0);
    fixture.assert_rows(500);
}

/// A crash leaves no registry and no pins. The next VACUUM returns what the writer had
/// allocated, reserved and never written included, and the index keeps answering.
#[test]
fn vacuum_returns_blocks_a_crash_left_behind() {
    let _schedule = SCHEDULE.lock().unwrap_or_else(|error| error.into_inner());
    let table = "merge_cleanup_crash";
    let mut fixture = Fixture::new(table, false);
    fixture.segments();
    let blocks_before: i64 = fixture.value(&format!(
        "SELECT pg_relation_size('{}') / current_setting('block_size')::bigint",
        fixture.index
    ));
    let writer = fixture.actor(
        "output",
        format!(
            "SELECT paradedb.test_write_unpublished('{}', {})",
            fixture.index,
            32 * 1024 * 1024
        ),
    );
    fixture.stage(&writer);
    let allocated: i64 = fixture.value::<i64>(&format!(
        "SELECT pg_relation_size('{}') / current_setting('block_size')::bigint",
        fixture.index
    )) - blocks_before;
    assert!(allocated > 2);

    let killed = std::process::Command::new("kill")
        .args(["-9", &writer.pid.to_string()])
        .status()
        .unwrap();
    assert!(killed.success());
    // The postmaster restarts every backend, this test's connections included.
    writer.finish().1.unwrap_err();
    fixture.controller = reconnect();
    fixture
        .controller
        .batch_execute("SET statement_timeout='60s'")
        .unwrap();

    // Every block the writer took is unaccounted: the extension counted above, plus whatever
    // it drained from the FSM.
    assert!(fixture.unaccounted_blocks() >= allocated);
    fixture
        .controller
        .batch_execute(&format!("VACUUM {table}"))
        .unwrap();
    assert_eq!(fixture.unaccounted_blocks(), 0);
    let (free, unique) = fixture.fsm_counts();
    assert_eq!(free, unique);
    fixture.assert_rows(500);
}
