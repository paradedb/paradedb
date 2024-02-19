ARG PG_VERSION_MAJOR=16

###############################################
# First Stage: Builder
###############################################

# TODO: Check that Debian 11/bullseye is the same as Ubuntu 22.04, which is what we test on

# We build the extensions on top of the official PostgreSQL image, since
# the Bitnami image doesn't have root access and necessary build tools
FROM postgres:${PG_VERSION_MAJOR}-bullseye as builder

ARG PG_VERSION_MAJOR=16
ARG RUST_VERSION=1.76.0
ARG PGRX_VERSION=0.11.2

# Declare buildtime environment variables
ENV PG_VERSION_MAJOR=${PG_VERSION_MAJOR} \
    RUST_VERSION=${RUST_VERSION} \
    PGRX_VERSION=${PGRX_VERSION}

# TODO: Do I need this?
SHELL ["/bin/bash", "-o", "pipefail", "-c", "-e"]

# TODO: Can we remove some of these?
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    clang \
    software-properties-common \
    ca-certificates \
    uuid-runtime \
    libssl-dev \
    curl \
    wget \
    git \
    cmake \
    pkg-config \
    liblz4-dev \
    libssl-dev \
    libcurl4-openssl-dev \
    libopenblas-dev \
    python3-dev \
    python3-pip \
    gnupg \
    gcc \
    make \
    postgresql-server-dev-all \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    && /root/.cargo/bin/rustup default "${RUST_VERSION}"

ENV PATH="/root/.cargo/bin:$PATH" \
    PGX_HOME=/usr/lib/postgresql/${PG_VERSION_MAJOR}

RUN cargo install cargo-get && \
    cargo install --locked cargo-pgrx --version "${PGRX_VERSION}" && \
    cargo pgrx init "--pg${PG_VERSION_MAJOR}=/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config"

# Configure package maintainer (needed by debmake)
ENV DEBEMAIL="support@paradedb.com"
ENV DEBFULLNAME="ParadeDB"

######################
# pg_bm25
######################

FROM builder as builder-pg_bm25

SHELL ["/bin/bash", "-o", "pipefail", "-c", "-e"]

COPY pg_bm25/ /tmp/pg_bm25
COPY shared/ /tmp/shared

# Build the extension
WORKDIR /tmp/pg_bm25
RUN cargo pgrx package --features icu --pg-config "/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config"

######################
# pg_analytics
######################

FROM builder as builder-pg_analytics

SHELL ["/bin/bash", "-o", "pipefail", "-c", "-e"]

COPY pg_analytics/ /tmp/pg_analytics
COPY shared/ /tmp/shared

# Build the extension
# Note: We require Rust nightly to build pg_analytics with SIMD
WORKDIR /tmp/pg_analytics
RUN rustup update nightly && \
    rustup override set nightly && \
    cargo install --locked cargo-pgrx --version "${PGRX_VERSION}" --force && \
    cargo pgrx package --pg-config "/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config"

######################
# pg_sparse
######################

FROM builder as builder-pg_sparse

SHELL ["/bin/bash", "-o", "pipefail", "-c", "-e"]

COPY pg_sparse/ /tmp/pg_sparse

# Build the extension
WORKDIR /tmp/pg_sparse
RUN echo "trusted = true" >> svector.control && \
    make clean -j && \
    make USE_PGXS=1 OPTFLAGS="" -j

######################
# pgvector
######################

FROM builder as builder-pgvector

SHELL ["/bin/bash", "-o", "pipefail", "-c", "-e"]

# Build the extension
RUN <<EOF
cd /tmp
git clone --branch v0.6.0 https://github.com/pgvector/pgvector.git
export PG_CFLAGS="-Wall -Wextra -Werror -Wno-unused-parameter -Wno-sign-compare"
cd /tmp/pgvector
echo "trusted = true" >> vector.control
make clean -j
make USE_PGXS=1 OPTFLAGS="" -j
EOF

###############################################
# Second Stage: PostgreSQL
###############################################

# As of writing, Debian 11 (bullseye) is the latest version supported by Bitnami
FROM bitnami/postgresql:${PG_VERSION_MAJOR}-debian-11 as paradedb

ARG PG_VERSION_MAJOR=16
ARG POSTHOG_API_KEY
ARG POSTHOG_HOST
ARG COMMIT_SHA
ARG TELEMETRY

# Declare runtime environment variables
ENV PG_VERSION_MAJOR=${PG_VERSION_MAJOR} \
    POSTHOG_API_KEY=${POSTHOG_API_KEY} \
    POSTHOG_HOST=${POSTHOG_HOST} \
    COMMIT_SHA=${COMMIT_SHA} \
    TELEMETRY=${TELEMETRY}

# Copy the ParadeDB extensions from their builder stages
COPY --from=builder-pg_sparse /tmp/pg_sparse/*.so /opt/bitnami/postgresql/lib/
COPY --from=builder-pg_sparse /tmp/pg_sparse/*.control /opt/bitnami/postgresql/share/extension/
COPY --from=builder-pg_sparse /tmp/pg_sparse/sql/*.sql /opt/bitnami/postgresql/share/extension/
COPY --from=builder-pg_bm25 /tmp/pg_bm25/target/release/pg_bm25-pg${PG_VERSION_MAJOR}/usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/* /opt/bitnami/postgresql/lib/
COPY --from=builder-pg_bm25 /tmp/pg_bm25/target/release/pg_bm25-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/* /opt/bitnami/postgresql/share/extension/
COPY --from=builder-pg_analytics /tmp/pg_analytics/target/release/pg_analytics-pg${PG_VERSION_MAJOR}/usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/* /opt/bitnami/postgresql/lib/
COPY --from=builder-pg_analytics /tmp/pg_analytics/target/release/pg_analytics-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/* /opt/bitnami/postgresql/share/extension/

# Copy third-party extensions from their builder stages
COPY --from=builder-pgvector /tmp/pgvector/*.so /opt/bitnami/postgresql/lib/
COPY --from=builder-pgvector /tmp/pgvector/*.control /opt/bitnami/postgresql/share/extension/
COPY --from=builder-pgvector /tmp/pgvector/sql/*.sql /opt/bitnami/postgresql/share/extension/




# TODOs:
# Handle the shared_preload_libraries
# Handle PostHog telemetry
# Handle permission for installing/creating extensions (maybe if automatic, no need to give extra permissions)
# Make pg_spare install
# Make pg_bm25 install
# Make pg_analytics install
# Make them all get installed automatically


# COPY ./docker/paradedb.sh /docker-entrypoint-preinitdb.d/10_paradedb.sh
# COPY ./docker/01_create_extensions.sql /docker-entrypoint-initdb.d/
# COPY ./docker/create_extension.sql /docker-entrypoint-initdb.d/create_extension.sql
