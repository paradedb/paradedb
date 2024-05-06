# ParadeDB pg_lakehouse ClickBench Benchmarks

This directory contains the ClickBench benchmark framework for ParadeDB's `pg_lakehouse`. These benchmarks are not yet published to the official ClickBench repository.

Note: This benchmark pulls the entire 100 million-row ClickBench dataset in Parquet format, which is ~15GBs.

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
./benchmark.sh -t <version> -w <workload>
```

Note that you may need to run with `sudo` on Linux, depending on your Docker configuration.

The workload can be either `single`, to use the ClickBench dataset as a single large Parquet file, or `partitioned`, which uses the ClickBench dataset as one hundred small partitioned Parquet files.

The version can be `latest`, to pull the latest ParadeDB image from DockerHub, `x.y.z`, to pull a specific ParadeDB image from DockerHub, or `local`, to build your local branch of ParadeDB.
