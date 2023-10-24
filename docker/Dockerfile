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
ARG PGML_VERSION
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
ARG PGVECTOR_VERSION
ARG PGAUDIT_VERSION
ARG POSTGIS_VERSION
ARG PGROUTING_VERSION
ARG PGSQL_HTTP_VERSION
ARG HYPOPG_VERSION
ARG RUM_VERSION
ARG AGE_VERSION
ARG POSTHOG_API_KEY
ARG POSTHOG_HOST
ARG COMMIT_SHA
ARG TELEMETRY

ENV PG_VERSION_MAJOR=${PG_VERSION_MAJOR} \
    PG_BM25_VERSION=${PG_BM25_VERSION} \
    PG_SEARCH_VERSION=${PG_SEARCH_VERSION} \
    PGML_VERSION=${PGML_VERSION} \
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
    PGVECTOR_VERSION=${PGVECTOR_VERSION} \
    PGAUDIT_VERSION=${PGAUDIT_VERSION} \
    POSTGIS_VERSION=${POSTGIS_VERSION} \
    PGROUTING_VERSION=${PGROUTING_VERSION} \
    PGSQL_HTTP_VERSION=${PGSQL_HTTP_VERSION} \
    HYPOPG_VERSION=${HYPOPG_VERSION} \
    RUM_VERSION=${RUM_VERSION} \
    AGE_VERSION=${AGE_VERSION} \
    POSTHOG_API_KEY=${POSTHOG_API_KEY} \
    POSTHOG_HOST=${POSTHOG_HOST} \
    COMMIT_SHA=${COMMIT_SHA} \
    TELEMETRY=${TELEMETRY} \
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

# Install Rust and other build dependencies
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
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

ENV PATH="/root/.cargo/bin:$PATH" \
    PGX_HOME=/usr/lib/postgresql/${PG_VERSION_MAJOR}

RUN cargo install cargo-pgrx --version 0.11.0 && \
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

# Install pgml and postgis runtime dependencies
RUN apt-fast update && apt-fast install -y --no-install-recommends \
    # postgis
    libgeos-c1v5 \
    libproj-dev \
    libprotobuf-c1 \
    libc++1 libc++abi1 \
    && rm -rf /var/lib/apt/lists/* /tmp/* && \
    # pgml
    pip3 install --no-cache-dir --break-system-packages accelerate==0.22.0 bitsandbytes==0.41.1 catboost==1.2 ctransformers==0.2.27 datasets==2.14.5 deepspeed==0.10.3 huggingface-hub==0.17.1 InstructorEmbedding==1.0.1 lightgbm==4.1.0 orjson==3.9.7 pandas==2.1.0 rich==13.5.2 rouge==1.0.1 sacrebleu==2.3.1 sacremoses==0.0.53 scikit-learn==1.3.0 sentencepiece==0.1.99 sentence-transformers==2.2.2 tokenizers==0.13.3 torch==2.0.1 torchaudio==2.0.2 torchvision==0.15.2 tqdm==4.66.1 transformers==4.33.1 xgboost==2.0.0 langchain==0.0.287 einops==0.6.1 pynvml==11.5.0 && \    
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
RUN curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pgvector-v${PGVECTOR_VERSION}-$TARGETARCH/pgvector-v${PGVECTOR_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pgvector.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pgaudit-v${PGAUDIT_VERSION}-$TARGETARCH/pgaudit-v${PGAUDIT_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pgaudit.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pg_cron-v${PG_CRON_VERSION}-$TARGETARCH/pg_cron-v${PG_CRON_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pg_cron.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pg_ivm-v${PG_IVM_VERSION}-$TARGETARCH/pg_ivm-v${PG_IVM_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pg_ivm.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pg_hashids-v${PG_HASHIDS_VERSION}-$TARGETARCH/pg_hashids-v${PG_HASHIDS_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pg_hashids.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pg_repack-v${PG_REPACK_VERSION}-$TARGETARCH/pg_repack-v${PG_REPACK_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pg_repack.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pg_stat_monitor-v${PG_STAT_MONITOR_VERSION}-$TARGETARCH/pg_stat_monitor-v${PG_STAT_MONITOR_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pg_stat_monitor.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pg_hint_plan-v${PG_HINT_PLAN_VERSION}-$TARGETARCH/pg_hint_plan-v${PG_HINT_PLAN_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pg_hint_plan.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pg_roaringbitmap-v${PG_ROARINGBITMAP_VERSION}-$TARGETARCH/pg_roaringbitmap-v${PG_ROARINGBITMAP_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pg_roaringbitmap.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pgfaceting-v${PGFACETING_VERSION}-$TARGETARCH/pgfaceting-v${PGFACETING_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pgfaceting.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pgtap-v${PGTAP_VERSION}-$TARGETARCH/pgtap-v${PGTAP_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pgtap.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/postgis-v${POSTGIS_VERSION}-$TARGETARCH/postgis-v${POSTGIS_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/postgis.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pgrouting-v${PGROUTING_VERSION}-$TARGETARCH/pgrouting-v${PGROUTING_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pgrouting.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pgsql-http-v${PGSQL_HTTP_VERSION}-$TARGETARCH/pgsql-http-v${PGSQL_HTTP_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pgsql-http.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/hypopg-v${HYPOPG_VERSION}-$TARGETARCH/hypopg-v${HYPOPG_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/hypopg.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/rum-v${RUM_VERSION}-$TARGETARCH/rum-v${RUM_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/rum.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/age-v${AGE_VERSION}-$TARGETARCH/age-v${AGE_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/age.deb && \
    curl -L "https://github.com/paradedb/third-party-pg_extensions/releases/download/pgml-v${PGML_VERSION}-$TARGETARCH/pgml-v${PGML_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pgml.deb && \
    curl -L "https://github.com/supabase/pg_net/releases/download/v${PG_NET_VERSION}/pg_net-v${PG_NET_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pg_net.deb && \
    curl -L "https://github.com/supabase/pg_graphql/releases/download/v${PG_GRAPHQL_VERSION}/pg_graphql-v${PG_GRAPHQL_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pg_graphql.deb && \
    curl -L "https://github.com/supabase/pg_jsonschema/releases/download/v${PG_JSONSCHEMA_VERSION}/pg_jsonschema-v${PG_JSONSCHEMA_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" -o /tmp/pg_jsonschema.deb && \
    apt-fast update && apt-fast install -y --no-install-recommends /tmp/*.deb \
    && rm -rf /var/lib/apt/lists/* /tmp/*

# Copy the pg_bm25 & pg_search extensions from their builder stages
# We order them by how unlikely they are to be modified, to reduce layer rebuilds
COPY --from=builder-pg_search /tmp/pg_search/target/release/pg_search-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/* /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder-pg_search /tmp/pg_search/target/release/pg_search-pg${PG_VERSION_MAJOR}/usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/* /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/
COPY --from=builder-pg_bm25 /tmp/pg_bm25/target/release/pg_bm25-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/* /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder-pg_bm25 /tmp/pg_bm25/target/release/pg_bm25-pg${PG_VERSION_MAJOR}/usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/* /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/

# Copy entrypoint script, which will be handled by the official image
# initialization scipt
COPY ./scripts/entrypoint.sh /docker-entrypoint-initdb.d/10_paradedb.sh

# Change the uid of postgres to 26
RUN usermod -u 26 postgres \
    && chown -R 26:26 /var/lib/postgresql \
    && chown -R 26:26 /var/run/postgresql \
    && chown -R 26:26 /usr/lib/postgresql
USER 26
