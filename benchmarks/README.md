# ParadeDB Benchmarks

This folder contains the results and scripts for benchmarking ParadeDB against other search engines and databases. Currently, the following systems are benchmarked:

- [x] ParadeDB
- [x] ElasticSearch
- [x] PostgreSQL tsquery

If you'd like to see benchmarks against another system, please open an issue or a pull request.

## Results

### Experimental Setup

The benchmarks below were run on a 2021 MacBook Pro with 16GB of RAM and an Apple M1 processor. A more thorough benchmarking setup is coming soon.

### pg_bm25

On a table with 1 million rows, `pg_bm25` indexes 50 seconds faster than `tsvector` and searches + ranks
results 20x faster. Indexing and search times are nearly identical to those of a dedicated ElasticSearch
instance.

<img src="../docs/images/bm25_index_benchmark.png" alt="" width="100%">

<img src="../docs/images/bm25_search_benchmark.png" alt="" width="100%">

## Generating Benchmarks

To generate new benchmarks, simply run the relevant Bash script:

```bash
# Benchmark ParadeDB
./benchmark-paradedb.sh

# Benchmark ElasticSearch
./benchmark-elasticsearch.sh

# Benchmark tsquery
./benchmark-tsquery
```

The results of the benchmarks will be written to a `.csv` file in the `out/` folder.
