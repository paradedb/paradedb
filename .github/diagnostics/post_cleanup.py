"""Measure which allocated pages remain after quiescence, VACUUM, and REINDEX."""

import json
import re
import subprocess
import time
from pathlib import Path

data = Path("/tmp/stressgres/Primary.data")
options = (data / "postmaster.opts").read_text(encoding="utf-8")
port = re.search(r"port=(\d+)", options).group(1)
binpath = Path("/usr/lib/postgresql/18/bin")
command = [
    str(binpath / "psql"),
    "-XAt",
    "-v",
    "ON_ERROR_STOP=1",
    "-h",
    "localhost",
    "-p",
    port,
    "-d",
    "stressgres",
]
results = []
SNAPSHOT = """
SELECT json_build_object(
 'indexes',(SELECT json_agg(q) FROM (
    SELECT c.relname,pg_relation_size(i.indexrelid) allocated_bytes,f.*
    FROM pg_index i JOIN pg_inherits p ON p.inhrelid=i.indrelid
    JOIN pg_class c ON c.oid=i.indexrelid JOIN pg_am am ON am.oid=c.relam
    CROSS JOIN LATERAL (SELECT count(*)*8192 free_bytes,count(DISTINCT free_blockno)*8192 distinct_free_bytes
                       FROM paradedb.fsm_info(i.indexrelid::regclass)) f
    WHERE p.inhparent='documents'::regclass AND am.amname IN ('paradedb','bm25')
 ) q),
 'segments',(SELECT json_agg(s) FROM paradedb.index_info('documents_idx',true) s),
 'heap',(SELECT json_build_object('rows',count(*),'payload_bytes',sum(octet_length(body))) FROM documents),
 'activity',(SELECT json_agg(q) FROM (SELECT backend_type,state,wait_event_type,wait_event,backend_xmin::text
                                    FROM pg_stat_activity WHERE datname=current_database()) q)
);
"""


def record(label):
    """Capture allocation and live-segment metadata without changing the index."""
    value = subprocess.check_output(command + ["-c", SNAPSHOT], text=True)
    results.append({"stage": label, "snapshot": json.loads(value)})
    Path("post-cleanup.json").write_text(
        json.dumps(results, indent=2), encoding="utf-8"
    )


try:
    subprocess.run(
        [
            str(binpath / "pg_ctl"),
            "-D",
            str(data),
            "-l",
            "/tmp/stressgres/PostCleanup.log",
            "-o",
            (
                f"-c port={port} -c shared_preload_libraries=pg_search -c max_worker_processes=96 "
                "-c paradedb.mutable_sweep_interval=0 "
                "-c paradedb.global_enable_background_merging=off "
                "-c autovacuum=off"
            ),
            "-w",
            "start",
        ],
        check=True,
    )
    record("quiescent_after_restart")
    for i in range(2):
        subprocess.run(
            command + ["-c", "VACUUM (INDEX_CLEANUP ON) documents"], check=True
        )
        record(f"after_vacuum_{i + 1}")
        time.sleep(1)
    verify = subprocess.check_output(
        command
        + [
            "-c",
            "SELECT json_agg(v) FROM pdb.verify_index('documents_idx',heapallindexed=>true) v",
        ],
        text=True,
    )
    results.append({"stage": "verification", "checks": json.loads(verify)})
    for _ in range(10):
        subprocess.run(
            command
            + [
                "-c",
                "UPDATE documents SET body=body||' updated' WHERE id BETWEEN 1 AND 100",
            ],
            check=True,
        )
    record("after_ten_more_updates")
    subprocess.run(command + ["-c", "VACUUM (INDEX_CLEANUP ON) documents"], check=True)
    record("after_reuse_cleanup")
    subprocess.run(command + ["-c", "REINDEX INDEX documents_idx"], check=True)
    record("after_reindex")
finally:
    subprocess.run(
        [str(binpath / "pg_ctl"), "-D", str(data), "-m", "fast", "-w", "stop"],
        check=False,
    )
    Path("post-cleanup.json").write_text(
        json.dumps(results, indent=2), encoding="utf-8"
    )
