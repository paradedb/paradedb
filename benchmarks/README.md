# ParadeDB Benchmarks

This folder contains the results and scripts for benchmarking ParadeDB against other search engines and databases. Currently, the following systems are benchmarked:

- [x] ParadeDB
- [x] ElasticSearch
- [x] PostgreSQL tsquery

If you'd like to see benchmarks against another system, please open an issue or a pull request.

## Results

TODO

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
