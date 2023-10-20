ARG PG_VERSION_MAJOR

###############################################
# First Stage: Base
###############################################

FROM postgres:${PG_VERSION_MAJOR}-bullseye as base

# Declare all the build arguments and set them to
# environment variables for use in build and runtime
ARG PG_VERSION_MAJOR
ARG PG_BM25_VERSION
ARG PG_SEARCH_VERSION
ARG PGNODEMX_VERSION
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
ARG PGML_VERSION
ARG PGTAP_VERSION
ARG PGVECTOR_VERSION
ARG PGAUDIT_VERSION
ARG POSTGIS_VERSION
ARG PGROUTING_VERSION
ARG PGSQL_HTTP_VERSION
ARG HYPOPG_VERSION
ARG RUM_VERSION
ARG AGE_VERSION

ENV PG_VERSION_MAJOR=${PG_VERSION_MAJOR} \
    PG_BM25_VERSION=${PG_BM25_VERSION} \
    PG_SEARCH_VERSION=${PG_SEARCH_VERSION} \
    PGNODEMX_VERSION=${PGNODEMX_VERSION} \
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
    PGML_VERSION=${PGML_VERSION} \
    PGTAP_VERSION=${PGTAP_VERSION} \
    PGVECTOR_VERSION=${PGVECTOR_VERSION} \
    PGAUDIT_VERSION=${PGAUDIT_VERSION} \
    POSTGIS_VERSION=${POSTGIS_VERSION} \
    PGROUTING_VERSION=${PGROUTING_VERSION} \
    PGSQL_HTTP_VERSION=${PGSQL_HTTP_VERSION} \
    HYPOPG_VERSION=${HYPOPG_VERSION} \
    RUM_VERSION=${RUM_VERSION} \
    AGE_VERSION=${AGE_VERSION} \
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
    software-properties-common \
    ca-certificates \
    python3.10 \
    libssl-dev \
    python3-dev \
    python3-pip \
    libopenblas-dev \
    && rm -rf /var/lib/apt/lists/*

# Install apt-fast
RUN /bin/bash -c "$(curl -sL https://git.io/vokNn)"

# Add PostgreSQL's third party repository to get the latest versions
RUN curl -s https://www.postgresql.org/media/keys/ACCC4CF8.asc | apt-key add -
RUN echo "deb http://apt.postgresql.org/pub/repos/apt/ $(lsb_release -cs)-pgdg main" | tee /etc/apt/sources.list.d/pgdg.list

RUN apt-get update && apt-fast install -y --no-install-recommends \
    postgresql-server-dev-${PG_VERSION_MAJOR} \
    && rm -rf /var/lib/apt/lists/*

###############################################
# Second Stage: Builder
###############################################

FROM base as builder

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

# Install Rust and other build dependencies
RUN apt-get update && apt-fast install -y --no-install-recommends \
    build-essential \
    checkinstall \
    clang \
    git \
    cmake \
    pkg-config \
    liblz4-dev \
    libcurl4-openssl-dev \
    && rm -rf /var/lib/apt/lists/* \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

ENV PATH="/root/.cargo/bin:$PATH" \
    PGX_HOME=/usr/lib/postgresql/${PG_VERSION_MAJOR}

RUN cargo install cargo-pgrx --version 0.9.8 && \
    cargo pgrx init "--pg${PG_VERSION_MAJOR}=/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config"

######################
# pg_bm25
######################

FROM builder as builder-pg_bm25

WORKDIR /tmp/pg_bm25

COPY pg_bm25/ /tmp/pg_bm25

# Use the build argument to update the version in Cargo.toml
RUN sed -i "s/version = \"[0-9]*\.[0-9]*\.[0-9]*\"/version = \"${PG_BM25_VERSION}\"/" Cargo.toml && \
    cargo pgrx package --pg-config "/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config"

####################
# pgml
####################

FROM builder as builder-pgml

RUN apt-get update && apt-fast install -y --no-install-recommends \
    libz-dev \
    tzdata \
    libpq-dev \
    libclang-dev \
    bison \
    flex \
    libreadline-dev \
    && rm -rf /var/lib/apt/lists/*

RUN git clone --branch "v${PGML_VERSION}" "https://github.com/paradedb/postgresml" /tmp/postgresml

WORKDIR /tmp/postgresml/pgml-extension

RUN git submodule update --init --recursive && \
    git fetch --tags && \
    git fetch --depth 1 origin tag "v${PGML_VERSION}" && \
    git checkout "v${PGML_VERSION}"

# Update pgml schema to paradedb
RUN sed -i "s/\(schema = \).*/\1'paradedb'/" pgml.control && \
    find . -type f -exec sed -i 's/pgml\./paradedb\./g' {} + && \
    cargo pgrx package --pg-config "/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config"

######################
# pg_search
######################

FROM builder as builder-pg_search

WORKDIR /tmp/pg_search

COPY pg_search/ /tmp/pg_search

# Use the build argument to update the version in Cargo.toml
RUN sed -i "s/version = \"[0-9]*\.[0-9]*\.[0-9]*\"/version = \"${PG_SEARCH_VERSION}\"/" Cargo.toml && \
    cargo pgrx package --pg-config "/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config"

###############################################
# Third Stage: PostgreSQL
###############################################

FROM base as paradedb

ARG TARGETARCH

WORKDIR /usr/src/app

COPY --from=builder-pgml /tmp/postgresml/pgml-extension/requirements.txt /usr/src/app/

# Install postgis, pgml, patroni & Crunchy operator runtime dependencies
RUN apt-get update && apt-fast install -y --no-install-recommends \
    # postgis
    libgeos-c1v5 \
    libproj-dev \
    libprotobuf-c1 \
    # Crunchy operator
    libnss-wrapper \
    pgbackrest \
    && rm -rf /var/lib/apt/lists/* /tmp/* && \
    # pgml
    pip3 install --no-cache-dir -r requirements.txt && rm -rf requirements.txt && \
    # patroni
    pip3 install --no-cache-dir patroni[etcd3] psycopg[binary]>=3.0.0 && \
    # Crunchy operator
    # Only need to copy on x84_64, since the lib is already in the right place on arm64
    if [ "$TARGETARCH" = "amd64" ]; then \
        mkdir -p /usr/lib64 && \
        cp /usr/lib/x86_64-linux-gnu/libnss_wrapper.so /usr/lib64/; \
    fi

# Download & install the non-ParadeDB extensions from their builder stage
RUN curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pgvector-v${PGVECTOR_VERSION}/pgvector-v${PGVECTOR_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pgvector.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pgaudit-${PGAUDIT_VERSION}/pgaudit-${PGAUDIT_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pgaudit.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pgnodemx-${PGNODEMX_VERSION}/pgnodemx-${PGNODEMX_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pgnodemx.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pg_cron-v${PG_CRON_VERSION}/pg_cron-v${PG_CRON_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pg_cron.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pg_ivm-v${PG_IVM_VERSION}/pg_ivm-v${PG_IVM_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pg_ivm.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pg_hashids-v${PG_HASHIDS_VERSION}/pg_hashids-v${PG_HASHIDS_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pg_hashids.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pg_repack-${PG_REPACK_VERSION}/pg_repack-${PG_REPACK_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pg_repack.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pg_stat_monitor-${PG_STAT_MONITOR_VERSION}/pg_stat_monitor-${PG_STAT_MONITOR_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pg_stat_monitor.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pg_hint_plan-${PG_HINT_PLAN_VERSION}/pg_hint_plan-${PG_HINT_PLAN_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pg_hint_plan.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pg_roaringbitmap-v${PG_ROARINGBITMAP_VERSION}/pg_roaringbitmap-v${PG_ROARINGBITMAP_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pg_roaringbitmap.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pgfaceting-v${PGFACETING_VERSION}/pgfaceting-v${PGFACETING_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pgfaceting.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pgtap-v${PGTAP_VERSION}/pgtap-v${PGTAP_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pgtap.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/postgis-v${POSTGIS_VERSION}/postgis-v${POSTGIS_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/postgis.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pgrouting-v${PGROUTING_VERSION}/pgrouting-v${PGROUTING_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pgrouting.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pgsql-http-v${PGSQL_HTTP_VERSION}/pgsql-http-v${PGSQL_HTTP_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pgsql-http.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/hypopg-${HYPOPG_VERSION}/hypopg-${HYPOPG_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/hypopg.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/rum-${RUM_VERSION}/rum-${RUM_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/rum.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/age-v${AGE_VERSION}/age-v${AGE_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/age.deb \
    curl -L "https://github.com/supabase/pg_net/releases/download/v${PG_NET_VERSION}/pg_net-v${PG_NET_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pg_net.deb && \
    curl -L "https://github.com/supabase/pg_graphql/releases/download/v${PG_GRAPHQL_VERSION}/pg_graphql-v${PG_GRAPHQL_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pg_graphql.deb && \
    curl -L "https://github.com/supabase/pg_jsonschema/releases/download/v${PG_JSONSCHEMA_VERSION}/pg_jsonschema-v${PG_JSONSCHEMA_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pg_jsonschema.deb && \
    apt-get update && apt-fast install -y --no-install-recommends /tmp/*.deb \
    && rm -rf /var/lib/apt/lists/* /tmp/*

# Copy the pg_bm25, pgml & pg_search extensions from their builder stages
# We order them by how unlikely they are to be modified, to reduce layer rebuilds
COPY --from=builder-pgml /tmp/postgresml/pgml-extension/target/release/pgml-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/* /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder-pgml /tmp/postgresml/pgml-extension/target/release/pgml-pg${PG_VERSION_MAJOR}/usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/* /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/
COPY --from=builder-pg_search /tmp/pg_search/target/release/pg_search-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/* /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder-pg_search /tmp/pg_search/target/release/pg_search-pg${PG_VERSION_MAJOR}/usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/* /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/
COPY --from=builder-pg_bm25 /tmp/pg_bm25/target/release/pg_bm25-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/* /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder-pg_bm25 /tmp/pg_bm25/target/release/pg_bm25-pg${PG_VERSION_MAJOR}/usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/* /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/

# Copy entrypoint script, which will be handled by the official image
# initialization scipt
COPY ./scripts/entrypoint.sh /docker-entrypoint-initdb.d/10_paradedb.sh

# Postgres user for Crunchy operator
USER 999
