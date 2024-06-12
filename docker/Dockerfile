ARG PG_VERSION_MAJOR=16

###############################################
# First Stage: Builder
###############################################

# We build the extensions on the official PostgreSQL image, since the Bitnami
# image locks root and has few pre-installed packages
# Note: Debian Bookworm = Debian 12
FROM postgres:${PG_VERSION_MAJOR}-bookworm as builder

ARG PG_VERSION_MAJOR=16
ARG RUST_VERSION=nightly-2024-04-21
ARG PGRX_VERSION=0.11.3

# Declare buildtime environment variables
ENV PG_VERSION_MAJOR=${PG_VERSION_MAJOR} \
    RUST_VERSION=${RUST_VERSION} \
    PGRX_VERSION=${PGRX_VERSION}

SHELL ["/bin/bash", "-o", "pipefail", "-c", "-e"]

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    software-properties-common \
    ca-certificates \
    build-essential \
    gnupg \
    curl \
    git \
    make \
    gcc \
    clang \
    pkg-config \
    libopenblas-dev \
    postgresql-server-dev-all \
    && rm -rf /var/lib/apt/lists/*

# Note: DataFusion requires Rust nightly to enable packed_simd for SIMD optimizations. We
# fix Rust nightly because current latest (1.80.0) is causing compilation issues with some dependencies
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain "${RUST_VERSION}" -y

ENV PATH="/root/.cargo/bin:$PATH" \
    PGX_HOME=/usr/lib/postgresql/${PG_VERSION_MAJOR}

RUN cargo install --locked cargo-pgrx --version "${PGRX_VERSION}" && \
    cargo pgrx init "--pg${PG_VERSION_MAJOR}=/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config"

######################
# pg_search
######################

FROM builder as builder-pg_search

ARG POSTHOG_API_KEY
ARG POSTHOG_HOST
ARG COMMIT_SHA
ARG PARADEDB_TELEMETRY

# Declare compile-time environment variables
ENV POSTHOG_API_KEY=${POSTHOG_API_KEY} \
    POSTHOG_HOST=${POSTHOG_HOST} \
    COMMIT_SHA=${COMMIT_SHA} \
    PARADEDB_TELEMETRY=${PARADEDB_TELEMETRY}

COPY pg_search/ /tmp/pg_search
COPY shared/ /tmp/shared
COPY tokenizers/ /tmp/tokenizers

# Build the extension
WORKDIR /tmp/pg_search
RUN cargo pgrx package --features icu --pg-config "/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config"

######################
# pg_lakehouse
######################

FROM builder as builder-pg_lakehouse

ARG POSTHOG_API_KEY
ARG POSTHOG_HOST
ARG COMMIT_SHA
ARG PARADEDB_TELEMETRY

# Declare compile-time environment variables
ENV POSTHOG_API_KEY=${POSTHOG_API_KEY} \
    POSTHOG_HOST=${POSTHOG_HOST} \
    COMMIT_SHA=${COMMIT_SHA} \
    PARADEDB_TELEMETRY=${PARADEDB_TELEMETRY}

COPY pg_lakehouse/ /tmp/pg_lakehouse
COPY shared/ /tmp/shared

# Build the extension
WORKDIR /tmp/pg_lakehouse
RUN cargo pgrx package --pg-config "/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config"

######################
# pgvector
######################

FROM builder as builder-pgvector

# Build the extension
WORKDIR /tmp
RUN git clone --branch v0.7.1 https://github.com/pgvector/pgvector.git
WORKDIR /tmp/pgvector
RUN export PG_CFLAGS="-Wall -Wextra -Werror -Wno-unused-parameter -Wno-sign-compare" && \
    echo "trusted = true" >> vector.control && \
    make clean -j && \
    make USE_PGXS=1 OPTFLAGS="" -j

######################
# pgvectorscale
######################

FROM builder as builder-pgvectorscale

ARG TARGETARCH

# Build the extension
WORKDIR /tmp
RUN git clone --branch 0.2.0 https://github.com/timescale/pgvectorscale.git
WORKDIR /tmp/pgvectorscale/pgvectorscale
RUN if [ "$TARGETARCH" = "amd64" ]; then \
        # Required for pgvectorscale to compile on x86_64/amd64
        RUSTFLAGS="-C target-feature=+avx2,+fma" cargo pgrx package --pg-config "/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config"; \
    else \
        cargo pgrx package --pg-config "/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config"; \
    fi

######################
# pg_cron
######################

FROM builder as builder-pg_cron

# Build the extension
WORKDIR /tmp
RUN git clone --branch v1.6.2 https://github.com/citusdata/pg_cron.git
WORKDIR /tmp/pg_cron
RUN echo "trusted = true" >> pg_cron.control && \
    make clean -j && \
    make USE_PGXS=1 -j

######################
# pg_ivm
######################

FROM builder as builder-pg_ivm

# Build the extension
WORKDIR /tmp
RUN git clone --branch v1.8 https://github.com/sraoss/pg_ivm.git
WORKDIR /tmp/pg_ivm
RUN echo "trusted = true" >> pg_ivm.control && \
    make clean -j && \
    make USE_PGXS=1 -j

###############################################
# Second Stage: PostgreSQL
###############################################

# Note: Debian Bookworm = Debian 12
FROM bitnami/postgresql:${PG_VERSION_MAJOR}-debian-12 as paradedb

ARG PG_VERSION_MAJOR=16

# Declare runtime environment variables
ENV PG_VERSION_MAJOR=${PG_VERSION_MAJOR}

# Copy third-party extensions from their builder stages
COPY --from=builder-pgvector /tmp/pgvector/*.so /opt/bitnami/postgresql/lib/
COPY --from=builder-pgvector /tmp/pgvector/*.control /opt/bitnami/postgresql/share/extension/
COPY --from=builder-pgvector /tmp/pgvector/sql/*.sql /opt/bitnami/postgresql/share/extension/
COPY --from=builder-pgvectorscale /tmp/pgvectorscale/pgvectorscale/target/release/vectorscale-pg${PG_VERSION_MAJOR}/usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/* /opt/bitnami/postgresql/lib/
COPY --from=builder-pgvectorscale /tmp/pgvectorscale/pgvectorscale/target/release/vectorscale-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/* /opt/bitnami/postgresql/share/extension/
COPY --from=builder-pg_cron /tmp/pg_cron/*.so /opt/bitnami/postgresql/lib/
COPY --from=builder-pg_cron /tmp/pg_cron/*.control /opt/bitnami/postgresql/share/extension/
COPY --from=builder-pg_cron /tmp/pg_cron/*.sql /opt/bitnami/postgresql/share/extension/
COPY --from=builder-pg_ivm /tmp/pg_ivm/*.so /opt/bitnami/postgresql/lib/
COPY --from=builder-pg_ivm /tmp/pg_ivm/*.control /opt/bitnami/postgresql/share/extension/
COPY --from=builder-pg_ivm /tmp/pg_ivm/*.sql /opt/bitnami/postgresql/share/extension/

# Copy the ParadeDB extensions from their builder stages
COPY --from=builder-pg_search /tmp/pg_search/target/release/pg_search-pg${PG_VERSION_MAJOR}/usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/* /opt/bitnami/postgresql/lib/
COPY --from=builder-pg_search /tmp/pg_search/target/release/pg_search-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/* /opt/bitnami/postgresql/share/extension/
COPY --from=builder-pg_lakehouse /tmp/pg_lakehouse/target/release/pg_lakehouse-pg${PG_VERSION_MAJOR}/usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/* /opt/bitnami/postgresql/lib/
COPY --from=builder-pg_lakehouse /tmp/pg_lakehouse/target/release/pg_lakehouse-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/* /opt/bitnami/postgresql/share/extension/

# Install runtime dependencies (requires switching to root temporarily)
USER root
RUN install_packages curl uuid-runtime libpq5
USER 1001

# Copy ParadeDB bootstrap script to install extensions, configure postgresql.conf, etc.
COPY ./docker/01_bootstrap.sh /docker-entrypoint-initdb.d/

# Configure shared_preload_libraries
# Note: pgaudit is needed here as it comes pre-packaged in the Bitnami image
ENV POSTGRESQL_SHARED_PRELOAD_LIBRARIES="pgaudit,pg_cron,pg_search,pg_lakehouse"

# We set a default password to enable users to get started quickly, as it is a required
# environment variable for the Bitnami image.
#
# Note: This *will* (and should) get overriden by passing the POSTGRESQL_PASSWORD environment
# varialbe via `docker-compose` or `docker run -e POSTGRESQL_PASSWORD`. All production usage
# should use a unique, secure password and not rely on the default settings.
ENV POSTGRESQL_PASSWORD=postgres
