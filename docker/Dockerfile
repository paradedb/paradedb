###############################################
# First Stage: Base
###############################################

FROM ubuntu:22.04 as base

# Declare all the build arguments and set them to
# environment variables for use in build and runtime
ARG PG_VERSION_MAJOR
ARG PG_BM25_VERSION
ARG PG_SEARCH_VERSION
ARG PG_CRON_VERSION
ARG PG_NET_VERSION
ARG PG_IVM_VERSION
ARG PG_GRAPHQL_VERSION
ARG PG_HASHIDS_VERSION
ARG PG_JSONSCHEMA_VERSION
ARG PG_REPACK_VERSION
ARG PG_STAT_MONITOR_VERSION
ARG PG_HINT_PLAN_VERSION
ARG PGML_VERSION
ARG PGTAP_VERSION
ARG PGVECTOR_VERSION
ARG PGAUDIT_VERSION
ARG POSTGIS_VERSION
ARG PGROUTING_VERSION
ARG PGSQL_HTTP_VERSION
ARG HYPOPG_VERSION
ARG RUM_VERSION
ARG POSTHOG_API_KEY
ARG COMMIT_SHA

ENV PG_VERSION_MAJOR=${PG_VERSION_MAJOR} \
    PG_BM25_VERSION=${PG_BM25_VERSION} \
    PG_SEARCH_VERSION=${PG_SEARCH_VERSION} \
    PG_CRON_VERSION=${PG_CRON_VERSION} \
    PG_NET_VERSION=${PG_NET_VERSION} \
    PG_IVM_VERSION=${PG_IVM_VERSION} \
    PG_GRAPHQL_VERSION=${PG_GRAPHQL_VERSION} \
    PG_HASHIDS_VERSION=${PG_HASHIDS_VERSION} \
    PG_JSONSCHEMA_VERSION=${PG_JSONSCHEMA_VERSION} \
    PG_REPACK_VERSION=${PG_REPACK_VERSION} \
    PG_STAT_MONITOR_VERSION=${PG_STAT_MONITOR_VERSION} \
    PG_HINT_PLAN_VERSION=${PG_HINT_PLAN_VERSION} \
    PGML_VERSION=${PGML_VERSION} \
    PGTAP_VERSION=${PGTAP_VERSION} \
    PGVECTOR_VERSION=${PGVECTOR_VERSION} \
    PGAUDIT_VERSION=${PGAUDIT_VERSION} \
    POSTGIS_VERSION=${POSTGIS_VERSION} \
    PGROUTING_VERSION=${PGROUTING_VERSION} \
    PGSQL_HTTP_VERSION=${PGSQL_HTTP_VERSION} \
    HYPOPG_VERSION=${HYPOPG_VERSION} \
    RUM_VERSION=${RUM_VERSION} \
    POSTHOG_API_KEY=${POSTHOG_API_KEY} \
    COMMIT_SHA=${COMMIT_SHA} \
    DEBIAN_FRONTEND=noninteractive \
    TZ=UTC

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

# Create postgres user with uid
RUN mkdir -p /var/lib/postgresql && useradd -u 1001 -o -m -d /var/lib/postgresql postgres

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

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

# Install Rust and other build dependencies
RUN apt-get update && apt-fast install -y --no-install-recommends \
    build-essential \
    checkinstall \
    clang \
    git \
    cmake \
    pkg-config \
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
    libssl-dev \
    bison \
    flex \
    libreadline-dev \
    libz-dev \
    tzdata \
    libpq-dev \
    libclang-dev \
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

######################
# Other extensions
######################

FROM builder as builder-others

ARG TARGETARCH

ENV USE_PGXS=1

RUN apt-get update && apt-fast install -y --no-install-recommends \
    # rum requirements
    systemtap-sdt-dev \
    # pg_repack requirements
    libreadline-dev \
    # pgaudit requirements
    libkrb5-dev \
    # postgis requirements
    libgeos-dev \
    libproj-dev \
    libprotobuf-c-dev \
    libsfcgal-dev \
    libxml2-dev \
    libjson-c-dev \
    libgdal-dev \
    gdal-bin \    
    gettext \
    libtool \
    xsltproc \
    protobuf-c-compiler \
    && rm -rf /var/lib/apt/lists/*

COPY scripts/install_pg_extensions.sh /usr/local/bin/

# Compile and install extensions from source
RUN /usr/local/bin/install_pg_extensions.sh \
    "pgvector,${PGVECTOR_VERSION},https://github.com/pgvector/pgvector/archive/refs/tags/v${PGVECTOR_VERSION}.tar.gz" \
    "pgaudit,${PGAUDIT_VERSION},https://github.com/pgaudit/pgaudit/archive/refs/tags/${PGAUDIT_VERSION}.tar.gz" \
    "pg_cron,${PG_CRON_VERSION},https://github.com/citusdata/pg_cron/archive/refs/tags/v${PG_CRON_VERSION}.tar.gz" \
    "pg_ivm,${PG_IVM_VERSION},https://github.com/sraoss/pg_ivm/archive/refs/tags/v${PG_IVM_VERSION}.tar.gz" \
    "pg_hashids,${PG_HASHIDS_VERSION},https://github.com/iCyberon/pg_hashids/archive/refs/tags/v${PG_HASHIDS_VERSION}.tar.gz" \
    "pg_repack,${PG_REPACK_VERSION},https://github.com/reorg/pg_repack/archive/refs/tags/${PG_REPACK_VERSION}.tar.gz" \
    "pg_stat_monitor,${PG_STAT_MONITOR_VERSION},https://github.com/percona/pg_stat_monitor/archive/refs/tags/${PG_STAT_MONITOR_VERSION}.tar.gz" \
    "pg_hint_plan,${PG_HINT_PLAN_VERSION},https://github.com/ossc-db/pg_hint_plan/archive/refs/tags/${PG_HINT_PLAN_VERSION}.tar.gz" \
    "pgtap,${PGTAP_VERSION},https://github.com/theory/pgtap/archive/refs/tags/v${PGTAP_VERSION}.tar.gz" \
    "postgis,${POSTGIS_VERSION},https://github.com/postgis/postgis/archive/refs/tags/${POSTGIS_VERSION}.tar.gz" \
    "pgrouting,${PGROUTING_VERSION},https://github.com/pgRouting/pgrouting/archive/refs/tags/v${PGROUTING_VERSION}.tar.gz" \
    "pgsql-http,${PGSQL_HTTP_VERSION},https://github.com/pramsey/pgsql-http/archive/refs/tags/v${PGSQL_HTTP_VERSION}.tar.gz" \
    "hypopg,${HYPOPG_VERSION},https://github.com/HypoPG/hypopg/archive/refs/tags/${HYPOPG_VERSION}.tar.gz" \
    "rum,${RUM_VERSION},https://github.com/postgrespro/rum/archive/refs/tags/${RUM_VERSION}.tar.gz"

# Install pre-compiled extensions
RUN curl -L "https://github.com/supabase/pg_net/releases/download/v${PG_NET_VERSION}/pg_net-v${PG_NET_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pg_net.deb && \
    curl -L "https://github.com/supabase/pg_graphql/releases/download/v${PG_GRAPHQL_VERSION}/pg_graphql-v${PG_GRAPHQL_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pg_graphql.deb && \
    curl -L "https://github.com/supabase/pg_jsonschema/releases/download/v${PG_JSONSCHEMA_VERSION}/pg_jsonschema-v${PG_JSONSCHEMA_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pg_jsonschema.deb

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
    && rm -rf /var/lib/apt/lists/* /tmp/* && \
    # pgml
    pip3 install --no-cache-dir -r requirements.txt && rm -rf requirements.txt && \
    # patroni
    pip3 install --no-cache-dir patroni[etcd3] psycopg[binary]>=3.0.0 && \
    # Crunchy operator
    # Only need to copy on x84_64, since the lib is already in the right place on arm64
    if [ "$TARGETARCH" = "amd64" ]; then \
        cp /usr/lib/x86_64-linux-gnu/libnss_wrapper.so /usr/lib64/; \
    fi

# Copy & install the non-pgrx extensions from their builder stage
COPY --from=builder-others /tmp/*.deb /tmp/
RUN apt-get update && apt-fast install -y --no-install-recommends \
    /tmp/*.deb \
    && rm -rf /var/lib/apt/lists/* /tmp/*

# Copy the pg_bm25, pgml & pg_search extensions from their builder stages
# We order them by how unlikely they are to be modified, to reduce layer rebuilds
COPY --from=builder-pgml /tmp/postgresml/pgml-extension/target/release/pgml-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/* /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder-pgml /tmp/postgresml/pgml-extension/target/release/pgml-pg${PG_VERSION_MAJOR}/usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/* /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/
COPY --from=builder-pg_search /tmp/pg_search/target/release/pg_search-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/* /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder-pg_search /tmp/pg_search/target/release/pg_search-pg${PG_VERSION_MAJOR}/usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/* /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/
COPY --from=builder-pg_bm25 /tmp/pg_bm25/target/release/pg_bm25-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/* /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder-pg_bm25 /tmp/pg_bm25/target/release/pg_bm25-pg${PG_VERSION_MAJOR}/usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/* /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/

# Copy the database initialization scripts into the container
COPY ./conf/postgresql.conf /conf/pg_hba.conf ./conf/pg_ident.conf /etc/postgresql/${PG_VERSION_MAJOR}/main/

# Copy the entrypoint script into the container
COPY ./scripts/entrypoint.sh /usr/src/app

# Add postgres binaries to path
ENV PATH="${PATH}:/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin"

# Create the postgres user and change to it
USER 1001

# Expose the PostgreSQL
EXPOSE 5432

ENTRYPOINT ["/usr/src/app/entrypoint.sh"]
