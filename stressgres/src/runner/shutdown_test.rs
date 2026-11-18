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

use super::{Conn, IoPolicy, WorkerClient};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const TEST_TIMEOUT: Duration = Duration::from_secs(10);

fn read_packet(stream: &mut TcpStream) -> Vec<u8> {
    stream.set_read_timeout(Some(TEST_TIMEOUT)).unwrap();
    let mut length = [0; 4];
    stream.read_exact(&mut length).unwrap();
    let mut body = vec![0; u32::from_be_bytes(length) as usize - length.len()];
    stream.read_exact(&mut body).unwrap();
    body
}

#[test]
fn shutdown_closes_query_socket_after_cancel_request_is_sent() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let addr = listener.local_addr().unwrap();
    let (query_seen, wait_for_query) = mpsc::channel();
    let (cancel_seen, wait_for_cancel) = mpsc::channel();
    let proxy = std::thread::spawn(move || {
        let (mut query, _) = listener.accept().unwrap();
        assert!(read_packet(&mut query).starts_with(&[0, 3, 0, 0]));
        // AuthenticationOk, BackendKeyData (PID 42, key 99), ReadyForQuery.
        query
            .write_all(b"R\0\0\0\x08\0\0\0\0K\0\0\0\x0c\0\0\0\x2a\0\0\0\x63Z\0\0\0\x05I")
            .unwrap();
        let mut tag = [0];
        query.read_exact(&mut tag).unwrap();
        assert_eq!(tag, *b"Q");
        assert_eq!(read_packet(&mut query), b"SELECT 1\0");
        query_seen.send(()).unwrap();

        // Read the complete cancellation request but keep the query socket open
        // without sending a response, as a proxy can during a partition.
        let (mut cancel, _) = listener.accept().unwrap();
        assert_eq!(
            read_packet(&mut cancel),
            [0x04, 0xd2, 0x16, 0x2e, 0, 0, 0, 42, 0, 0, 0, 99]
        );
        drop(cancel);
        cancel_seen.send(()).unwrap();
        assert_eq!(
            query.read(&mut tag).unwrap(),
            0,
            "client must close the socket"
        );
    });

    let policy = IoPolicy {
        interrupt_shutdown: true,
        ..IoPolicy::default()
    };
    let client = WorkerClient::connect(
        &format!(
            "host=127.0.0.1 port={} user=test sslmode=disable",
            addr.port()
        ),
        &policy,
    )
    .unwrap();
    let token = client.client.cancel_token();
    let (query_finished, wait_for_result) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let error = client
            .io
            .run(client.client.batch_execute("SELECT 1"))
            .unwrap_err();
        assert!(error.is_closed(), "{error}");
        query_finished.send(()).unwrap();
    });

    wait_for_query.recv_timeout(TEST_TIMEOUT).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        tokio::time::timeout(TEST_TIMEOUT, token.cancel_query(Conn::tls_connector()))
            .await
            .unwrap()
            .unwrap();
    });
    wait_for_cancel.recv_timeout(TEST_TIMEOUT).unwrap();
    assert!(matches!(
        wait_for_result.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));

    *policy.shutdown.write() = Some(Instant::now());
    wait_for_result.recv_timeout(TEST_TIMEOUT).unwrap();
    worker.join().unwrap();
    proxy.join().unwrap();
}
