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
    let mut client = if let Ok(url) = std::env::var("PG_SEARCH_TEST_DATABASE_URL") {
        Client::connect(&url, NoTls).unwrap()
    } else {
        static INITIALIZE: Once = Once::new();
        INITIALIZE.call_once(|| {
            pgrx_tests::run_test(
                "bootstrap_test_server",
                None,
                vec![
                    "shared_preload_libraries='pg_search'",
                    "autovacuum_naptime='1s'",
                ],
            )
            .unwrap();
        });
        pgrx_tests::client().unwrap().0
    };
    client.batch_execute("SET statement_timeout='15s'").unwrap();
    client
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
    sweep_interval: i32,
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
        let interval = self.sweep_interval;
        let restored = self
            .controller
            .batch_execute(&format!(
                "ALTER SYSTEM SET paradedb.mutable_sweep_interval = {interval}"
            ))
            .and_then(|_| self.controller.simple_query("SELECT pg_reload_conf()"));
        if !std::thread::panicking() {
            restored.unwrap();
        }
    }
}

impl Fixture {
    fn new(table: &'static str, mutable: bool) -> Self {
        let mut controller = connect();
        let sweep_interval: i32 = controller
            .query_one(
                "SELECT setting::int FROM pg_settings WHERE name='paradedb.mutable_sweep_interval'",
                &[],
            )
            .unwrap()
            .get(0);
        if sweep_interval != 0 {
            controller
                .batch_execute("ALTER SYSTEM SET paradedb.mutable_sweep_interval = 0")
                .unwrap();
            controller.simple_query("SELECT pg_reload_conf()").unwrap();
            wait_until("sweep configuration reload", || {
                controller
                    .query_one(
                        "SELECT setting::int FROM pg_settings WHERE name='paradedb.mutable_sweep_interval'",
                        &[],
                    )
                    .unwrap()
                    .get::<_, i32>(0)
                    == 0
            });
        }
        wait_until("existing sweeps exiting", || {
            controller.query_one(
                "SELECT NOT EXISTS(SELECT FROM pg_stat_activity WHERE backend_type='pg_search mutable sweep')",
                &[],
            ).unwrap().get(0)
        });
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
                "DROP TABLE IF EXISTS {table}; CREATE TABLE {table}(id bigint, body text) WITH (autovacuum_enabled=false);
             CREATE INDEX {index} ON {table} USING paradedb(id,body) WITH ({options})"
            ))
            .unwrap();
        Self {
            controller,
            mutable,
            table,
            index,
            sweep_interval,
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
        let batches = if self.mutable { 6 } else { 4 };
        for batch in 0..batches {
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
        wait_until(
            &format!("backend {} reaching ready key {}", actor.pid, actor.ready),
            || {
                self.controller
                    .query_one(
                        "SELECT EXISTS(SELECT FROM pg_locks WHERE pid=$1 AND locktype='advisory'
             AND objid::bigint=$2 AND granted)",
                        &[&actor.pid, &actor.ready],
                    )
                    .unwrap()
                    .get(0)
            },
        );
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
        format!("SELECT paradedb.test_merge_index('{}')", fixture.index),
    );
    fixture.stage(&merger);
    fixture.release(&vacuum);
    wait_until("deletion metadata publication", || {
        fixture.value::<i64>(&format!(
            "SELECT paradedb.test_live_docs('{}')",
            fixture.index
        )) == 500
    });
    fixture.waiting(&vacuum, "page");
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
            "SELECT paradedb.test_merge_index('{}')",
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
    fixture.waiting(&vacuum, "page");
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
    fixture.waiting(&vacuum, "page");
    assert!(fixture.value::<bool>(&format!(
        "SELECT paradedb.test_vacuum_active('{}')",
        fixture.index
    )));
    fixture.release(&merger);
    merger.finish().1.unwrap();
    vacuum.finish().1.unwrap();
}

#[test]
fn cancelled_foreground_merge_does_not_keep_inputs_reserved() {
    let _schedule = SCHEDULE.lock().unwrap_or_else(|error| error.into_inner());
    let mut fixture = Fixture::new("merge_cleanup_cancelled_reservation", true);
    fixture
        .controller
        .batch_execute(&format!(
            "ALTER TABLE {} SET (autovacuum_enabled=false)",
            fixture.table
        ))
        .unwrap();
    fixture.segments();

    // The production merge pause runs after add_segment_ids has reserved the inputs.
    let inserter = fixture.actor(
        "merge",
        format!(
            "INSERT INTO {} SELECT id, 'alpha beta' FROM generate_series(601,700) id",
            fixture.table
        ),
    );
    fixture.stage(&inserter);
    let inserter_pid = inserter.pid;
    assert!(
        fixture.value::<i64>(&format!(
            "SELECT count(*) FROM paradedb.merge_info('{}') WHERE pid={inserter_pid}",
            fixture.index
        )) > 0
    );

    fixture.cancel(&inserter, false);
    let (mut inserter_client, result) = inserter.finish();
    assert_error(result, "57014");
    assert_eq!(
        inserter_client
            .query_one("SELECT pg_backend_pid()", &[])
            .unwrap()
            .get::<_, i32>(0),
        inserter_pid
    );

    // Keep the cancelled session connected, and run no VACUUM or reservation GC before this
    // assertion: either backend exit or GC could hide a stale MergeEntry.
    assert!(
        fixture.value::<i64>(&format!(
            "SELECT paradedb.test_merge_index('{}')",
            fixture.index
        )) > 0,
        "a cancelled merge must not keep its inputs reserved while its session remains connected"
    );
    fixture.assert_rows(300);
    drop(inserter_client);
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
            format!("INSERT INTO {table} SELECT id, 'alpha beta' FROM generate_series(601,700) id"),
        );
        fixture.stage(&inserter);
        let vacuum = fixture.actor("vacuum", format!("VACUUM {table}"));
        fixture.waiting(&vacuum, "BufferPin");
        if cancel {
            fixture.cancel(&inserter, false);
            assert_error(inserter.finish().1, "57014");
        } else {
            let mut writer = connect();
            for batch in 7..12 {
                writer
                    .batch_execute(&format!(
                        "INSERT INTO {table} SELECT id, 'alpha beta' FROM generate_series({},{}) id",
                        batch * 100 + 1,
                        (batch + 1) * 100
                    ))
                    .unwrap();
            }
            let newcomer = fixture.actor(
                "merge",
                format!(
                    "INSERT INTO {table} SELECT id, 'alpha beta' FROM generate_series(1201,1300) id"
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
        fixture.assert_rows(if cancel { 300 } else { 1000 });
    }
}

#[test]
fn late_reader_does_not_cancel_autovacuum_waiting_for_merger() {
    let _schedule = SCHEDULE.lock().unwrap_or_else(|error| error.into_inner());
    let mut fixture = Fixture::new("merge_cleanup_autovacuum", false);
    fixture.segments();
    let (ready, gate) = fixture.pause_keys();
    fixture.controller.batch_execute(&format!(
        "DO $$ BEGIN
         EXECUTE format('ALTER DATABASE %I SET pg_search.test_vacuum_ready = %L', current_database(), '{ready}');
         EXECUTE format('ALTER DATABASE %I SET pg_search.test_vacuum_gate = %L', current_database(), '{gate}');
         END $$"
    )).unwrap();
    let previous_workers: Vec<i32> = fixture
        .controller
        .query(
            "SELECT pid FROM pg_stat_activity WHERE backend_type='autovacuum worker'
         AND datname=current_database()",
            &[],
        )
        .unwrap()
        .iter()
        .map(|row| row.get(0))
        .collect();
    wait_until("workers with old database settings exiting", || {
        fixture
            .controller
            .query_one(
                "SELECT NOT EXISTS(SELECT FROM pg_stat_activity WHERE pid=ANY($1))",
                &[&previous_workers],
            )
            .unwrap()
            .get(0)
    });
    fixture
        .controller
        .batch_execute(
            "ALTER TABLE merge_cleanup_autovacuum SET
         (autovacuum_enabled=true, autovacuum_vacuum_threshold=0,
          autovacuum_vacuum_scale_factor=0, autovacuum_vacuum_cost_delay=0);
         SELECT pg_stat_force_next_flush()",
        )
        .unwrap();
    let mut vacuum_pid = 0;
    wait_until("autovacuum reaching the exclusion handoff", || {
        let row = fixture
            .controller
            .query_opt(
                "SELECT a.pid, a.query FROM pg_stat_activity a
             JOIN pg_stat_progress_vacuum v ON v.pid=a.pid
             JOIN pg_locks l ON l.pid=a.pid
             WHERE a.backend_type='autovacuum worker'
             AND v.relid='merge_cleanup_autovacuum'::regclass
             AND v.datid=(SELECT oid FROM pg_database WHERE datname=current_database())
             AND l.locktype='advisory' AND l.objid::bigint=$1 AND l.granted",
                &[&ready],
            )
            .unwrap();
        if let Some(row) = row {
            vacuum_pid = row.get(0);
            assert!(!row.get::<_, String>(1).contains("prevent wraparound"));
            true
        } else {
            false
        }
    });
    fixture
        .controller
        .batch_execute(
            "DO $$ BEGIN
         EXECUTE format('ALTER DATABASE %I RESET pg_search.test_vacuum_ready', current_database());
         EXECUTE format('ALTER DATABASE %I RESET pg_search.test_vacuum_gate', current_database());
         END $$",
        )
        .unwrap();
    for batch in 4..8 {
        fixture
            .controller
            .batch_execute(&format!(
                "INSERT INTO merge_cleanup_autovacuum SELECT id, 'alpha beta ' || md5(id::text)
             FROM generate_series({},{}) id",
                batch * 250 + 1,
                (batch + 1) * 250
            ))
            .unwrap();
    }
    let merger = fixture.actor(
        "merge",
        format!("SELECT paradedb.test_merge_index('{}')", fixture.index),
    );
    fixture.stage(&merger);
    fixture
        .controller
        .query_one("SELECT pg_advisory_unlock($1)", &[&gate])
        .unwrap();
    wait_until("autovacuum waiting for the overlapping merge", || {
        fixture
            .controller
            .query_opt(
                "SELECT wait_event FROM pg_stat_activity WHERE pid=$1",
                &[&vacuum_pid],
            )
            .unwrap()
            .is_some_and(|row| {
                matches!(
                    row.get::<_, Option<String>>(0).as_deref(),
                    Some("BufferPin" | "page")
                )
            })
    });
    let mut reader = connect();
    reader
        .batch_execute("SET deadlock_timeout='100ms'; SET log_lock_waits=on")
        .unwrap();
    let reader_pid: i32 = reader
        .query_one("SELECT pg_backend_pid()", &[])
        .unwrap()
        .get(0);
    let late_reader = spawn(move || {
        reader.query_one(
            "SELECT count(*) FROM merge_cleanup_autovacuum WHERE id @@@ paradedb.all()",
            &[],
        )
    });
    wait_until("late reader queuing for admission", || {
        fixture
            .controller
            .query_one(
                "SELECT EXISTS(SELECT FROM pg_locks WHERE pid=$1 AND locktype='page'
             AND mode='ShareLock' AND NOT granted)",
                &[&reader_pid],
            )
            .unwrap()
            .get(0)
    });
    sleep(Duration::from_millis(500));
    let survived: bool = fixture
        .controller
        .query_one(
            "SELECT EXISTS(SELECT FROM pg_stat_progress_vacuum WHERE pid=$1
         AND relid='merge_cleanup_autovacuum'::regclass)",
            &[&vacuum_pid],
        )
        .unwrap()
        .get(0);
    let reader_still_waiting = !late_reader.is_finished();
    fixture.release(&merger);
    assert!(merger.finish().1.unwrap()[0].get::<_, i64>(0) > 0);
    assert_eq!(late_reader.join().unwrap().unwrap().get::<_, i64>(0), 1500);
    assert!(
        survived,
        "late reader canceled autovacuum while the merger held its pins"
    );
    assert!(
        reader_still_waiting,
        "late reader bypassed autovacuum admission"
    );
    wait_until("successful automatic vacuum", || {
        fixture.value::<i64>(
        "SELECT autovacuum_count FROM pg_stat_user_tables WHERE relname='merge_cleanup_autovacuum'"
    ) > 0
    });
    fixture.assert_rows(1500);
    assert!(fixture.value::<bool>(&format!(
        "SELECT bool_and(passed) FROM pdb.verify_index('{}',heapallindexed=>true)",
        fixture.index
    )));
}

#[test]
fn periodic_sweep_cancellation_releases_vacuum_barriers() {
    let _schedule = SCHEDULE.lock().unwrap_or_else(|error| error.into_inner());
    for terminate in [false, true] {
        let table = if terminate {
            "mutable_sweep_terminated"
        } else {
            "mutable_sweep_cancelled"
        };
        let mut fixture = Fixture::new(table, true);
        fixture
            .controller
            .batch_execute(&format!(
                "INSERT INTO {table} SELECT id, 'alpha beta' FROM generate_series(1,50) id"
            ))
            .unwrap();
        let (ready, gate) = fixture.pause_keys();
        fixture.controller.batch_execute(&format!(
            "DO $$ BEGIN
             EXECUTE format('ALTER DATABASE %I SET pg_search.test_merge_ready = %L', current_database(), '{ready}');
             EXECUTE format('ALTER DATABASE %I SET pg_search.test_merge_gate = %L', current_database(), '{gate}');
             END $$;"
        )).unwrap();
        fixture
            .controller
            .batch_execute("ALTER SYSTEM SET paradedb.mutable_sweep_interval = 1000")
            .unwrap();
        fixture
            .controller
            .simple_query("SELECT pg_reload_conf()")
            .unwrap();
        let mut pid = 0;
        wait_until("periodic sweep reaching merge publication", || {
            let row = fixture
                .controller
                .query_opt(
                    "SELECT a.pid FROM pg_stat_activity a JOIN pg_locks l ON l.pid=a.pid
                 WHERE a.backend_type='pg_search mutable sweep' AND a.datname=current_database() AND a.query='merging'
                   AND l.locktype='advisory' AND l.objid::bigint=$1 AND l.granted",
                    &[&ready],
                )
                .unwrap();
            if let Some(row) = row {
                pid = row.get(0);
                true
            } else {
                false
            }
        });
        fixture.controller.batch_execute(
            "DO $$ BEGIN
             EXECUTE format('ALTER DATABASE %I RESET pg_search.test_merge_ready', current_database());
             EXECUTE format('ALTER DATABASE %I RESET pg_search.test_merge_gate', current_database());
             END $$"
        ).unwrap();
        fixture.cancel_pid(pid, terminate);
        wait_until("cancelled sweep releasing its backend", || {
            fixture
                .controller
                .query_one(
                    "SELECT NOT EXISTS(SELECT FROM pg_stat_activity WHERE pid=$1)",
                    &[&pid],
                )
                .unwrap()
                .get(0)
        });
        fixture
            .controller
            .query_one("SELECT pg_advisory_unlock($1)", &[&gate])
            .unwrap();
        fixture
            .controller
            .batch_execute(&format!("VACUUM {table}"))
            .unwrap();
        fixture.assert_rows(50);
        let deadline = Instant::now() + Duration::from_secs(45);
        while !fixture.value::<bool>(&format!(
            "SELECT NOT EXISTS(SELECT FROM paradedb.index_info('{}') WHERE visible AND mutable)",
            fixture.index
        )) {
            assert!(
                Instant::now() < deadline,
                "cancelled worker was not rediscovered"
            );
            sleep(Duration::from_millis(20));
        }
        assert!(fixture.value::<bool>(&format!(
            "SELECT bool_and(passed) FROM pdb.verify_index('{}', heapallindexed => true)",
            fixture.index
        )));
    }
}

#[test]
fn periodic_sweep_skips_exclusive_vacuum_admission() {
    let _schedule = SCHEDULE.lock().unwrap_or_else(|error| error.into_inner());
    let mut fixture = Fixture::new("sweep_busy_admission", true);
    let holder = fixture.actor(
        "merge",
        format!(
            "SELECT paradedb.test_hold_cleanup_admission('{}')",
            fixture.index
        ),
    );
    fixture.stage(&holder);
    fixture.controller.batch_execute(
        "DROP TABLE IF EXISTS sweep_free_admission;
         CREATE TABLE sweep_free_admission (id bigint,body text);
         CREATE INDEX sweep_free_admission_idx ON sweep_free_admission USING paradedb(id,body) WITH(mutable_segment_rows=10000);
         INSERT INTO sweep_free_admission VALUES(1,'alpha beta')"
    ).unwrap();
    fixture
        .controller
        .batch_execute("ALTER SYSTEM SET paradedb.mutable_sweep_interval = 1000")
        .unwrap();
    fixture
        .controller
        .simple_query("SELECT pg_reload_conf()")
        .unwrap();
    wait_until("sweep skipping busy VACUUM admission", || {
        assert!(!fixture.value::<bool>(
            "SELECT EXISTS(SELECT FROM pg_locks l JOIN pg_stat_activity a ON l.pid=a.pid
             WHERE a.backend_type='pg_search mutable sweep' AND l.locktype='page'
               AND l.relation='sweep_busy_admission_idx'::regclass AND NOT l.granted)"
        ));
        fixture.value::<bool>("SELECT NOT EXISTS(SELECT FROM paradedb.index_info('sweep_free_admission_idx') WHERE visible AND mutable)")
    });
    fixture.release(&holder);
    holder.finish().1.unwrap();
    fixture
        .controller
        .batch_execute("VACUUM sweep_busy_admission")
        .unwrap();
    fixture
        .controller
        .batch_execute("DROP TABLE sweep_free_admission")
        .unwrap();
    fixture.assert_rows(0);
}
