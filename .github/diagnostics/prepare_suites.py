"""Add identical telemetry and cache settings to the two controls."""

import os
import re
from pathlib import Path

mode = os.environ["CACHE_MODE"]
assert mode in ("on", "off")
suite = os.environ["TEST_FILE"]
path = Path("source/stressgres/suites") / suite
text = path.read_text(encoding="utf-8")
sections = text.split("[[jobs]]")
for i in range(1, len(sections)):
    title = re.search(r'^title = "([^"]+)"', sections[i], re.MULTILINE).group(1)
    setting = f"SET paradedb.enable_mutable_segment_cache={mode}; SET application_name='{title}';"
    if 'on_connect = """' in sections[i]:
        sections[i] = sections[i].replace(
            'on_connect = """', 'on_connect = """\n' + setting, 1
        )
    else:
        sections[i] = f'\non_connect = """{setting}"""\n' + sections[i]
text = "[[jobs]]".join(sections)
if suite == "partitioned-table.toml":
    text += '''
[[jobs]]
refresh_ms = 1000
title = "Diagnostic Partition Allocation"
log_tps = false
log_columns = ["allocated_bytes", "free_bytes", "distinct_free_bytes", "nonfree_bytes"]
sql = """
SELECT sum(pg_relation_size(i.indexrelid)) AS allocated_bytes,
       sum(f.free_pages) * 8192 AS free_bytes,
       sum(f.distinct_free_pages) * 8192 AS distinct_free_bytes,
       sum(pg_relation_size(i.indexrelid)) - sum(f.distinct_free_pages) * 8192 AS nonfree_bytes
FROM pg_index i JOIN pg_inherits p ON p.inhrelid=i.indrelid
JOIN pg_class c ON c.oid=i.indexrelid JOIN pg_am am ON am.oid=c.relam
CROSS JOIN LATERAL (SELECT count(*) free_pages,count(DISTINCT free_blockno) distinct_free_pages FROM paradedb.fsm_info(i.indexrelid)) f
WHERE p.inhparent='documents'::regclass AND am.amname IN ('bm25','paradedb');
"""

[[jobs]]
refresh_ms = 1000
title = "Diagnostic Partition Heap"
log_tps = false
log_columns = ["dead_tuples", "live_tuples", "updates", "autovacuums"]
sql = """
SELECT sum(n_dead_tup) dead_tuples,sum(n_live_tup) live_tuples,sum(n_tup_upd) updates,sum(autovacuum_count) autovacuums
FROM pg_stat_all_tables WHERE relname IN ('documents_low','documents_high');
"""
'''
else:
    assert suite == "logical-replication-multi-subscriber.toml"
    text += '''
[[jobs]]
refresh_ms = 1000
title = "Diagnostic Subscriber Allocation"
destinations = ["SubscriberA", "SubscriberB"]
log_tps = false
log_columns = ["allocated_bytes", "free_bytes", "distinct_free_bytes"]
sql = """
SELECT pg_relation_size('events_idx') allocated_bytes,count(*)*8192 free_bytes,count(DISTINCT free_blockno)*8192 distinct_free_bytes
FROM paradedb.fsm_info('events_idx');
"""

[[jobs]]
refresh_ms = 1000
title = "Diagnostic Subscriber Heap"
destinations = ["SubscriberA", "SubscriberB"]
log_tps = false
log_columns = ["dead_tuples", "live_tuples", "updates", "autovacuums", "hot_payload_bytes"]
sql = """
SELECT n_dead_tup dead_tuples,n_live_tup live_tuples,n_tup_upd updates,autovacuum_count autovacuums,
       (SELECT sum(octet_length(message)) FROM events WHERE id BETWEEN 1 AND 100) hot_payload_bytes
FROM pg_stat_all_tables WHERE relname='events';
"""

[[jobs]]
refresh_ms = 1000
title = "Diagnostic Subscriber Apply Lag"
destinations = ["SubscriberA", "SubscriberB"]
log_tps = false
log_columns = ["received_not_applied_bytes"]
sql = """
SELECT greatest(0,coalesce(max(pg_wal_lsn_diff(s.received_lsn,o.remote_lsn)),0)) received_not_applied_bytes
FROM pg_stat_subscription s CROSS JOIN pg_replication_origin_status o;
"""
'''
if os.environ.get("POST_CLEANUP") == "on":
    before, tail = text.split("[server.teardown]", 1)
    teardown, after = tail.split("[server.monitor]", 1)
    teardown = re.sub(r'sql = """[\s\S]*?"""', 'sql = ""', teardown, count=1)
    text = before + "[server.teardown]" + teardown + "[server.monitor]" + after
path.write_text(text, encoding="utf-8")
