# ParadeDB Benchmarking Tool

This folder contains scripts that benchmark ParadeDB against other search engines and databases.

## Running Benchmarks

1. Download and unzip the [Wikipedia articles dataset](https://www.dropbox.com/s/wwnfnu441w1ec9p/wiki-articles.json.bz2?dl=0), which will be used for benchmarking.

2. Move the dataset into the current `benchmark` directory.

3. Run the following scripts to obtain benchmarks:

```bash
# Benchmark ParadeDB
./benchmark-paradedb.sh

# Benchmark ElasticSearch
./benchmark-elasticsearch.sh

# Benchmark tsquery
./benchmark-tsquery
```

The results of the benchmarks will be written to a `.csv` file in the `out/` folder.
