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

use postgres::{Client, NoTls};
use rstest::rstest;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::{Mutex, Once};
use std::thread::{sleep, spawn};
use std::time::{Duration, Instant};

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
    client.batch_execute("SET statement_timeout='10s'").unwrap();
    client
}

fn wait_until(description: &str, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !condition() {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {description}"
        );
        sleep(Duration::from_millis(10));
    }
}

#[rstest]
#[case::cancel_reader(false, false, 65251)]
#[case::terminate_reader(true, false, 65252)]
#[case::cancel_builder(false, true, 65253)]
#[case::terminate_builder(true, true, 65254)]
fn backend_exit_releases_cache_resources(
    #[case] terminate: bool,
    #[case] building: bool,
    #[case] id: i32,
) {
    let _schedule = SCHEDULE.lock().unwrap_or_else(|error| error.into_inner());
    let mut control = connect();
    let mut victim = connect();
    let pid: i32 = victim
        .query_one("SELECT pg_backend_pid()", &[])
        .unwrap()
        .get(0);
    let victim = spawn(move || {
        victim.query_one(
            "SELECT tests.test_mutable_cache_access($1, $2, $3)",
            &[&id, &building, &!building],
        )
    });
    wait_until("cache holder parking", || {
        control
            .query_one(
                "SELECT EXISTS(SELECT FROM pg_stat_activity WHERE pid=$1 AND wait_event='PgSleep')",
                &[&pid],
            )
            .unwrap()
            .get(0)
    });
    let (waiter, keeper) = if building {
        let mut waiter = connect();
        let waiter_pid: i32 = waiter
            .query_one("SELECT pg_backend_pid()", &[])
            .unwrap()
            .get(0);
        let handle = spawn(move || {
            waiter.query_one(
                "SELECT tests.test_mutable_cache_access($1, false, false)",
                &[&id],
            )
        });
        wait_until("query waiting for cache builder", || {
            control.query_one(
                "SELECT EXISTS(SELECT FROM pg_stat_activity WHERE pid=$1 AND wait_event='Extension')",
                &[&waiter_pid],
            ).unwrap().get(0)
        });
        (Some(handle), None)
    } else {
        let mut keeper = connect();
        let keeper_pid: i32 = keeper
            .query_one("SELECT pg_backend_pid()", &[])
            .unwrap()
            .get(0);
        let handle = spawn(move || {
            keeper.query_one(
                "SELECT tests.test_mutable_cache_access($1, false, true)",
                &[&id],
            )
        });
        wait_until("second cache reader parking", || {
            control.query_one(
                "SELECT EXISTS(SELECT FROM pg_stat_activity WHERE pid=$1 AND wait_event='PgSleep')",
                &[&keeper_pid],
            ).unwrap().get(0)
        });
        let readers: i64 = control
            .query_one("SELECT tests.test_mutable_cache_readers($1, true)", &[&id])
            .unwrap()
            .get(0);
        assert_eq!(readers, 2);
        (None, Some((keeper_pid, handle)))
    };
    let signal = if terminate {
        "pg_terminate_backend"
    } else {
        "pg_cancel_backend"
    };
    assert!(
        control
            .query_one(&format!("SELECT {signal}($1)"), &[&pid])
            .unwrap()
            .get::<_, bool>(0)
    );
    assert!(victim.join().unwrap().is_err());
    if let Some(waiter) = waiter {
        wait_until("waiter recovering after builder exits", || {
            waiter.is_finished()
        });
        assert!(waiter.join().unwrap().unwrap().get::<_, bool>(0));
    }
    if let Some((pid, keeper)) = keeper {
        wait_until("only the surviving reader's pin remaining", || {
            control
                .query_one("SELECT tests.test_mutable_cache_readers($1, false)", &[&id])
                .unwrap()
                .get::<_, i64>(0)
                == 1
        });
        assert!(
            control
                .query_one("SELECT pg_cancel_backend($1)", &[&pid])
                .unwrap()
                .get::<_, bool>(0)
        );
        assert!(keeper.join().unwrap().is_err());
    }
    wait_until("reader pins reclaimed", || {
        control
            .query_one("SELECT tests.test_mutable_cache_readers($1, true)", &[&id])
            .unwrap()
            .get::<_, i64>(0)
            == -1
    });
    assert_eq!(
        control.query_one("SELECT 1", &[]).unwrap().get::<_, i32>(0),
        1
    );
}

#[test]
fn maintenance_merge_rechecks_heap_visibility() {
    let _schedule = SCHEDULE.lock().unwrap_or_else(|error| error.into_inner());
    let mut client = connect();
    let interval: i32 = client
        .query_one(
            "SELECT setting::int FROM pg_settings WHERE name='paradedb.mutable_sweep_interval'",
            &[],
        )
        .unwrap()
        .get(0);
    let result = catch_unwind(AssertUnwindSafe(|| {
        if interval != 0 {
            client
                .batch_execute("ALTER SYSTEM SET paradedb.mutable_sweep_interval=0")
                .unwrap();
            client.simple_query("SELECT pg_reload_conf()").unwrap();
        }
        wait_until("periodic flushing paused", || {
            client
                .query_one(
                    "SELECT current_setting('paradedb.mutable_sweep_interval')::int=0
                 AND NOT EXISTS(SELECT FROM pg_stat_activity
                     WHERE datname=current_database() AND backend_type='pg_search mutable sweep')",
                    &[],
                )
                .unwrap()
                .get(0)
        });
        client
            .batch_execute(
                "SET max_parallel_workers_per_gather=0;
             SET paradedb.enable_aggregate_custom_scan=off;",
            )
            .unwrap();
        let mut sizes = Vec::new();
        for enabled in [false, true] {
            let table = format!("cache_dead_merge_{enabled}");
            client.batch_execute(&format!(
            "SET paradedb.enable_mutable_segment_cache={enabled};
             DROP TABLE IF EXISTS {table};
             CREATE TABLE {table}(id bigint PRIMARY KEY, body text) WITH(autovacuum_enabled=false);
             CREATE INDEX {table}_idx ON {table} USING paradedb(id, body)
                 WITH(mutable_segment_rows=200, target_segment_count=1, layer_sizes='0', background_layer_sizes='0');"
        )).unwrap();
            client.execute(&format!(
            "INSERT INTO {table}
             SELECT i, 'marker ' || (SELECT string_agg(md5(g::text), ' ') FROM generate_series(1,500) g)
             FROM generate_series(1,200) i"
        ), &[]).unwrap();
            let query = format!("SELECT count(*) FROM {table} WHERE body ||| 'marker'");
            let plan = client
                .query(&format!("EXPLAIN (COSTS OFF) {query}"), &[])
                .unwrap();
            assert!(
                plan.iter()
                    .any(|row| row.get::<_, String>(0).contains("Custom Scan"))
            );
            assert_eq!(client.query_one(&query, &[]).unwrap().get::<_, i64>(0), 200);
            client
                .execute(&format!("DELETE FROM {table}"), &[])
                .unwrap();
            client
                .batch_execute(&format!(
                    "ALTER INDEX {table}_idx SET(mutable_segment_rows=0)"
                ))
                .unwrap();
            assert_eq!(
                client
                    .query_one(
                        &format!("SELECT paradedb.test_merge_index('{table}_idx')"),
                        &[]
                    )
                    .unwrap()
                    .get::<_, i64>(0),
                1
            );
            sizes.push(client.query_one(&format!(
            "SELECT coalesce(sum(byte_size),0)::bigint FROM paradedb.index_info('{table}_idx') WHERE visible"
        ), &[]).unwrap().get::<_, i64>(0));
            client
                .batch_execute(&format!("DROP TABLE {table}"))
                .unwrap();
        }
        assert!(
            sizes[1] <= sizes[0] + 8192,
            "cached merge retained dead contents: {sizes:?}"
        );
    }));
    if interval != 0 {
        client
            .batch_execute(&format!(
                "ALTER SYSTEM SET paradedb.mutable_sweep_interval={interval}"
            ))
            .unwrap();
        client.simple_query("SELECT pg_reload_conf()").unwrap();
    }
    if let Err(error) = result {
        resume_unwind(error);
    }
}
