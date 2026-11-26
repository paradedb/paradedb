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
use super::{Conn, IoPolicy, JobRunner, Suite};
use parking_lot::RwLock;
use postgres::error::SqlState;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread;
use sysinfo::System;

const CANCELED: &[u8] = b"SERROR\0VERROR\0C57014\0Mcanceling statement due to user request\0\0";

fn reply(stream: &mut TcpStream, tag: u8, body: &[u8]) {
    stream.write_all(&[tag]).unwrap();
    stream
        .write_all(&(4 + body.len() as u32).to_be_bytes())
        .unwrap();
    stream.write_all(body).unwrap();
}

fn with_fake_postgres(abort: bool, run: impl FnOnce(Arc<JobRunner>, &mut Conn)) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        let (mut query, _) = listener.accept().unwrap();
        query.set_nodelay(true).unwrap();
        assert!(read_packet(&mut query).starts_with(&[0, 3, 0, 0]));
        // AuthenticationOk, BackendKeyData (PID 42, key 99), ReadyForQuery.
        query
            .write_all(b"R\0\0\0\x08\0\0\0\0K\0\0\0\x0c\0\0\0\x2a\0\0\0\x63Z\0\0\0\x05I")
            .unwrap();

        let mut executions = 0;
        let mut state = b'I';
        let mut canceled_rollback = false;
        let mut tag = [0];
        while query.read(&mut tag).unwrap() != 0 {
            let body = read_packet(&mut query);
            match tag[0] {
                b'Q' => {
                    let sql = body.strip_suffix(b"\0").unwrap();
                    match sql {
                        b"START TRANSACTION" if state == b'E' => {
                            reply(
                                &mut query,
                                b'E',
                                b"SERROR\0C25P02\0Mtransaction aborted\0\0",
                            );
                        }
                        b"START TRANSACTION" => {
                            state = b'T';
                            reply(&mut query, b'C', b"START TRANSACTION\0");
                        }
                        b"ROLLBACK" if !canceled_rollback => {
                            canceled_rollback = true;
                            state = b'E';
                            reply(&mut query, b'E', CANCELED);
                        }
                        b"ROLLBACK" => {
                            state = b'I';
                            reply(&mut query, b'C', b"ROLLBACK\0");
                        }
                        _ => panic!("unexpected simple query: {}", String::from_utf8_lossy(sql)),
                    }
                    reply(&mut query, b'Z', &[state]);
                }
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
                    if !abort && executions == 2 {
                        reply(&mut query, b'E', CANCELED);
                        continue;
                    }
                    reply(&mut query, b'D', b"\0\x01\0\0\0\x04\0\0\0\x2a");
                    reply(&mut query, b'C', b"SELECT 1\0");
                }
                b'C' => reply(&mut query, b'3', b""), // CloseComplete
                b'S' => reply(&mut query, b'Z', &[state]), // ReadyForQuery
                b'X' => break,
                other => panic!("unexpected frontend tag: {}", char::from(other)),
            }
        }
    });

    let sql = if abort {
        "BEGIN; UPDATE t SET v = v + 1 WHERE id IN (10, 11); ABORT;"
    } else {
        "SELECT 1"
    };
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
            sql = "{sql}"
            "#,
        ))
        .unwrap(),
    ));
    let job = suite.jobs().next().unwrap().clone();
    let mut conn = Conn::open(&suite, &job, &IoPolicy::default()).unwrap();
    let runner = JobRunner::new(
        suite,
        job,
        false,
        Arc::new(AtomicBool::new(true)),
        Arc::new(RwLock::new(System::new().unwrap())),
    )
    .unwrap();
    run(Arc::new(runner), &mut conn);
    drop(conn);
    server.join().unwrap();
}

#[test]
fn canceled_job_keeps_running_on_the_same_connection() {
    with_fake_postgres(false, |runner, conn| {
        runner.run(conn).unwrap();
        let stats = runner.runtime_stats();
        let (info, stats) = stats.first_key_value().unwrap();
        assert_eq!(
            stats.results.as_ref().unwrap_err().code(),
            Some(&SqlState::QUERY_CANCELED)
        );
        assert!(!runner.errored());
        assert!(!runner.connection_errored(info));
        runner.run(conn).unwrap();
    });
}

#[test]
fn canceled_abort_does_not_poison_the_next_iteration() {
    with_fake_postgres(true, |runner, conn| {
        runner.run(conn).unwrap();
        // No reconnect: without cleanup, START TRANSACTION on this session gets 25P02.
        runner.run(conn).unwrap();
        assert!(!runner.errored());
    });
}
