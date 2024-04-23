# ParadeDB TPC-H Benchmarks

This directory contains the TPC-H benchmark framework for ParadeDB. Note that because the TPC-H benchmark structure is slightly modified for convenience of running, mainly to facilitate running queries in a reproducible environment and on macOS for development purposes, the results should _not_ be considered official TPC-H results, as per their guidelines.

## Prerequisites

To run the TPC-H benchmarks, you need to install the following packages:

```bash
# Linux Ubuntu/Debian
sudo apt-get install -y unzip make gcc
sudo snap install docker

# macOS
brew install unzip make gcc docker
```

## Running the Benchmarks

You can then run the benchmarks via:

```bash
./benchmark.sh -t <version> -w <workload> -s <scale_factor>
```

Note that you may need to run with `sudo` on Linux, depending on your Docker configuration.

The version can be `latest`, to pull the latest ParadeDB image from DockerHub, `x.y.z`, to pull a specific ParadeDB image from DockerHub, or `local`, to build your local branch of ParadeDB.

The workload can be either `olap`, to create only `parquet` tables, or `htap` to create a mix of `parquet` and standard Postgres `heap` tables on which to run the queries.

The scale factor can be either `1, 10, 100, 1000`, which is the number of GBs of data generated to run the TPC-H benchmark on. For testing, we recommend setting a scale factor of `1`. For official benchmarking, we recommend either `100` or `1000`. Note that even on a large machine, generating `100` GBs takes upwards of 30 minutes, and generating `1000` GBs can take several hours.
