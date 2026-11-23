// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

use super::shutdown_test::read_packet;
use super::{GraceWindow, IoPolicy, JobRunner, Suite, SuiteRunner};
use parking_lot::RwLock;
use postgres::error::SqlState;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;
use sysinfo::System;

const TEST_TIMEOUT: Duration = Duration::from_secs(10);

fn reply(stream: &mut TcpStream, tag: u8, body: &[u8]) {
    stream.write_all(&[tag]).unwrap();
    stream
        .write_all(&(4 + body.len() as u32).to_be_bytes())
        .unwrap();
    stream.write_all(body).unwrap();
}

#[test]
fn canceled_job_keeps_running_on_the_same_connection() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    let (resumed, wait_for_resume) = mpsc::channel();
    let (finish, wait_for_finish) = mpsc::channel();
    let server = thread::spawn(move || {
        let (mut query, _) = listener.accept().unwrap();
        query.set_nodelay(true).unwrap();
        assert!(read_packet(&mut query).starts_with(&[0, 3, 0, 0]));
        // AuthenticationOk, BackendKeyData (PID 42, key 99), ReadyForQuery.
        query
            .write_all(b"R\0\0\0\x08\0\0\0\0K\0\0\0\x0c\0\0\0\x2a\0\0\0\x63Z\0\0\0\x05I")
            .unwrap();

        let mut executions = 0;
        let mut tag = [0];
        while query.read(&mut tag).unwrap() != 0 {
            read_packet(&mut query);
            match tag[0] {
                b'P' => reply(&mut query, b'1', b""), // ParseComplete
                b'D' => {
                    reply(&mut query, b't', b"\0\0"); // No parameters
                    // RowDescription: one int4 (OID 23) column in text format.
                    reply(
                        &mut query,
                        b'T',
                        b"\0\x01result\0\0\0\0\0\0\0\0\0\0\x17\0\x04\xff\xff\xff\xff\0\0",
                    );
                }
                b'B' => reply(&mut query, b'2', b""), // BindComplete
                b'E' => {
                    executions += 1;
                    // The first Execute is Conn::open's backend PID probe, not a job iteration.
                    if executions == 3 {
                        reply(
                            &mut query,
                            b'E',
                            b"SERROR\0VERROR\0C57014\0Mcanceling statement due to user request\0\0",
                        );
                        continue;
                    }
                    if executions == 4 {
                        // Only this session is served: reaching the third job iteration proves
                        // the worker did not reconnect. Hold it so the test can inspect stats.
                        resumed.send(()).unwrap();
                        wait_for_finish.recv_timeout(TEST_TIMEOUT).unwrap();
                    }
                    reply(&mut query, b'D', b"\0\x01\0\0\0\x04\0\0\0\x2a");
                    reply(&mut query, b'C', b"SELECT 1\0");
                }
                b'C' => reply(&mut query, b'3', b""), // CloseComplete
                b'S' => reply(&mut query, b'Z', b"I"), // ReadyForQuery
                b'X' => break,
                other => panic!("unexpected frontend tag: {}", char::from(other)),
            }
        }
    });

    let suite = Arc::new(Suite::new(
        toml::from_str(&format!(
            r#"
            [[server]]
            name = "Primary"
            style.With.connection_string = "postgresql://test@127.0.0.1:{port}/test?sslmode=disable"
            setup.sql = ""
            teardown.sql = ""
            monitor.sql = ""

            [[jobs]]
            sql = "SELECT 1"
            refresh_ms = 1
            "#,
        ))
        .unwrap(),
    ));
    let job = suite.jobs().next().unwrap().clone();
    let alive = Arc::new(AtomicBool::new(true));
    let runner = Arc::new(
        JobRunner::new(
            suite,
            job,
            false,
            alive.clone(),
            Arc::new(RwLock::new(System::new().unwrap())),
        )
        .unwrap(),
    );
    let worker = thread::spawn({
        let runner = runner.clone();
        move || {
            SuiteRunner::job_runner_worker(
                Arc::new(AtomicBool::new(false)),
                Duration::from_millis(1),
                GraceWindow::fixed(Duration::ZERO),
                IoPolicy::default(),
                &runner,
            )
        }
    });

    wait_for_resume
        .recv_timeout(TEST_TIMEOUT)
        .expect("job did not resume after cancellation");
    let stats = runner.runtime_stats();
    let (conn, stats) = stats.first_key_value().unwrap();
    let errored = runner.errored();
    let connection_errored = runner.connection_errored(conn);

    alive.store(false, Ordering::Release);
    finish.send(()).unwrap();
    worker.join().unwrap().unwrap();
    server.join().unwrap();

    assert_eq!(
        stats.results.as_ref().unwrap_err().code(),
        Some(&SqlState::QUERY_CANCELED)
    );
    assert!(!errored, "cancellation is not a job failure");
    assert!(
        !connection_errored,
        "cancellation is not a connection failure"
    );
}
