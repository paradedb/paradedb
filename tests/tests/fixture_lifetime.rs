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

use async_std::task::block_on;
use rstest::rstest;
use sqlx::{Connection, PgConnection, Postgres, testing::TestSupport};
use std::sync::mpsc;
use std::thread;
use tests::fixtures::*;

#[derive(Clone, Copy, Debug)]
enum Exit {
    Return,
    Error,
    Panic,
}

fn observer() -> PgConnection {
    block_on(PgConnection::connect(
        &dotenvy::var("DATABASE_URL").expect("DATABASE_URL must be set"),
    ))
    .expect("connect to the parent database")
}

fn database_exists(conn: &mut PgConnection, name: &str) -> bool {
    block_on(
        sqlx::query_scalar("SELECT EXISTS (SELECT FROM pg_database WHERE datname = $1)")
            .bind(name)
            .fetch_one(conn),
    )
    .unwrap()
}

fn registered(conn: &mut PgConnection, name: &str) -> bool {
    block_on(
        sqlx::query_scalar("SELECT EXISTS (SELECT FROM _sqlx_test.databases WHERE db_name = $1)")
            .bind(name)
            .fetch_one(conn),
    )
    .unwrap()
}

fn citus_workers(conn: &mut PgConnection, name: &str) -> i64 {
    block_on(sqlx::query_scalar(
        "SELECT count(*) FROM pg_stat_activity WHERE datname = $1 AND backend_type LIKE 'Citus%'",
    )
    .bind(name)
    .fetch_one(conn))
    .unwrap()
}

fn finish(exit: Exit) -> Result<(), &'static str> {
    match exit {
        Exit::Return => Ok(()),
        Exit::Error => Err("test body returned an error"),
        Exit::Panic => panic!("test body panicked"),
    }
}

// The parent observes real catalogs after joining the test thread, including its
// TLS destructors. Never clean up another test's databases, even on the old code.
fn check_teardown(
    body: impl FnOnce(mpsc::Sender<String>) -> Result<(), &'static str> + Send + 'static,
    exit: Exit,
    citus: bool,
) {
    let mut admin = observer();
    let (send, recv) = mpsc::channel();
    let result = thread::spawn(move || body(send)).join();
    let name = recv.recv().expect("test published its database name");
    let exists = database_exists(&mut admin, &name);
    let tracked = registered(&mut admin, &name);
    let workers = citus_workers(&mut admin, &name);
    eprintln!(
        "after {exit:?}, citus={citus}: database_exists={exists}, registered={tracked}, citus_workers={workers}"
    );

    // The regression intentionally leaks on main. Remove only this test's DB so
    // a fail-before run leaves the dedicated verification cluster clean as well.
    if exists || tracked {
        block_on(Postgres::cleanup_test(&name)).expect("clean the regression's own database");
    }
    match exit {
        Exit::Return => assert_eq!(result.unwrap(), Ok(())),
        Exit::Error => assert!(result.unwrap().is_err()),
        Exit::Panic => assert!(result.is_err()),
    }
    assert!(
        !exists,
        "database {name} leaked after the test thread exited"
    );
    assert!(!tracked, "SQLx still tracks {name}");
    assert_eq!(workers, 0, "Citus workers outlived {name}");
}

#[rstest]
#[case::returns(Exit::Return)]
#[case::errors(Exit::Error)]
#[case::panics(Exit::Panic)]
fn connection_fixture_lifetime(#[case] exit: Exit, #[values(false, true)] citus: bool) {
    if citus
        && !"SHOW shared_preload_libraries"
            .fetch_one::<(String,)>(&mut observer())
            .0
            .contains("citus")
    {
        eprintln!("Skipping Citus lifetime check: Citus is not preloaded");
        return;
    }
    check_teardown(
        move |send| {
            let mut conn = conn::default();
            let name = "SELECT current_database()"
                .fetch_one::<(String,)>(&mut conn)
                .0;
            send.send(name.clone()).unwrap();
            let mut admin = observer();
            assert!(database_exists(&mut admin, &name));
            assert!(registered(&mut admin, &name));
            if citus {
                "CREATE EXTENSION citus".execute(&mut conn);
                // Positive control: a daemon really occupied a worker slot.
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
                while citus_workers(&mut admin, &name) == 0 {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "Citus daemon did not start"
                    );
                    thread::sleep(std::time::Duration::from_millis(50));
                }
            }
            // Longer than PostgreSQL's five-second DROP DATABASE wait. The test's
            // connection must still work, and no cleanup may hold its database lock.
            assert_eq!(
                "SELECT 42 FROM pg_sleep(6)"
                    .fetch_one::<(i32,)>(&mut conn)
                    .0,
                42
            );
            assert_eq!("SELECT 43".fetch_one::<(i32,)>(&mut conn).0, 43);
            finish(exit)
        },
        exit,
        citus,
    );
}

#[rstest]
#[case::returns(Exit::Return)]
#[case::errors(Exit::Error)]
#[case::panics(Exit::Panic)]
fn database_fixture_lifetime(#[case] exit: Exit) {
    check_teardown(
        move |send| {
            // Also exercise direct Db callers with multiple connections, including
            // an async runtime and unwinding inside it.
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async move {
                let db = database::default();
                let mut first = db.connection().await;
                let mut second = db.connection().await;
                let name: String = sqlx::query_scalar("SELECT current_database()")
                    .fetch_one(&mut first)
                    .await
                    .unwrap();
                send.send(name.clone()).unwrap();
                assert!(database_exists(&mut observer(), &name));
                drop(db);
                sqlx::query("SELECT pg_sleep(6)")
                    .execute(&mut first)
                    .await
                    .unwrap();
                assert!(
                    sqlx::query("SELECT 1 / 0")
                        .execute(&mut second)
                        .await
                        .is_err()
                );
                assert_eq!(
                    sqlx::query_scalar::<_, i32>("SELECT 42")
                        .fetch_one(&mut second)
                        .await
                        .unwrap(),
                    42
                );
                finish(exit)
            })
        },
        exit,
        false,
    );
}
