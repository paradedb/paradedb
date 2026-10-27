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
                vec!["shared_preload_libraries='pg_search'"],
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
