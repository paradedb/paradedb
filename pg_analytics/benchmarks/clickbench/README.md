# ParadeDB pg_analytics ClickBench Benchmarks

This directory contains the ClickBench benchmark framework for ParadeDB's `pg_analytics`. This folder is the same as the official ClickBench repository's [paradedb/](https://github.com/ClickHouse/ClickBench/tree/main/paradedb) folder made available here for easy benchmarking as part of our development process.

Note: The default dataset pulled by this version of the benchmarks is a 5 million-row version of the full ClickBench dataset. This is for convenience in testing and CI. You can change this in the `benchmark.sh` script.

## Prerequisites

To run the ClickBench benchmarks, you need to install the following packages:

```bash
# Linux Ubuntu/Debian
sudo apt-get install -y postgresql-client
sudo snap install docker

# macOS
brew install docker postgresql
```

## Running the Benchmarks

You can then run the benchmarks via:

```bash
./benchmark.sh -t <version>
```

Note that you may need to run with `sudo` on Linux, depending on your Docker configuration.

The version can be `latest`, to pull the latest ParadeDB image from DockerHub, `x.y.z`, to pull a specific ParadeDB image from DockerHub, or `local`, to build your local branch of ParadeDB.
