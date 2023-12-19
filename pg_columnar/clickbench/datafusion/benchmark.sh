#!/bin/bash

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rust-init.sh
bash rust-init.sh -y
source ~/.cargo/env


# Install Dependencies
sudo yum update -y
sudo yum install gcc -y


# Install DataFusion main branch
git clone https://github.com/apache/arrow-datafusion.git
cd arrow-datafusion/datafusion-cli
git checkout 33.0.0
CARGO_PROFILE_RELEASE_LTO=true RUSTFLAGS="-C codegen-units=1" cargo build --release
export PATH="`pwd`/target/release:$PATH"
cd ../..


# Download benchmark target data, single file
wget --no-verbose --continue https://datasets.clickhouse.com/hits_compatible/hits.parquet

# Download benchmark target data, partitioned
mkdir -p partitioned
seq 0 99 | xargs -P100 -I{} bash -c 'wget --no-verbose --directory-prefix partitioned --continue https://datasets.clickhouse.com/hits_compatible/athena_partitioned/hits_{}.parquet'

# Run benchmarks for single parquet and partitioned
./run.sh single
./run.sh partitioned
