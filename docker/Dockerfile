ARG PG_VERSION_MAJOR=16

###############################################
# First Stage: Base
###############################################

FROM postgres:${PG_VERSION_MAJOR}-bookworm as base

# Declare all the build arguments and set them to
# environment variables for use in build and runtime
ARG PG_VERSION_MAJOR
ARG POSTHOG_API_KEY
ARG POSTHOG_HOST
ARG COMMIT_SHA
ARG TELEMETRY
ARG BUILD_DATE

ENV PG_VERSION_MAJOR=${PG_VERSION_MAJOR} \
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
    software-properties-common \
    ca-certificates \
    uuid-runtime \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*


###############################################
# Second Stage: Builder
###############################################

FROM base as builder

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

# Install Rust (specific version) and other build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    checkinstall \
    clang \
    git \
    cmake \
    pkg-config \
    liblz4-dev \
    libcurl4-openssl-dev \
    libopenblas-dev \
    python3-dev \
    python3-pip \
    curl \
    wget \
    sudo \
    gnupg \
    gcc \
    make

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    && /root/.cargo/bin/rustup default 1.73.0

# Add PostgreSQL's third party repository to get the latest versions
RUN curl -s https://www.postgresql.org/media/keys/ACCC4CF8.asc | apt-key add -
RUN echo "deb http://apt.postgresql.org/pub/repos/apt/ $(lsb_release -cs)-pgdg main" | tee /etc/apt/sources.list.d/pgdg.list

RUN apt-get update && apt-get install -y postgresql-server-dev-${PG_VERSION_MAJOR} \
    && rm -rf /var/lib/apt/lists/*

ENV PATH="/root/.cargo/bin:$PATH" \
    PGX_HOME=/usr/lib/postgresql/${PG_VERSION_MAJOR}

RUN cargo install cargo-get
RUN cargo install --locked cargo-pgrx --version 0.11.2 && \
    cargo pgrx init "--pg${PG_VERSION_MAJOR}=/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config"

######################
# pg_bm25
######################

FROM builder as builder-pg_bm25

COPY pg_bm25/ /tmp/pg_bm25
COPY shared/ /tmp/shared

WORKDIR /tmp/pg_bm25

# Use the build argument to update the version in Cargo.toml
RUN cargo pgrx package --features icu --pg-config "/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config"

RUN checkinstall --default -D --nodoc --install=no --fstrans=no --backup=no --pakdir=/tmp \
   --exclude=/root/.cargo \
   --pkgname $(cargo get package.name) \
   --pkgversion $(cargo get package.version) \
   --maintainer "ParadeDB <support@paradedb.com>" \
   -- cargo pgrx install --release --features icu --pg-config "/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config"

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
    cargo install --locked cargo-pgrx --version 0.11.2 --force && \
    cargo pgrx init "--pg${PG_VERSION_MAJOR}=/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config"

# Use the build argument to update the version in Cargo.toml
RUN cargo pgrx package --features simd --pg-config "/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config"


RUN checkinstall --default -D --nodoc --install=no --fstrans=no --backup=no --pakdir=/tmp \
    --exclude=/root/.cargo \
    --pkgname $(cargo get package.name) \
    --pkgversion $(cargo get package.version) \
    --maintainer "ParadeDB <support@paradedb.com>" \
    -- cargo pgrx install --release --features simd --pg-config "/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config"

######################
# pg_sparse
######################
FROM builder as builder-pg_sparse

WORKDIR /tmp/pg_sparse
COPY pg_sparse/ /tmp/pg_sparse

RUN make clean && \
    make USE_PGXS=1 OPTFLAGS="" -j && \
    checkinstall --default -D --nodoc --install=no --fstrans=no --backup=no --pakdir=/tmp \
    --pkgname pg-sparse \
    --pkgversion $(awk -F "= '|\''" '/default_version/ {print $2}' svector.control) \
    --maintainer "ParadeDB <support@paradedb.com>" \
    -- make USE_PGXS=1 install -j

######################
# pgvector
######################

FROM builder as builder-pgvector

WORKDIR /tmp/extensions-deb
RUN apt-get download postgresql-${PG_VERSION_MAJOR}-pgvector

###############################################
# Third Stage: PostgreSQL
###############################################

FROM base as paradedb

ARG TARGETARCH

# Needed for publishing to ArtifactHub
LABEL org.opencontainers.image.created="$BUILD_DATE"
LABEL org.opencontainers.image.description="PostgreSQL for Search"
LABEL io.artifacthub.package.readme-url="https://github.com/paradedb/paradedb/blob/main/README.md"

# Copy the ParadeDB extensions from their builder stages
COPY --from=builder-pg_sparse /tmp/pg-sparse*.deb /tmp/extensions-deb/
COPY --from=builder-pg_bm25 /tmp/pg-bm25*.deb /tmp/extensions-deb/
COPY --from=builder-pgvector /tmp/pgvector*.deb /tmp/extensions-deb/
COPY --from=builder-pg_analytics /tmp/pg-analytics*.deb /tmp/extensions-deb/

RUN dpkg -i /tmp/extensions-deb/*.deb \
    && rm -rf /tmp/extensions-deb

# Copy entrypoint script, which will be handled by the official image initialization scipt
COPY ./docker/entrypoint.sh /docker-entrypoint-initdb.d/10_paradedb.sh

# Change the uid of postgres to 26
RUN usermod -u 26 postgres \
    && chown -R 26:999 /var/lib/postgresql \
    && chown -R 26:999 /var/run/postgresql \
    && chmod -R 700 /var/lib/postgresql

USER 26

FROM builder-pgvector as bb