# syntax=docker/dockerfile:1
#
# pg18 with this branch's pg_search built with `vector_audit`, the router-audit binary, pdbench
# and the runner's scripts. The build context is the repository; `--build-context pdbench=<dir>`
# supplies the pdbench sources.

ARG BASE_IMAGE=paradedb/paradedb:latest-pg18
FROM ${BASE_IMAGE} AS base

FROM base AS builder

# hadolint ignore=DL3002
USER 0
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
    postgresql-server-dev-18 build-essential ca-certificates curl git \
    pkg-config libssl-dev libicu-dev clang lld binutils libopenblas-dev && \
    rm -rf /var/lib/apt/lists/*

ENV CARGO_HOME=/opt/cargo RUSTUP_HOME=/opt/rustup
ENV PATH="/opt/cargo/bin:/usr/lib/postgresql/18/bin:${PATH}"
WORKDIR /versions
COPY rust-toolchain.toml Cargo.toml ./
RUN rust=$(sed -nE 's/^channel = "([^"]+)"$/\1/p' rust-toolchain.toml) && \
    pgrx=$(sed -nE 's/^pgrx = "=?([^"]+)"$/\1/p' Cargo.toml) && \
    test -n "$rust" && test -n "$pgrx" && \
    curl -fsSL --retry 5 --retry-all-errors https://sh.rustup.rs -o /tmp/rustup.sh && \
    sh /tmp/rustup.sh -y --profile minimal --default-toolchain "$rust" && \
    rustup toolchain install 1.97.1 --profile minimal && \
    cargo install cargo-pgrx --version "$pgrx" --debug --locked && \
    cargo pgrx init --pg18=/usr/lib/postgresql/18/bin/pg_config

WORKDIR /pdbench
COPY --from=pdbench . .
RUN cargo +1.97.1 build --locked --release -p pdbench --target-dir target

WORKDIR /build
COPY . .
RUN /bin/bash <<'BUILD'
set -euo pipefail
cargo pgrx package --package pg_search --profile release --features vector_audit \
    --pg-config /usr/lib/postgresql/18/bin/pg_config --out-dir /package
cargo build --release -p router-audit
mkdir -p /artifacts/usr/lib/postgresql/18/lib /artifacts/usr/share/postgresql/18/extension \
    /artifacts/usr/local/bin /artifacts/opt/router-audit/benchmarks/cohere
find /package -type f -name 'pg_search*.so' -exec cp {} /artifacts/usr/lib/postgresql/18/lib/ \;
find /package -type f -name 'pg_search*.control' -exec cp {} /artifacts/usr/share/postgresql/18/extension/ \;
find /package -type f -name 'pg_search*.sql' -exec cp {} /artifacts/usr/share/postgresql/18/extension/ \;
cp target/release/router-audit /pdbench/target/release/pdbench /artifacts/usr/local/bin/
cp -R benchmarks/cohere/assets /artifacts/opt/router-audit/benchmarks/cohere/assets
cp -R benchmarks/router-audit /artifacts/opt/router-audit/benchmarks/router-audit
BUILD

FROM base AS runtime

RUN rm -f /usr/lib/postgresql/18/lib/pg_search*.so /usr/lib/postgresql/18/lib/pg_search*.so.dbg \
    /usr/share/postgresql/18/extension/pg_search*.control \
    /usr/share/postgresql/18/extension/pg_search*.sql
COPY --from=builder /artifacts/ /
COPY docker/bootstrap.sh /docker-entrypoint-initdb.d/10_bootstrap_paradedb.sh

# hadolint ignore=DL3002
USER 0
RUN apt-get update && apt-get install -y --no-install-recommends python3 python3-numpy && \
    rm -rf /var/lib/apt/lists/*
USER postgres
