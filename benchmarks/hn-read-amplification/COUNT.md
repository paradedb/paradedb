# Experimental exact COUNT visibility proof

Enable `paradedb.experiment_count_all_visible` to let bare SQL COUNT reuse the existing index-count path after a fresh visibility-map proof over the immutable reader's CTID min/max ranges. Every covered heap page must be all-visible. The same reader, active MVCC snapshot, alive bits, and cleanup pin are retained; mutable segments, unsupported filters, or a failed proof use the existing path. No index format change is required. The GUC is off by default.

On 28,737,557 HN rows under PG17.9, five-/ten-term OR counts returned exactly 1,783,546/1,859,528 and reduced whole-statement cold PostgreSQL buffer loads from 8,846/8,986 to 844/984. OS caches remained warm. Two roughly 31-second paired Benchmarker runs measured approximately 3.7x faster warm counts. Single-term COUNT improved 8,443 to 340 buffer loads. Dense natural-language OR improved only 1.55x in reads.

Selective AND counts regressed from about 2ms to 10ms: a production cost gate is required before default enablement. HOT-specific and broader concurrency testing remain follow-up work. This is an experimental checkpoint, not a production rollout.

## Regression fixture

The included script requires psycopg 3 and an installed experimental pg_search build. It creates and retains only `hn_count_visibility_fixture.docs`; use a disposable database where that schema does not already contain this fixture.

```sh
BENCHMARK_DSN='host=127.0.0.1 port=28821 dbname=hn_benchmark user=YOUR_USER' python count_visible_fixture.py
```

The original run passed 110 flag-off/on comparisons against independent SQL predicates, covering term/OR/AND/phrase/empty queries, deletes, vacuum tombstones, old snapshots, writes, mutable fallback, reindexing, and all-deleted data. A concurrent VACUUM visibly waited on the held reader's BufferPin; releasing it let VACUUM complete. The script writes its plans/results beside itself.
