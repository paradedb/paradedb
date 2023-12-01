ARG PG_VERSION_MAJOR

###############################################
# First Stage: Base
###############################################

FROM postgres:${PG_VERSION_MAJOR}-bookworm as base

# Declare all the build arguments and set them to
# environment variables for use in build and runtime
ARG PG_VERSION_MAJOR
ARG PG_BM25_VERSION
ARG PG_SEARCH_VERSION
ARG PG_SPARSE_VERSION
ARG PGVECTOR_VERSION
ARG PG_CRON_VERSION
ARG PG_NET_VERSION
ARG PG_IVM_VERSION
ARG PG_GRAPHQL_VERSION
ARG PG_HASHIDS_VERSION
ARG PG_JSONSCHEMA_VERSION
ARG PG_REPACK_VERSION
ARG PG_STAT_MONITOR_VERSION
ARG PG_HINT_PLAN_VERSION
ARG PG_ROARINGBITMAP_VERSION
ARG PGFACETING_VERSION
ARG PGTAP_VERSION
ARG PGAUDIT_VERSION
ARG POSTGIS_VERSION
ARG PGROUTING_VERSION
ARG PGSQL_HTTP_VERSION
ARG HYPOPG_VERSION
ARG RUM_VERSION
ARG AGE_VERSION
ARG CITUS_VERSION
ARG PGSODIUM_VERSION
ARG PGFINCORE_VERSION
ARG PG_PARTMAN_VERSION
ARG PG_JOBMON_VERSION
ARG PG_AUTO_FAILOVER_VERSION
ARG PG_SHOW_PLANS_VERSION
ARG SQLITE_FDW_VERSION
ARG PGDDL_VERSION
ARG MYSQL_FDW_VERSION
ARG POSTHOG_API_KEY
ARG POSTHOG_HOST
ARG COMMIT_SHA
ARG TELEMETRY
ARG BUILD_DATE

ENV PG_VERSION_MAJOR=${PG_VERSION_MAJOR} \
    PG_BM25_VERSION=${PG_BM25_VERSION} \
    PG_SEARCH_VERSION=${PG_SEARCH_VERSION} \
    PG_SPARSE_VERSION=${PG_SPARSE_VERSION} \
    PGVECTOR_VERSION=${PGVECTOR_VERSION} \
    PG_CRON_VERSION=${PG_CRON_VERSION} \
    PG_NET_VERSION=${PG_NET_VERSION} \
    PG_IVM_VERSION=${PG_IVM_VERSION} \
    PG_GRAPHQL_VERSION=${PG_GRAPHQL_VERSION} \
    PG_HASHIDS_VERSION=${PG_HASHIDS_VERSION} \
    PG_JSONSCHEMA_VERSION=${PG_JSONSCHEMA_VERSION} \
    PG_REPACK_VERSION=${PG_REPACK_VERSION} \
    PG_STAT_MONITOR_VERSION=${PG_STAT_MONITOR_VERSION} \
    PG_HINT_PLAN_VERSION=${PG_HINT_PLAN_VERSION} \
    PG_ROARINGBITMAP_VERSION=${PG_ROARINGBITMAP_VERSION} \
    PGFACETING_VERSION=${PGFACETING_VERSION} \
    PGTAP_VERSION=${PGTAP_VERSION} \
    PGAUDIT_VERSION=${PGAUDIT_VERSION} \
    POSTGIS_VERSION=${POSTGIS_VERSION} \
    PGROUTING_VERSION=${PGROUTING_VERSION} \
    PGSQL_HTTP_VERSION=${PGSQL_HTTP_VERSION} \
    HYPOPG_VERSION=${HYPOPG_VERSION} \
    RUM_VERSION=${RUM_VERSION} \
    AGE_VERSION=${AGE_VERSION} \
    CITUS_VERSION=${CITUS_VERSION} \
    PGSODIUM_VERSION=${PGSODIUM_VERSION} \
    PGFINCORE_VERSION=${PGFINCORE_VERSION} \
    PG_PARTMAN_VERSION=${PG_PARTMAN_VERSION} \
    PG_JOBMON_VERSION=${PG_JOBMON_VERSION} \
    PG_AUTO_FAILOVER_VERSION=${PG_AUTO_FAILOVER_VERSION} \
    PG_SHOW_PLANS_VERSION=${PG_SHOW_PLANS_VERSION} \
    SQLITE_FDW_VERSION=${SQLITE_FDW_VERSION} \
    PGDDL_VERSION=${PGDDL_VERSION} \
    MYSQL_FDW_VERSION=${MYSQL_FDW_VERSION} \
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
    cargo pgrx package --pg-config "/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config"

######################
# pg_search
######################

FROM builder as builder-pg_search

WORKDIR /tmp/pg_search

COPY pg_search/ /tmp/pg_search
COPY shared/ /tmp/shared

# Use the build argument to update the version in Cargo.toml
RUN sed -i "/\[package\]/,/\[/{s/version = \"[0-9]*\.[0-9]*\.[0-9]*\"/version = \"${PG_SEARCH_VERSION}\"/;}" Cargo.toml && \
    cargo pgrx package --pg-config "/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config"

###############################################
# Third Stage: PostgreSQL
###############################################

FROM base as paradedb

ARG TARGETARCH

# Needed for publishing to ArtifactHub
LABEL org.opencontainers.image.created="$BUILD_DATE"
LABEL org.opencontainers.image.description="PostgreSQL for Search"
LABEL io.artifacthub.package.readme-url="https://github.com/paradedb/paradedb/blob/main/README.md"

# Install barman (for backups, and postgis runtime dependencies
RUN apt-fast update && apt-fast install -y --no-install-recommends \
    # postgis
    libgeos-c1v5 \
    libproj-dev \
    libprotobuf-c1 \
    libc++1 \
    libc++abi1 \
    # barman
    barman \
    barman-cli-cloud \
    && rm -rf /var/lib/apt/lists/* /tmp/* && \
    # Symlink libproj.so.22 to libproj.so.25, since Postgis requires an older version, and
    # copy relevant library for Crunchy operator
    # Only need to copy on x84_64, since the lib is already in the right place on arm64
    if [ "$TARGETARCH" = "amd64" ]; then \
        mkdir -p /usr/lib64 && \
        cp /usr/lib/x86_64-linux-gnu/libnss_wrapper.so /usr/lib64/; \
        ln -s /usr/lib/x86_64-linux-gnu/libproj.so.25 /usr/lib/x86_64-linux-gnu/libproj.so.22; \
    else \
        ln -s /usr/lib/aarch64-linux-gnu/libproj.so.25 /usr/lib/aarch64-linux-gnu/libproj.so.22; \
    fi

# Download & install the non-ParadeDB extensions from their builder stage
RUN curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pgvector-v${PGVECTOR_VERSION}-$TARGETARCH/pgvector-v${PGVECTOR_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-ubuntu2204.deb" -o /tmp/pgvector.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pgaudit-v${PGAUDIT_VERSION}-$TARGETARCH/pgaudit-v${PGAUDIT_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-ubuntu2204.deb" -o /tmp/pgaudit.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pg_cron-v${PG_CRON_VERSION}-$TARGETARCH/pg_cron-v${PG_CRON_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-ubuntu2204.deb" -o /tmp/pg_cron.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pg_ivm-v${PG_IVM_VERSION}-$TARGETARCH/pg_ivm-v${PG_IVM_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-ubuntu2204.deb" -o /tmp/pg_ivm.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pg_hashids-v${PG_HASHIDS_VERSION}-$TARGETARCH/pg_hashids-v${PG_HASHIDS_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-ubuntu2204.deb" -o /tmp/pg_hashids.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pg_repack-v${PG_REPACK_VERSION}-$TARGETARCH/pg_repack-v${PG_REPACK_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-ubuntu2204.deb" -o /tmp/pg_repack.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pg_stat_monitor-v${PG_STAT_MONITOR_VERSION}-$TARGETARCH/pg_stat_monitor-v${PG_STAT_MONITOR_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-ubuntu2204.deb" -o /tmp/pg_stat_monitor.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pg_hint_plan-v${PG_HINT_PLAN_VERSION}-$TARGETARCH/pg_hint_plan-v${PG_HINT_PLAN_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-ubuntu2204.deb" -o /tmp/pg_hint_plan.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pg_roaringbitmap-v${PG_ROARINGBITMAP_VERSION}-$TARGETARCH/pg_roaringbitmap-v${PG_ROARINGBITMAP_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-ubuntu2204.deb" -o /tmp/pg_roaringbitmap.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pgfaceting-v${PGFACETING_VERSION}-$TARGETARCH/pgfaceting-v${PGFACETING_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-ubuntu2204.deb" -o /tmp/pgfaceting.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pgtap-v${PGTAP_VERSION}-$TARGETARCH/pgtap-v${PGTAP_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-ubuntu2204.deb" -o /tmp/pgtap.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/postgis-v${POSTGIS_VERSION}-$TARGETARCH/postgis-v${POSTGIS_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-ubuntu2204.deb" -o /tmp/postgis.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pgrouting-v${PGROUTING_VERSION}-$TARGETARCH/pgrouting-v${PGROUTING_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-ubuntu2204.deb" -o /tmp/pgrouting.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pgsql-http-v${PGSQL_HTTP_VERSION}-$TARGETARCH/pgsql-http-v${PGSQL_HTTP_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-ubuntu2204.deb" -o /tmp/pgsql-http.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/hypopg-v${HYPOPG_VERSION}-$TARGETARCH/hypopg-v${HYPOPG_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-ubuntu2204.deb" -o /tmp/hypopg.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/rum-v${RUM_VERSION}-$TARGETARCH/rum-v${RUM_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-ubuntu2204.deb" -o /tmp/rum.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/age-v${AGE_VERSION}-$TARGETARCH/age-v${AGE_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-ubuntu2204.deb" -o /tmp/age.deb && \
    



    curl -L "https://github.com/supabase/pg_net/releases/download/v${PG_NET_VERSION}/pg_net-v${PG_NET_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-ubuntu2204.deb" -o /tmp/pg_net.deb && \
    curl -L "https://github.com/supabase/pg_graphql/releases/download/v${PG_GRAPHQL_VERSION}/pg_graphql-v${PG_GRAPHQL_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-ubuntu2204.deb" -o /tmp/pg_graphql.deb && \
    curl -L "https://github.com/supabase/pg_jsonschema/releases/download/v${PG_JSONSCHEMA_VERSION}/pg_jsonschema-v${PG_JSONSCHEMA_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-ubuntu2204.deb" -o /tmp/pg_jsonschema.deb && \
    apt-fast update && apt-fast install -y --no-install-recommends /tmp/*.deb \
    && rm -rf /var/lib/apt/lists/* /tmp/*

# Copy the ParadeDB pgrx extensions from their builder stages
# We order them by how unlikely they are to be modified, to reduce layer rebuilds
COPY --from=builder-pg_search /tmp/pg_search/target/release/pg_search-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/* /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder-pg_search /tmp/pg_search/target/release/pg_search-pg${PG_VERSION_MAJOR}/usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/* /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/
COPY --from=builder-pg_bm25 /tmp/pg_bm25/target/release/pg_bm25-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/* /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder-pg_bm25 /tmp/pg_bm25/target/release/pg_bm25-pg${PG_VERSION_MAJOR}/usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/* /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/

# Install the ParadeDB pg_sparse extension directly, since it's not as modular as pgrx extensions
WORKDIR /tmp/pg_sparse
COPY pg_sparse/ /tmp/pg_sparse
RUN sed -i "s/default_version = .*/default_version = '${PG_SPARSE_VERSION}'/" svector.control && \
    sed -i "s/^EXTVERSION = .*/EXTVERSION = ${PG_SPARSE_VERSION}/" Makefile && \
    make && \
    make install && \
    rm -rf /tmp/pg_sparse
WORKDIR /

# Copy entrypoint script, which will be handled by the official image
# initialization scipt
COPY ./scripts/entrypoint.sh /docker-entrypoint-initdb.d/10_paradedb.sh

# Change the uid of postgres to 26
RUN usermod -u 26 postgres \
    && chown -R 26:999 /var/lib/postgresql \
    && chown -R 26:999 /var/run/postgresql \
    && chmod -R 700 /var/lib/postgresql

USER 26
