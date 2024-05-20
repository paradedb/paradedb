# ParadeDB Benchmarks

The ParadeDB benchmarks are split into:

- [Search - pg_search](../pg_search/benchmarks/README.md)

In the future, we'll be adding benchmarks for workloads intersecting both search and analytics. If there's anything specific you'd like to see, please open a GitHub issue or come chat with us in the [ParadeDB Community Slack](https://join.slack.com/t/paradedbcommunity/shared_invite/zt-217mordsh-ielS6BiZf7VW3rqKBFgAlQ).

## Benchmark pg_search

Currently, the following systems are benchmarked:

- [x] ParadeDB `pg_search`
- [x] PostgreSQL tsquery/tsvector
- [x] Elasticsearch

### Experimental Setup

The benchmarks below were run on the following hardware and software:

```bash
# Instance
Instance Type: Amazon EC2 c6a.12xlarge

# Image
OS Version: Ubuntu 22.04.4 LTS
Kernel Version: 6.5.0-1016-aws

# CPU
vCPUs: 48
CPU: AMD EPYC 7R13 Processor
CPU MHz: 2649.998
Cache size: 768 KiB
Bogomips: 5299.99
Address sizes: 48 bits physical, 48 bits virtual

# Memory
RAM: 96GB
Storage: 500GB gp3, 16,000 IOPS and 1,000 MB/s throughput
Max Data Disks: 8
Max temp storage throughput: 19000 / 250 IOPS/MBps
Max uncached disk throughput: 6400 / 144 IOPS/MBps
Max burst uncached disk throughput: 20000 / 600 IOPS/MBps

# Network
Max NICs: 4
Max network bandwidth: 18750 Mbps
```

Data is generated and benchmarks are run with the [cargo-paradedb](/cargo-paradedb/README.md) tool.

For index/table building benchmarks no warmup steps are taken, and the times are recorded based off a single run, as benchmarks against large datasets can take many hours.

For query benchmarks, the Rust Criterion library is used. Iterations, warmups, and averages are reported in the output. The query used is a simple search of the word "flame" in the "message" field of the [Elasticsearch benchmark corpus](https://github.com/elastic/elasticsearch-opensearch-benchmark).

- ParadeDB `pg_search`: 0.6.0
- PostgreSQL: 16.2
- Elasticsearch: 7.17.20

For any questions, clarifications, or suggestions regarding our benchmarking experimental setup, please open a GitHub issue or come chat with us in the [ParadeDB Community Slack](https://join.slack.com/t/paradedbcommunity/shared_invite/zt-217mordsh-ielS6BiZf7VW3rqKBFgAlQ).

### Results

Building `pg_search` index, indexing the `message` column, over 1 billion rows.

```text
Start time: SystemTime { tv_sec: 1712856911, tv_nsec: 741370871 }
End time: SystemTime { tv_sec: 1712866842, tv_nsec: 72802069 }
Duration: 9930331 milliseconds
Duration: 9930.3314 seconds
Duration: 165.5055 minutes
Duration: 2.7584 hours
```

Querying `pg_search` index for `"message:flame"`:

```text
Benchmarking Search Query/bench_eslogs_query_search_index
Benchmarking Search Query/bench_eslogs_query_search_index: Warming up for 3.0000 s
Benchmarking Search Query/bench_eslogs_query_search_index: Collecting 60 samples in estimated 6.6780 s (3660 iterations)
Benchmarking Search Query/bench_eslogs_query_search_index: Analyzing
Search Query/bench_eslogs_query_search_index
                        time:   [1.5890 ms 1.6117 ms 1.6437 ms]
Found 7 outliers among 60 measurements (11.67%)
  3 (5.00%) high mild
  4 (6.67%) high severe
```

Building Elasticsearch index, indexing the `message` column, over 1 billion rows:

```text
Start time: SystemTime { tv_sec: 1713302753, tv_nsec: 701639825 }
End time: SystemTime { tv_sec: 1713347291, tv_nsec: 876946205 }
Duration: 44538175 milliseconds
Duration: 44538.1753 seconds
Duration: 742.3029 minutes
Duration: 12.3717 hours
```

Querying Elasticsearch index, for the term `flame` in the `message` field:

```text
Benchmarking Elasticsearch Index/bench_eslogs_query_elastic_table
Benchmarking Elasticsearch Index/bench_eslogs_query_elastic_table: Warming up for 3.0000 s
Benchmarking Elasticsearch Index/bench_eslogs_query_elastic_table: Collecting 60 samples in estimated 8.3315 s (3660 iterations)
Benchmarking Elasticsearch Index/bench_eslogs_query_elastic_table: Analyzing
Elasticsearch Index/bench_eslogs_query_elastic_table
                        time:   [1.9526 ms 1.9948 ms 2.0516 ms]
                        change: [+775.44% +798.00% +820.76%] (p = 0.00 < 0.05)
                        Performance has regressed.
Found 4 outliers among 60 measurements (6.67%)
  2 (3.33%) high mild
  2 (3.33%) high severe
```
## Benchmark pg_lakehouse

Note: This benchmark pulls the entire 100 million-row ClickBench dataset in Parquet format, which is ~15GBs.

You can run the benchmarks via the `cargo-paradedb` tool with:

```bash
cargo paradedb bench hits run
```

The benchmark tool will look for a `DATABASE_URL` environment variable for a running Postgres instance. You can also pass the url directly with the `--url` option.

The benchmark tool also accepts a `--workload / -w` option. This can be either `single`, to use the ClickBench dataset as a single large Parquet file, or `partitioned`, to use the ClickBench dataset as one hundred small partitioned Parquet files. The default is `single`.
