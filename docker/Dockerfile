# The default Postgres major version ParadeDB ships with. In CI, this gets updated to ship builds for all official PGDG versions.
ARG PG_VERSION_MAJOR=18

# Antithesis is a deterministic simulation testing tool requiring instrumentation at compile-time and runtime. We only enable it
# when building ParadeDB images specifically for use with Antithesis.
ARG ENABLE_ANTITHESIS_INSTRUMENTATION=false

###############################################
# First Stage: Builder
###############################################

# Note: Debian Trixie = Debian 13
FROM postgres:${PG_VERSION_MAJOR}-trixie AS builder

ARG PG_VERSION_MAJOR

# Declare buildtime environment variables
ENV PG_VERSION_MAJOR=${PG_VERSION_MAJOR} \
    RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo

SHELL ["/bin/bash", "-o", "pipefail", "-c", "-e"]

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    clang \
    curl \
    git \
    gnupg \
    libopenblas-dev \
    pkg-config \
    postgresql-server-dev-${PG_VERSION_MAJOR} \
    && rm -rf /var/lib/apt/lists/*

COPY --from=rust:1.95.0-trixie /usr/local/cargo /usr/local/cargo
COPY --from=rust:1.95.0-trixie /usr/local/rustup /usr/local/rustup

ENV PATH="/usr/local/bin:/usr/local/cargo/bin:$PATH" \
    PGX_HOME=/usr/lib/postgresql/${PG_VERSION_MAJOR}

# Copy only Cargo.toml to extract pgrx version without invalidating cache on source changes
COPY Cargo.toml /tmp/
WORKDIR /tmp/

# Extract version from Cargo.toml (handles 'pgrx = "=0.18.0"')
# This replaces `cargo tree` to avoid needing the full source tree
RUN PGRX_VERSION=$(grep 'pgrx =' Cargo.toml | head -n 1 | cut -d '"' -f 2 | tr -d '=') && \
    echo "PGRX_VERSION=$PGRX_VERSION" && \
    cargo install --locked cargo-pgrx --version "${PGRX_VERSION}" && \
    cargo pgrx init "--pg${PG_VERSION_MAJOR}=/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config"

######################
# pg_search
######################

ARG ENABLE_ANTITHESIS_INSTRUMENTATION

# Declare compile-time environment variables
ENV ENABLE_ANTITHESIS_INSTRUMENTATION=${ENABLE_ANTITHESIS_INSTRUMENTATION}

COPY Cargo.toml Cargo.lock rust-toolchain.toml /tmp/
COPY .cargo/ /tmp/.cargo/
COPY pg_search/ /tmp/pg_search/
COPY tokenizers/ /tmp/tokenizers/
COPY macros/ /tmp/macros/

# tests, benchmarks, and stressgres are only required because they are included as workspace members
# in the main Cargo.toml
COPY tests/Cargo.toml /tmp/tests/
COPY tests/src/lib.rs /tmp/tests/src/
COPY benchmarks/Cargo.toml /tmp/benchmarks/
COPY benchmarks/src/lib.rs /tmp/benchmarks/src/
COPY stressgres/Cargo.toml /tmp/stressgres/
COPY stressgres/src/main.rs /tmp/stressgres/src/

# Build the extension in release mode, but with debug symbols (which will be split out later)
# Optionally install the Antithesis instrumentation library if this Docker build is intended for use with Antithesis
RUN if [ "$ENABLE_ANTITHESIS_INSTRUMENTATION" = "true" ]; then \
    curl -L -o /usr/lib/libvoidstar.so https://antithesis.com/assets/instrumentation/libvoidstar.so; \
    export RUSTFLAGS="-Ccodegen-units=1 -Cpasses=sancov-module -Cllvm-args=-sanitizer-coverage-level=3 -Cllvm-args=-sanitizer-coverage-trace-pc-guard -Clink-args=-Wl,--build-id  -L/usr/lib/libvoidstar.so -lvoidstar" ; \
  fi; \
  cd /tmp/pg_search && \
  cargo pgrx package --profile=release-with-debug --pg-config "/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config"

# Strip debug symbols from the extension and add a debug link
RUN LIB_PATH="/tmp/target/release-with-debug/pg_search-pg${PG_VERSION_MAJOR}/usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/pg_search.so" && \
    objcopy --only-keep-debug "${LIB_PATH}" "${LIB_PATH}.dbg" && \
    objcopy --strip-debug "${LIB_PATH}" && \
    objcopy --add-gnu-debuglink="${LIB_PATH}.dbg" "${LIB_PATH}"

######################
# pgvector v0.8.1
######################

ENV PG_CONFIG=/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config

# Build the extension
WORKDIR /tmp
RUN curl -fsSL -o pgvector.tar.gz "https://codeload.github.com/pgvector/pgvector/tar.gz/778dacf20c07caf904557a88705142631818d8cb" && \
    echo "4c33cf053329784ba6d992d05c9588b93789e907a7511f20ff5a5a5b8a0703c1 *pgvector.tar.gz" | sha256sum --strict --check && \
    mkdir pgvector && \
    tar -xzf pgvector.tar.gz -C pgvector --strip-components=1 && \
    rm pgvector.tar.gz
WORKDIR /tmp/pgvector
RUN export PG_CFLAGS="-Wall -Wextra -Wno-unused-parameter -Wno-sign-compare" && \
    make clean -j && \
    make USE_PGXS=1 OPTFLAGS="" -j

######################
# pg_cron v1.6.7
######################

# Build the extension
WORKDIR /tmp
RUN curl -fsSL -o pg_cron.tar.gz "https://codeload.github.com/citusdata/pg_cron/tar.gz/465b38c737f584d520229f5a1d69d1d44649e4e5" && \
    echo "ab41d388d845c05ab6f34fa8e12011da2d71f7f562194ee105a6fdecb506a70f *pg_cron.tar.gz" | sha256sum --strict --check && \
    mkdir pg_cron && \
    tar -xzf pg_cron.tar.gz -C pg_cron --strip-components=1 && \
    rm pg_cron.tar.gz
WORKDIR /tmp/pg_cron
RUN make clean -j && \
    make USE_PGXS=1 -j

######################
# pg_ivm v1.13
######################

# Build the extension
WORKDIR /tmp
RUN curl -fsSL -o pg_ivm.tar.gz "https://codeload.github.com/sraoss/pg_ivm/tar.gz/98a262ce0906b8afbb8e6bce8dd05779f8c50cac" && \
    echo "8d91beee4cf0f1d5bfc2b056b7513e0ea060d4da442926f5c6925ed6b77c528c *pg_ivm.tar.gz" | sha256sum --strict --check && \
    mkdir pg_ivm && \
    tar -xzf pg_ivm.tar.gz -C pg_ivm --strip-components=1 && \
    rm pg_ivm.tar.gz
WORKDIR /tmp/pg_ivm
RUN make clean -j && \
    make USE_PGXS=1 -j

###############################################
# Second Stage: PostgreSQL and Barman Cloud
###############################################

FROM postgres:${PG_VERSION_MAJOR}-trixie AS paradedb

LABEL maintainer="ParadeDB - https://paradedb.com" \
    org.opencontainers.image.description="ParadeDB - The Transactional Elasticsearch Alternative Built on Postgres" \
    org.opencontainers.image.source="https://github.com/paradedb/paradedb"

ARG PG_VERSION_MAJOR
ARG ENABLE_ANTITHESIS_INSTRUMENTATION

# Declare runtime environment variables
ENV PG_VERSION_MAJOR=${PG_VERSION_MAJOR} \
    ENABLE_ANTITHESIS_INSTRUMENTATION=${ENABLE_ANTITHESIS_INSTRUMENTATION}

SHELL ["/bin/bash", "-o", "pipefail", "-c", "-e"]

# Copy third-party extensions from their builder stages
COPY --from=builder /tmp/pgvector/*.so /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/
COPY --from=builder /tmp/pgvector/*.control /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder /tmp/pgvector/sql/*.sql /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder /tmp/pg_cron/*.so /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/
COPY --from=builder /tmp/pg_cron/*.control /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder /tmp/pg_cron/*.sql /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder /tmp/pg_ivm/*.so /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/
COPY --from=builder /tmp/pg_ivm/*.control /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder /tmp/pg_ivm/*.sql /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/

# Install Barman Cloud and its dependencies for Azure, Google, and AWS via `pip`, and clean up after the installation to
# minimize the size of the image. These are required for enabling Postgres backups in our CloudNativePG deployments.
RUN apt-get update && \
    apt-get install -y --no-install-recommends libpq5 python3-pip python3-dev python3-psycopg2 && \
    rm /usr/lib/python*/EXTERNALLY-MANAGED && \
    pip3 install --no-cache-dir 'setuptools==82.0.1' 'barman[cloud,azure,snappy,google]==3.18.0' && \
    apt-get remove -y python3-dev python3-pip --purge && \
    apt-get autoremove -y && \
    rm -rf /var/lib/apt/lists/* && \
    find /usr/lib | grep -E "(/__pycache__$|\.pyc$|\.pyo$)" | xargs rm -rf && \
    find /usr/local | grep -E "(/__pycache__$|\.pyc$|\.pyo$)" | xargs rm -rf && \
    find /var/cache -type f -exec truncate --size 0 {} \; && \
    find /var/log -type f -exec truncate --size 0 {} \;

# Install Postgis and ca-certificates
# ca-certificates required for PostGIS
ENV POSTGIS_VERSION_MAJOR=3
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
    postgresql-$PG_VERSION_MAJOR-postgis-$POSTGIS_VERSION_MAJOR \
    postgresql-$PG_VERSION_MAJOR-postgis-$POSTGIS_VERSION_MAJOR-scripts \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/* && \
    update-ca-certificates

# The postgresql.conf.sample file is used as a template for the postgresql.conf file, which
# does not exist until the first time the container is started. By adding our settings to the
# postgresql.conf.sample file, we ensure that our settings are applied onto the postgresql.conf file.
#
# The `postgres` database is the default database that exists in every Postgres installation. The pg_cron
# extension requires a database to store its metadata tables. By using `postgres`, we ensure that it has a
# stable, always-available database for its operations, no matter what other databases are created or deleted.
RUN sed -i "s/^#shared_preload_libraries = ''/shared_preload_libraries = 'pg_search,pg_cron,pg_stat_statements'/" /usr/share/postgresql/postgresql.conf.sample && \
    grep "shared_preload_libraries = 'pg_search,pg_cron,pg_stat_statements'" /usr/share/postgresql/postgresql.conf.sample && \
    echo "cron.database_name = 'postgres'" >> /usr/share/postgresql/postgresql.conf.sample && \
    echo "pg_stat_statements.track = 'top'" >> /usr/share/postgresql/postgresql.conf.sample

# Copy the ParadeDB extension from its builder stage
COPY --from=builder /tmp/target/release-with-debug/pg_search-pg${PG_VERSION_MAJOR}/usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/* /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/
COPY --from=builder /tmp/target/release-with-debug/pg_search-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/* /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/

# Optionally install the Antithesis instrumentation library if this Docker build is intended for use with Antithesis
RUN if [ "$ENABLE_ANTITHESIS_INSTRUMENTATION" = "true" ]; then \
   apt-get update && \
   apt-get install -y --no-install-recommends wget && \
   wget -qO /usr/lib/libvoidstar.so https://antithesis.com/assets/instrumentation/libvoidstar.so && \
   apt-get purge -y wget && \
   apt-get autoremove -y && \
   rm -rf /var/lib/apt/lists/* && \
   mkdir /symbols && \
   ln -s /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/pg_search.so /symbols/pg_search.so ; \
 fi

# Reset the working directory to the root directory
WORKDIR /

# Copy ParadeDB bootstrap script to install extensions and configure postgresql.conf
COPY ./docker/bootstrap.sh /docker-entrypoint-initdb.d/10_bootstrap_paradedb.sh

# The upstream `postgres` Docker image comes with its own `entrypoint.sh` script which
# starts as `root` and then switches to the `postgres` user after running chown and chmod
# on the PostgreSQL data directory. To maintain compatibility with the upstream image and
# ensure that the `postgres` user has the correct permissions on the data directory, we let
# the upstream `entrypoint.sh` script run as the entrypoint and don't specify a custom user.
