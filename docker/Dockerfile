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
ARG RUST_VERSION=stable

# Declare buildtime environment variables
ENV PG_VERSION_MAJOR=${PG_VERSION_MAJOR} \
    RUST_VERSION=${RUST_VERSION}

SHELL ["/bin/bash", "-o", "pipefail", "-c", "-e"]

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
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

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain "${RUST_VERSION}" -y

ENV PATH="/usr/local/bin:/root/.cargo/bin:$PATH" \
    PGX_HOME=/usr/lib/postgresql/${PG_VERSION_MAJOR}

# Copy only Cargo.toml to extract pgrx version without invalidating cache on source changes
COPY Cargo.toml /tmp/
WORKDIR /tmp/

# Extract version from Cargo.toml (handles 'pgrx = "=0.16.1"')
# This replaces `cargo tree` to avoid needing the full source tree
RUN PGRX_VERSION=$(grep 'pgrx =' Cargo.toml | head -n 1 | cut -d '"' -f 2 | tr -d '=') && \
    echo "PGRX_VERSION=$PGRX_VERSION" && \
    cargo install --locked cargo-pgrx --version "${PGRX_VERSION}" && \
    cargo pgrx init "--pg${PG_VERSION_MAJOR}=/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config"

######################
# pg_search
######################

FROM builder AS builder-pg_search

ARG COMMIT_SHA
ARG PARADEDB_VERSION
ARG ENABLE_ANTITHESIS_INSTRUMENTATION

# Declare compile-time environment variables
ENV COMMIT_SHA=${COMMIT_SHA} \
    PARADEDB_VERSION=${PARADEDB_VERSION} \
    ENABLE_ANTITHESIS_INSTRUMENTATION=${ENABLE_ANTITHESIS_INSTRUMENTATION}

# Copy the full project. This layer will invalidate if any project file changes.
COPY . /tmp/

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
# pgvector
######################

FROM builder AS builder-pgvector

ARG PG_VERSION_MAJOR
ENV PG_CONFIG=/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config

# Build the extension
WORKDIR /tmp
RUN git clone --branch v0.8.1 https://github.com/pgvector/pgvector.git
WORKDIR /tmp/pgvector
RUN export PG_CFLAGS="-Wall -Wextra -Wno-unused-parameter -Wno-sign-compare" && \
    echo "trusted = true" >> vector.control && \
    make clean -j && \
    make USE_PGXS=1 OPTFLAGS="" -j

######################
# pg_cron
######################

FROM builder AS builder-pg_cron

ARG PG_VERSION_MAJOR
ENV PG_CONFIG=/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config

# Build the extension
WORKDIR /tmp
RUN git clone --branch v1.6.7 https://github.com/citusdata/pg_cron.git
WORKDIR /tmp/pg_cron
RUN echo "trusted = true" >> pg_cron.control && \
    make clean -j && \
    make USE_PGXS=1 -j

######################
# pg_ivm
######################

FROM builder AS builder-pg_ivm

ARG PG_VERSION_MAJOR
ENV PG_CONFIG=/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config

# Build the extension
WORKDIR /tmp
RUN git clone --branch v1.13 https://github.com/sraoss/pg_ivm.git
WORKDIR /tmp/pg_ivm
RUN echo "trusted = true" >> pg_ivm.control && \
    make clean -j && \
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
COPY --from=builder-pgvector /tmp/pgvector/*.so /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/
COPY --from=builder-pgvector /tmp/pgvector/*.control /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder-pgvector /tmp/pgvector/sql/*.sql /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder-pg_cron /tmp/pg_cron/*.so /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/
COPY --from=builder-pg_cron /tmp/pg_cron/*.control /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder-pg_cron /tmp/pg_cron/*.sql /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder-pg_ivm /tmp/pg_ivm/*.so /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/
COPY --from=builder-pg_ivm /tmp/pg_ivm/*.control /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder-pg_ivm /tmp/pg_ivm/*.sql /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/

# Install Barman Cloud and its dependencies for Azure, Google, and AWS via `pip`, and clean up after the installation to
# minimize the size of the image. These are required for enabling Postgres backups in our CloudNativePG deployments.
RUN apt-get update && \
    apt-get install -y --no-install-recommends libpq5 python3-pip python3-dev python3-psycopg2 && \
    rm /usr/lib/python*/EXTERNALLY-MANAGED && \
    pip3 install --no-cache-dir 'barman[cloud,azure,snappy,google]==3.13.2' && \
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


WORKDIR /tmp
RUN apt-get update && apt-get install -y --no-install-recommends wget && \
    rm -rf /var/lib/apt/lists/*

# Copy the ParadeDB extension from its builder stage
COPY --from=builder-pg_search /tmp/target/release-with-debug/pg_search-pg${PG_VERSION_MAJOR}/usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/* /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/
COPY --from=builder-pg_search /tmp/target/release-with-debug/pg_search-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/* /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/

# Optionally install the Antithesis instrumentation library if this Docker build is intended for use with Antithesis
RUN if [ "$ENABLE_ANTITHESIS_INSTRUMENTATION" = "true" ]; then \
   wget -qO /usr/lib/libvoidstar.so https://antithesis.com/assets/instrumentation/libvoidstar.so && \
   mkdir /symbols && \
   ln -s /usr/lib/postgresql/18/lib/pg_search.so /symbols/pg_search.so ; \
 fi

# In order for users to manually install additional PostgreSQL extensions
# (e.g. PostGIS, pg_partman, etc.) that ParadeDB does not ship with by default,
# we ensure the PostgreSQL APT (PGDG) repository is present in the image.
#
# The upstream `postgres` image already installs the PGDG signing key into
# /usr/local/share/keyrings/postgres.gpg.asc. We reference that key explicitly
# via `signed-by` so the repository can be verified correctly.
#
# We intentionally do NOT run `apt-get update` here to avoid downloading
# package indexes during image build. Users can run `apt-get update`
# themselves before installing additional packages.
RUN echo "deb [signed-by=/usr/local/share/keyrings/postgres.gpg.asc] https://apt.postgresql.org/pub/repos/apt trixie-pgdg main" \
    > /etc/apt/sources.list.d/pgdg.list && \
    apt-get purge -y wget && \
    apt-get autoremove -y && \
    rm -rf /var/lib/apt/lists/*

# Reset the working directory to the root directory
WORKDIR /

# Copy ParadeDB bootstrap script to install extensions and configure postgresql.conf
COPY ./docker/bootstrap.sh /docker-entrypoint-initdb.d/10_bootstrap_paradedb.sh

# The upstream `postgres` Docker image comes with its own `entrypoint.sh` script which
# starts as `root` and then switches to the `postgres` user after running chown and chmod
# on the PostgreSQL data directory. To maintain compatibility with the upstream image and
# ensure that the `postgres` user has the correct permissions on the data directory, we let
# the upstream `entrypoint.sh` script run as the entrypoint and don't specify a custom user.

# Copy the auto-tuning scripts. 'tune-postgresql.sh' contains the logic to calculate
# configuration values based on container resources. 'entrypoint-wrapper.sh' wraps
# the upstream entrypoint to ensure tuning runs at the correct time (boot vs restart).
COPY ./docker/tune-postgresql.sh /usr/local/bin/tune-postgresql.sh
COPY ./docker/entrypoint-wrapper.sh /usr/local/bin/entrypoint-wrapper.sh
RUN chmod +x /usr/local/bin/tune-postgresql.sh /usr/local/bin/entrypoint-wrapper.sh

# Override the default entrypoint to use our wrapper. This wrapper will trigger
# the auto-tuning script and then exec the upstream docker-entrypoint.sh.
ENTRYPOINT ["/usr/local/bin/entrypoint-wrapper.sh"]

CMD ["postgres"]
