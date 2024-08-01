ARG PG_VERSION_MAJOR=16

###############################################
# First Stage: Builder
###############################################

# Note: Debian Bookworm = Debian 12
FROM postgres:${PG_VERSION_MAJOR}-bookworm AS builder

ARG PG_VERSION_MAJOR=16
ARG RUST_VERSION=1.79.0
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

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain "${RUST_VERSION}" -y

ENV PATH="/root/.cargo/bin:$PATH" \
    PGX_HOME=/usr/lib/postgresql/${PG_VERSION_MAJOR}

RUN cargo install --locked cargo-pgrx --version "${PGRX_VERSION}" && \
    cargo pgrx init "--pg${PG_VERSION_MAJOR}=/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config"

######################
# pg_search
######################

FROM builder AS builder-pg_search

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
# pg_analytics
######################

FROM builder AS builder-pg_analytics

ARG POSTHOG_API_KEY
ARG POSTHOG_HOST
ARG COMMIT_SHA
ARG PARADEDB_TELEMETRY

# Declare compile-time environment variables
ENV POSTHOG_API_KEY=${POSTHOG_API_KEY} \
    POSTHOG_HOST=${POSTHOG_HOST} \
    COMMIT_SHA=${COMMIT_SHA} \
    PARADEDB_TELEMETRY=${PARADEDB_TELEMETRY}

RUN git clone --brach 0.1.0 https://github.com/paradedb/pg_analytics.git /tmp/pg_analytics/
COPY shared/ /tmp/shared

# Build the extension
WORKDIR /tmp/pg_analytics
RUN cargo pgrx package --pg-config "/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config"

######################
# pgvector
######################

FROM builder AS builder-pgvector

# Build the extension
WORKDIR /tmp
RUN git clone --branch v0.7.4 https://github.com/pgvector/pgvector.git
WORKDIR /tmp/pgvector
RUN export PG_CFLAGS="-Wall -Wextra -Werror -Wno-unused-parameter -Wno-sign-compare" && \
    echo "trusted = true" >> vector.control && \
    make clean -j && \
    make USE_PGXS=1 OPTFLAGS="" -j

######################
# pgvectorscale
######################

FROM builder AS builder-pgvectorscale

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

FROM builder AS builder-pg_cron

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

FROM builder AS builder-pg_ivm

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
FROM postgres:${PG_VERSION_MAJOR}-bookworm AS paradedb

LABEL maintainer="ParadeDB - https://paradedb.com" \
      org.opencontainers.image.description="ParadeDB - Postgres for Search and Analytics" \
      org.opencontainers.image.source="https://github.com/paradedb/paradedb"

ARG PG_VERSION_MAJOR=16

# Declare runtime environment variables
ENV PG_VERSION_MAJOR=${PG_VERSION_MAJOR}

# Copy third-party extensions from their builder stages
COPY --from=builder-pgvector /tmp/pgvector/*.so /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/
COPY --from=builder-pgvector /tmp/pgvector/*.control /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder-pgvector /tmp/pgvector/sql/*.sql /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder-pgvectorscale /tmp/pgvectorscale/pgvectorscale/target/release/vectorscale-pg${PG_VERSION_MAJOR}/usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/* /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/
COPY --from=builder-pgvectorscale /tmp/pgvectorscale/pgvectorscale/target/release/vectorscale-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/* /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder-pg_cron /tmp/pg_cron/*.so /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/
COPY --from=builder-pg_cron /tmp/pg_cron/*.control /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder-pg_cron /tmp/pg_cron/sql/*.sql /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder-pg_ivm /tmp/pg_ivm/*.so /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/
COPY --from=builder-pg_ivm /tmp/pg_ivm/*.control /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder-pg_ivm /tmp/pg_ivm/*.sql /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/

# Copy the ParadeDB extensions from their builder stages
COPY --from=builder-pg_search /tmp/pg_search/target/release/pg_search-pg${PG_VERSION_MAJOR}/usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/* /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/
COPY --from=builder-pg_search /tmp/pg_search/target/release/pg_search-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/* /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder-pg_analytics /tmp/pg_analytics/target/release/pg_analytics-pg${PG_VERSION_MAJOR}/usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/* /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/
COPY --from=builder-pg_analytics /tmp/pg_analytics/target/release/pg_analytics-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/* /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/

# Switch to root for configuring runtime dependencies
USER root

# Required for pg_analytics, pg_cron
RUN mkdir .duckdb/ && chmod 777 .duckdb/ && \
    apt-get install -y --no-install-recommends libpq5 && rm -rf /var/lib/apt/lists/*

# The postgresql.conf.sample file is used as a template for the postgresql.conf file, which
# does not exist until the first time the container is started. By adding our settings to the
# postgresql.conf.sample file, we ensure that our settings are applied onto the postgresql.conf file.
#
# The `postgres` database is the default database that exists in every Postgres installation. The pg_cron
# extension requires a database to store its metadata tables. By using `postgres`, we ensure that it has a
# stable, always-available database for its operations, no matter what other databases are created or deleted.
RUN sed -i "s/^#shared_preload_libraries = ''/shared_preload_libraries = 'pg_search,pg_analytics,pg_cron'/" /usr/share/postgresql/postgresql.conf.sample && \
    echo "cron.database_name = 'postgres'" >> /usr/share/postgresql/postgresql.conf.sample

# Switch back to the postgres user
USER postgres

# Copy ParadeDB scripts to install extensions, configure postgresql.conf, update extensions, etc.
COPY ./docker/bootstrap.sh /docker-entrypoint-initdb.d/10_bootstrap_paradedb.sh
COPY ./docker/update_pg_search.sh /usr/local/bin/
COPY ./docker/update_pg_analytics.sh /usr/local/bin/
