ARG PG_VERSION_MAJOR

###############################################
# First Stage: Base
###############################################

FROM postgres:${PG_VERSION_MAJOR}-bookworm as base

# Declare all the build arguments and set them to
# environment variables for use in build and runtime
ARG PG_VERSION_MAJOR
ARG PG_BM25_VERSION
ARG PG_ANALYTICS_VERSION
ARG PG_SPARSE_VERSION
ARG POSTHOG_API_KEY
ARG POSTHOG_HOST
ARG COMMIT_SHA
ARG TELEMETRY
ARG BUILD_DATE

ENV PG_VERSION_MAJOR=${PG_VERSION_MAJOR} \
    PG_BM25_VERSION=${PG_BM25_VERSION} \
    PG_ANALYTICS_VERSION=${PG_ANALYTICS_VERSION} \
    PG_SPARSE_VERSION=${PG_SPARSE_VERSION} \
    POSTHOG_API_KEY=${POSTHOG_API_KEY} \
    POSTHOG_HOST=${POSTHOG_HOST} \
    COMMIT_SHA=${COMMIT_SHA} \
    TELEMETRY=${TELEMETRY} \
    BUILD_DATE=${BUILD_DATE} \
    DEBIAN_FRONTEND=noninteractive \
    TZ=UTC

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

# Install common dependencies to builder and runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    wget \
    sudo \
    gnupg \
    gcc \
    make \
    uuid-runtime \
    software-properties-common \
    ca-certificates \
    libssl-dev \
    libopenblas-dev \
    python3-dev \
    python3-pip \
    && rm -rf /var/lib/apt/lists/*

# Install apt-fast
RUN /bin/bash -c "$(curl -sL https://git.io/vokNn)"

# Add PostgreSQL's third party repository to get the latest versions
RUN curl -s https://www.postgresql.org/media/keys/ACCC4CF8.asc | apt-key add -
RUN echo "deb http://apt.postgresql.org/pub/repos/apt/ $(lsb_release -cs)-pgdg main" | tee /etc/apt/sources.list.d/pgdg.list

RUN apt-fast update && apt-fast install -y --no-install-recommends \
    postgresql-server-dev-${PG_VERSION_MAJOR} \
    && rm -rf /var/lib/apt/lists/*

###############################################
# Second Stage: Builder
###############################################

FROM base as builder

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

# Install Rust (specific version) and other build dependencies
RUN apt-fast update && apt-fast install -y --no-install-recommends \
    build-essential \
    checkinstall \
    clang \
    git \
    cmake \
    pkg-config \
    liblz4-dev \
    libcurl4-openssl-dev \
    && rm -rf /var/lib/apt/lists/* \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    && /root/.cargo/bin/rustup default 1.73.0

ENV PATH="/root/.cargo/bin:$PATH" \
    PGX_HOME=/usr/lib/postgresql/${PG_VERSION_MAJOR}

RUN cargo install --locked cargo-pgrx --version 0.11.1 && \
    cargo pgrx init "--pg${PG_VERSION_MAJOR}=/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config"

######################
# pg_bm25
######################

FROM builder as builder-pg_bm25

WORKDIR /tmp/pg_bm25

COPY pg_bm25/ /tmp/pg_bm25
COPY shared/ /tmp/shared

# Use the build argument to update the version in Cargo.toml
RUN sed -i "/\[package\]/,/\[/{s/version = \"[0-9]*\.[0-9]*\.[0-9]*\"/version = \"${PG_BM25_VERSION}\"/;}" Cargo.toml && \
    cargo pgrx package --features icu --pg-config "/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config"

######################
# pg_analytics
######################

FROM builder as builder-pg_analytics

WORKDIR /tmp/pg_analytics

COPY pg_analytics/ /tmp/pg_analytics
COPY shared/ /tmp/shared

# Use the build argument to update the version in Cargo.toml
# Note: We require Rust nightly to build pg_analytics with SIMD support
RUN rustup update nightly && \
    rustup override set nightly && \
    cargo install --locked cargo-pgrx --version 0.11.1 --force && \
    sed -i "/\[package\]/,/\[/{s/version = \"[0-9]*\.[0-9]*\.[0-9]*\"/version = \"${PG_ANALYTICS_VERSION}\"/;}" Cargo.toml && \
    cargo pgrx package --features simd --pg-config "/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config"

###############################################
# Third Stage: PostgreSQL
###############################################

FROM base as paradedb

ARG TARGETARCH

# Needed for publishing to ArtifactHub
LABEL org.opencontainers.image.created="$BUILD_DATE"
LABEL org.opencontainers.image.description="PostgreSQL for Search"
LABEL io.artifacthub.package.readme-url="https://github.com/paradedb/paradedb/blob/main/README.md"

# Install pgvector
RUN apt-fast update && apt-fast install -y --no-install-recommends \
    postgresql-16-pgvector \
    && rm -rf /var/lib/apt/lists/*

# Copy the ParadeDB pgrx extensions from their builder stages
# We order them by how unlikely they are to be modified, to reduce layer rebuilds
COPY --from=builder-pg_bm25 /tmp/pg_bm25/target/release/pg_bm25-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/* /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder-pg_bm25 /tmp/pg_bm25/target/release/pg_bm25-pg${PG_VERSION_MAJOR}/usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/* /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/
COPY --from=builder-pg_analytics /tmp/pg_analytics/target/release/pg_analytics-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/* /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder-pg_analytics /tmp/pg_analytics/target/release/pg_analytics-pg${PG_VERSION_MAJOR}/usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/* /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/

# Install the ParadeDB pg_sparse extension directly, since it's not as modular as pgrx extensions
# We set OPTFLAGS="" to disable the -march=native flag, which isn't supported on macOS ARM
WORKDIR /tmp/pg_sparse
COPY pg_sparse/ /tmp/pg_sparse
RUN sed -i "s/default_version = .*/default_version = '${PG_SPARSE_VERSION}'/" svector.control && \
    sed -i "s/^EXTVERSION = .*/EXTVERSION = ${PG_SPARSE_VERSION}/" Makefile && \
    make clean && \
    make USE_PGXS=1 OPTFLAGS="" && \
    make install && \
    rm -rf /tmp/pg_sparse
WORKDIR /

# Copy entrypoint script, which will be handled by the official image initialization scipt
COPY ./scripts/entrypoint.sh /docker-entrypoint-initdb.d/10_paradedb.sh

# Change the uid of postgres to 26
RUN usermod -u 26 postgres \
    && chown -R 26:999 /var/lib/postgresql \
    && chown -R 26:999 /var/run/postgresql \
    && chmod -R 700 /var/lib/postgresql

USER 26
