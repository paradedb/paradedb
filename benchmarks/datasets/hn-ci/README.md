# Hacker News Top-K workloads

The workload uses the existing 28,737,557-document `hn-ci` snapshot. The workflow
downloads the data from S3 and overlays this directory's index definition. Query
fixtures and validation are versioned here; changing them does not require
replacing the S3 archive.

Every query selects the full row and returns at most 10 rows. The original
40-term `single_topk` baseline is retained. Each additional category has nominal
50%, 10%, 1%, and 0.1% fast-field pass-rate buckets:

| Category          | Predicate                                                                                                    | Ordering             |
| ----------------- | ------------------------------------------------------------------------------------------------------------ | -------------------- |
| `text_and_ff`     | text AND timestamp range                                                                                     | BM25 descending      |
| `text_or_ff`      | text OR timestamp range                                                                                      | BM25 descending      |
| `text_and_ff_and` | text AND (timestamp range AND points range)                                                                  | BM25 descending      |
| `text_and_ff_or`  | text AND (timestamp range OR points range)                                                                   | BM25 descending      |
| `ff_and`          | timestamp range AND points range                                                                             | timestamp descending |
| `ff_or`           | timestamp range OR points range                                                                              | timestamp descending |
| `ff_five`         | `((time >= cutoff AND score <= maximum) OR (type = 'story' AND descendants >= minimum)) AND deleted = false` | timestamp descending |

`score` in a predicate is HN points; `pdb.score(id)` is text relevance. The
five-filter category always uses the same five fields and Boolean expression.
Timestamp ordering gives filter-only queries a useful ranking without text.
Those queries include `id @@@ pdb.all()` to use ParadeDB.

There are 29 scenarios with 40 deterministic fixtures each. Text scenarios
cycle through all 40 original single-word search terms. Filter-only scenarios
cycle through 40 different filter tuples. Parameter values are frozen, and every
iteration advances to the next fixture in order.

## Calibration and validation

Bucket names describe the fraction of all indexed documents passing the
**complete fast-field predicate**, not the final text/filter union or intersection.
Timestamp cutoffs and integer thresholds were initialized from a deterministic
2% `TABLESAMPLE SYSTEM` sample with seed 6779, then checked against exact counts
on the full snapshot. Within each bucket, the 40 fixtures vary around the target.
The fixture file records exact filter counts, pass rates, text counts, intersection
counts, and final match counts where applicable. Correlated filters are counted
together; their selectivities are not multiplied.

The fixture calibration used PostgreSQL 18.3 and pg_search 0.25.11. Calibration
counts are descriptive; current-version CI validation executes the actual queries.
All 1,160 fixtures returned rows locally. Three tight text-filter scenarios contain
a fixture with seven matches, so a full ten-row page is not required. Empty results
are always failures.

`preflight.py` emits a SQL validation block. After loading the CI index, the
workflow runs it using `psql -v ON_ERROR_STOP=1`. It checks the required fast fields,
executes all fixtures, and rejects empty results, missing Top-K plans, and residual
PostgreSQL filters before measurements begin. The k6 script independently fails on
query errors or empty results during the run.

## Execution and output

The workflow runs one scenario at a time, with one query VU and a discarded
30-second warmup immediately before measuring. Measurements start at 30 seconds;
the three slowest local scenarios start at 37, 39, or 54 seconds. If a CI measurement
does not complete at least two full fixture cycles, it is discarded and retried
with a longer duration. Only the accepted measurement is published.

Every scenario exports its own Benchmarker JSON/HTML dashboard and p50, p95,
p99, and QPS metrics. The original baseline retains the `topk` output filename
and `single_topk` chart name to preserve its existing metric identity.

Run via the `benchmark-benchmarker-hn` PR label or dispatch
`benchmark-pg_search-benchmarker.yml` with dataset `hn-ci`. A dispatch on `main`
after merging publishes the refreshed baseline. This change does not enable HN
on every main push.
