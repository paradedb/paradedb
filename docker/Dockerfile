###############################################
# First Stage: Base
###############################################

FROM ubuntu:22.04 as base

ARG PG_VERSION_MAJOR

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

ENV DEBIAN_FRONTEND=noninteractive
ENV TZ=UTC

# Install common dependencies to builder and runtime
# TODO: Do we need all these 4 python packages? It's for pgml
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    wget \
    sudo \
    gnupg \
    software-properties-common \
    ca-certificates \
    python3.10 \
    python3-pip \
    libpython3.10-dev \
    python3.10-dev \
    libopenblas-dev \
    && rm -rf /var/lib/apt/lists/*

# Install apt-fast
RUN /bin/bash -c "$(curl -sL https://git.io/vokNn)"

# Add PostgreSQL's third party repository to get the latest versions
RUN curl -s https://www.postgresql.org/media/keys/ACCC4CF8.asc | apt-key add -
RUN echo "deb http://apt.postgresql.org/pub/repos/apt/ $(lsb_release -cs)-pgdg main" | tee /etc/apt/sources.list.d/pgdg.list

RUN apt-get update && apt-fast install -y --no-install-recommends \
    postgresql-${PG_VERSION_MAJOR} \
    postgresql-server-dev-${PG_VERSION_MAJOR} \
    && rm -rf /var/lib/apt/lists/*

###############################################
# Second Stage: Builder
###############################################

FROM base as builder

ARG PG_VERSION_MAJOR

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

# Install build dependencies
RUN apt-get update && apt-fast install -y --no-install-recommends \
    build-essential \
    checkinstall \
    clang \
    git \
    cmake \
    pkg-config \
    libcurl4-openssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:$PATH"

# Set the PGX_HOME environment variable
ENV PGX_HOME=/usr/lib/postgresql/${PG_VERSION_MAJOR}

# Install cargo-pgrx
RUN cargo install cargo-pgrx

# ####################
# pg_bm25
# ####################

FROM builder as builder-pg_bm25

ARG PG_VERSION_MAJOR
ARG PG_BM25_VERSION

# Set the working directory
WORKDIR /usr/src/app

# Copy the pg_bm25 directory contents into the container at /usr/src/app
COPY pg_bm25/ /usr/src/app

# Use the build argument to update the version in Cargo.toml
RUN sed -i "s/version = \"[0-9]*\.[0-9]*\.[0-9]*\"/version = \"${PG_BM25_VERSION}\"/" Cargo.toml

# Initialize pgrx
RUN cargo pgrx init --pg${PG_VERSION_MAJOR}=/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config && \
    cargo pgrx package --pg-config /usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config

# ####################
# other extensions
######################

FROM builder as builder-generic

ARG PG_VERSION_MAJOR
ARG PG_CRON_VERSION
ARG PG_NET_VERSION
ARG PG_IVM_VERSION
ARG PG_GRAPHQL_VERSION
ARG PG_HASHIDS_VERSION
ARG PG_JSONSCHEMA_VERSION
ARG PG_REPACK_VERSION
ARG PG_STAT_MONITOR_VERSION
ARG PG_HINT_PLAN_VERSION
ARG PGTAP_VERSION
ARG PGVECTOR_VERSION
ARG POSTGIS_VERSION
ARG PGAUDIT_VERSION
ARG PGROUTING_VERSION
ARG PGSQL_HTTP_VERSION
ARG HYPOPG_VERSION
ARG RUM_VERSION
ARG TARGETARCH

# TODOs: Left to install
# ARG PG_STAT_STATEMENTS_VERSION
# ARG PGROONGA_VERSION
# ARG PLPGSQL_CHECK_VERSION
# ARG PLV8_VERSION
# ARG TIMESCALEDB_VERSION
# ARG PG_WAL2JSON_VERSION
# ARG PGSODIUM_VERSION
# ARG SFCGAL_VERSION
# ARG PG_SAFEUPDATE_VERSION
# ARG PLJAVA_VERSION
# ARG VAULT_VERSION
# ARG WRAPPERS_VERSION
# ARG PG_TLE_VERSION
# ARG WAL_G_VERSION
# ARG MYSQL_FDW_VERSION
# ARG PARQUET_S3_FDW_VERSION

ENV USE_PGXS=1

# Rum requirements
RUN apt-get update && apt-get install -y --no-install-recommends \
    systemtap-sdt-dev \
    && rm -rf /var/lib/apt/lists/*

# PostGIS requirements
RUN apt-get update && apt-get install -y --no-install-recommends \
    libproj-dev \
    libgeos-dev \
    libxml2-dev \
    gettext \
    libtool \
    libjson-c-dev \
    libgdal-dev \
    libsfcgal-dev \
    libprotobuf-c-dev \
    protobuf-c-compiler \
    && rm -rf /var/lib/apt/lists/*

RUN apt-get update && apt-get install -y --no-install-recommends \
    # pg_repack requirements
    libreadline-dev \
    # pgaudit requirements
    libkrb5-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy the install_pg_extension.sh script into the container
COPY scripts/install_pg_extensions.sh /usr/local/bin/

RUN /usr/local/bin/install_pg_extensions.sh \
    "pg_cron,${PG_CRON_VERSION},https://github.com/citusdata/pg_cron/archive/refs/tags/v${PG_CRON_VERSION}.tar.gz" \
    "pg_ivm,${PG_IVM_VERSION},https://github.com/sraoss/pg_ivm/archive/refs/tags/v${PG_IVM_VERSION}.tar.gz" \
    "pg_hashids,${PG_HASHIDS_VERSION},https://github.com/iCyberon/pg_hashids/archive/refs/tags/v${PG_HASHIDS_VERSION}.tar.gz" \
    "pg_repack,${PG_REPACK_VERSION},https://github.com/reorg/pg_repack/archive/refs/tags/${PG_REPACK_VERSION}.tar.gz" \
    "pg_stat_monitor,${PG_STAT_MONITOR_VERSION},https://github.com/percona/pg_stat_monitor/archive/refs/tags/${PG_STAT_MONITOR_VERSION}.tar.gz" \
    "pg_hint_plan,${PG_HINT_PLAN_VERSION},https://github.com/ossc-db/pg_hint_plan/archive/refs/tags/${PG_HINT_PLAN_VERSION}.tar.gz" \
    "pgtap,${PGTAP_VERSION},https://github.com/theory/pgtap/archive/refs/tags/v${PGTAP_VERSION}.tar.gz" \
    "pgvector,${PGVECTOR_VERSION},https://github.com/pgvector/pgvector/archive/refs/tags/v${PGVECTOR_VERSION}.tar.gz" \
    "postgis,${POSTGIS_VERSION},https://github.com/postgis/postgis/archive/refs/tags/${POSTGIS_VERSION}.tar.gz" \
    "pgaudit,${PGAUDIT_VERSION},https://github.com/pgaudit/pgaudit/archive/refs/tags/${PGAUDIT_VERSION}.tar.gz" \
    "pgrouting,${PGROUTING_VERSION},https://github.com/pgRouting/pgrouting/archive/refs/tags/v${PGROUTING_VERSION}.tar.gz" \
    "pgsql-http,${PGSQL_HTTP_VERSION},https://github.com/pramsey/pgsql-http/archive/refs/tags/v${PGSQL_HTTP_VERSION}.tar.gz" \
    "hypopg,${HYPOPG_VERSION},https://github.com/HypoPG/hypopg/archive/refs/tags/${HYPOPG_VERSION}.tar.gz" \
    "rum,${RUM_VERSION},https://github.com/postgrespro/rum/archive/refs/tags/${RUM_VERSION}.tar.gz"

# Install pre-compiled extensions
ADD "https://github.com/supabase/pg_net/releases/download/v${PG_NET_VERSION}/pg_net-v${PG_NET_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" /tmp/pg_net.deb
ADD "https://github.com/supabase/pg_graphql/releases/download/v${PG_GRAPHQL_VERSION}/pg_graphql-v${PG_GRAPHQL_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" /tmp/pg_graphql.deb
ADD "https://github.com/supabase/pg_jsonschema/releases/download/v${PG_JSONSCHEMA_VERSION}/pg_jsonschema-v${PG_JSONSCHEMA_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" /tmp/pg_jsonschema.deb

####################
# pgml
####################

FROM builder as builder-pgml

ARG PG_VERSION_MAJOR
ARG PGML_VERSION

RUN apt-get update && apt-fast install -y --no-install-recommends \
    libssl-dev \
    bison \
    flex \
    libreadline-dev \
    libz-dev \
    tzdata \
    libpq-dev \
    libclang-dev \
    && rm -rf /var/lib/apt/lists/*

RUN git clone --branch "v${PGML_VERSION}" "https://github.com/postgresml/postgresml" /tmp/postgresml

WORKDIR /tmp/postgresml/pgml-extension

RUN git submodule update --init --recursive && \
    git fetch --tags && \
    git fetch --depth 1 origin tag v${PGML_VERSION} && \
    git checkout v${PGML_VERSION}

# Initialize pgrx
RUN cargo pgrx init --pg${PG_VERSION_MAJOR}=/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config && \
    cargo pgrx package --pg-config /usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config

###############################################
# Third Stage: PostgreSQL
###############################################

FROM base as paradedb

ARG PG_VERSION_MAJOR

# Set the working directory
WORKDIR /usr/src/app

# Install pgml runtime dependencies
RUN pip3 install --no-cache-dir xgboost lightgbm scikit-learn

# # Copy the pg_bm25 extension from the builder stage
COPY --from=builder-pg_bm25 /usr/src/app/target/release/pg_bm25-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/* /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder-pg_bm25 /usr/src/app/target/release/pg_bm25-pg${PG_VERSION_MAJOR}/usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/* /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/

# # Copy the pgml extension from the builder stage
COPY --from=builder-pgml /tmp/postgresml/pgml-extension/target/release/pgml-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/* /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder-pgml /tmp/postgresml/pgml-extension/target/release/pgml-pg${PG_VERSION_MAJOR}/usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/* /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/

# Load extensions
COPY --from=builder-generic /tmp/*.deb /tmp/

# Install the Postgres extensions
RUN apt-get update && apt-fast install -y --no-install-recommends \
    /tmp/*.deb \
    && rm -rf /var/lib/apt/lists/* /tmp/*

# Copy the database initialization script into the container
COPY ./conf/postgresql.conf /etc/postgresql/${PG_VERSION_MAJOR}/main/postgresql.conf
COPY ./conf/pg_hba.conf /etc/postgresql/${PG_VERSION_MAJOR}/main/pg_hba.conf
COPY ./conf/pg_ident.conf /etc/postgresql/${PG_VERSION_MAJOR}/main/pg_ident.conf

# Copy the entrypoint script into the container
COPY ./entrypoint.sh /usr/src/app

# Expose the PostgreSQL port
# TODO: can i remove this?
EXPOSE 5432

# Set the entrypoint script
ENTRYPOINT ["/usr/src/app/entrypoint.sh"]
