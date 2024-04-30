ARG PG_VERSION_MAJOR=16

# Note: As of writing, Debian 11/Bullseye is the latest stable Debian distro
# and if equivalent to Ubuntu 22.04, which we use for testing.

###############################################
# First Stage: Builder
###############################################

# We build the extensions on the official PostgreSQL image, since the Bitnami
# image does not have root access and necessary build tools
FROM postgres:${PG_VERSION_MAJOR}-bullseye as builder

ARG PG_VERSION_MAJOR=16
ARG RUST_VERSION=1.77.0
ARG PGRX_VERSION=0.12.0-alpha.1

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
    postgresql-server-dev-all \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    && /root/.cargo/bin/rustup default "${RUST_VERSION}"

ENV PATH="/root/.cargo/bin:$PATH" \
    PGX_HOME=/usr/lib/postgresql/${PG_VERSION_MAJOR}

RUN cargo install cargo-get && \
    cargo install --locked cargo-pgrx --version "${PGRX_VERSION}" && \
    cargo pgrx init "--pg${PG_VERSION_MAJOR}=/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config"

######################
# pg_search
######################

FROM builder as builder-pg_search

ARG POSTHOG_API_KEY
ARG POSTHOG_HOST
ARG COMMIT_SHA
ARG TELEMETRY

# Declare compile-time environment variables
ENV POSTHOG_API_KEY=${POSTHOG_API_KEY} \
    POSTHOG_HOST=${POSTHOG_HOST} \
    COMMIT_SHA=${COMMIT_SHA} \
    TELEMETRY=${TELEMETRY}

COPY pg_search/ /tmp/pg_search
COPY shared/ /tmp/shared
COPY tokenizers/ /tmp/tokenizers

# Build the extension
WORKDIR /tmp/pg_search
RUN cargo pgrx package --features icu --pg-config "/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config"

######################
# pg_analytics
######################

FROM builder as builder-pg_analytics

ARG POSTHOG_API_KEY
ARG POSTHOG_HOST
ARG COMMIT_SHA
ARG TELEMETRY

# Declare compile-time environment variables
ENV POSTHOG_API_KEY=${POSTHOG_API_KEY} \
    POSTHOG_HOST=${POSTHOG_HOST} \
    COMMIT_SHA=${COMMIT_SHA} \
    TELEMETRY=${TELEMETRY}

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
# pgvector
######################

FROM builder as builder-pgvector

# Build the extension
WORKDIR /tmp
RUN git clone --branch v0.6.0 https://github.com/pgvector/pgvector.git
WORKDIR /tmp/pgvector
RUN export PG_CFLAGS="-Wall -Wextra -Werror -Wno-unused-parameter -Wno-sign-compare" && \
    echo "trusted = true" >> vector.control && \
    make clean -j && \
    make USE_PGXS=1 OPTFLAGS="" -j

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
# citus
######################

# FROM builder as builder-citus

# # Install citus extra dependencies
# RUN apt-get update && apt-get install -y --no-install-recommends \
#     autoconf \
#     liblz4-dev \
#     libzstd-dev \
#     libkrb5-dev \
#     && rm -rf /var/lib/apt/lists/*

# # Build the extension
# WORKDIR /tmp
# RUN git clone --branch v12.1.3 https://github.com/citusdata/citus.git
# WORKDIR /tmp/citus
# RUN ./configure --without-libcurl && \
#     make clean -j && \
#     CPPFLAGS+=" -I/usr/include/gssapi" make USE_PGXS=1 -j

###############################################
# Second Stage: PostgreSQL
###############################################

FROM bitnami/postgresql:${PG_VERSION_MAJOR}-debian-11 as paradedb

ARG PG_VERSION_MAJOR=16

# Declare runtime environment variables
ENV PG_VERSION_MAJOR=${PG_VERSION_MAJOR}

# Copy third-party extensions from their builder stages
COPY --from=builder-pgvector /tmp/pgvector/*.so /opt/bitnami/postgresql/lib/
COPY --from=builder-pgvector /tmp/pgvector/*.control /opt/bitnami/postgresql/share/extension/
COPY --from=builder-pgvector /tmp/pgvector/sql/*.sql /opt/bitnami/postgresql/share/extension/
COPY --from=builder-pg_cron /tmp/pg_cron/*.so /opt/bitnami/postgresql/lib/
COPY --from=builder-pg_cron /tmp/pg_cron/*.control /opt/bitnami/postgresql/share/extension/
COPY --from=builder-pg_cron /tmp/pg_cron/*.sql /opt/bitnami/postgresql/share/extension/

# Copy the ParadeDB extensions from their builder stages
COPY --from=builder-pg_search /tmp/pg_search/target/release/pg_search-pg${PG_VERSION_MAJOR}/usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/* /opt/bitnami/postgresql/lib/
COPY --from=builder-pg_search /tmp/pg_search/target/release/pg_search-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/* /opt/bitnami/postgresql/share/extension/
COPY --from=builder-pg_analytics /tmp/pg_analytics/target/release/pg_analytics-pg${PG_VERSION_MAJOR}/usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/* /opt/bitnami/postgresql/lib/
COPY --from=builder-pg_analytics /tmp/pg_analytics/target/release/pg_analytics-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/* /opt/bitnami/postgresql/share/extension/

# COPY --from=builder-citus /tmp/citus/src/backend/distributed/*.so /opt/bitnami/postgresql/lib/
# COPY --from=builder-citus /tmp/citus/src/backend/distributed/*.control /opt/bitnami/postgresql/share/extension/
# COPY --from=builder-citus /tmp/citus/src/backend/distributed/sql/*.sql /opt/bitnami/postgresql/share/extension/
# COPY --from=builder-citus /tmp/citus/src/backend/columnar/*.so /opt/bitnami/postgresql/lib/
# COPY --from=builder-citus /tmp/citus/src/backend/columnar/*.control /opt/bitnami/postgresql/share/extension/
# COPY --from=builder-citus /tmp/citus/src/backend/columnar/sql/*.sql /opt/bitnami/postgresql/share/extension/

# Install runtime dependencies (requires switching to root temporarily)
USER root
RUN install_packages curl autoconf uuid-runtime git libpq5 liblz4-dev libzstd-dev libkrb5-dev libcurl4-gnutls-dev libicu-dev libkrb5-dev liblz4-dev libpam0g-dev libreadline-dev libselinux1-dev libssl-dev libxslt1-dev libzstd-dev make uuid-dev software-properties-common ca-certificates build-essential gnupg curl git make gcc clang pkg-config postgresql-server-dev-all 



# Citus installs properly if I do this, so it must be missing some files when copying over

# Build the extension
WORKDIR /tmp
RUN git clone --branch v12.1.3 https://github.com/citusdata/citus.git
WORKDIR /tmp/citus
RUN ./configure --without-libcurl && \
    make clean -j && \
    CPPFLAGS+=" -I/usr/include/gssapi" make USE_PGXS=1 -j && \
    make install


USER 1001



# Copy ParadeDB bootstrap script to install extensions, configure postgresql.conf, etc.
COPY ./docker/01_bootstrap.sh /docker-entrypoint-initdb.d/

# Configure shared_preload_libraries
# Note: pgaudit is needed here as it comes pre-packaged in the Bitnami image
ENV POSTGRESQL_SHARED_PRELOAD_LIBRARIES="citus,pgaudit,pg_cron,pg_search,pg_analytics"
# ENV POSTGRESQL_SHARED_PRELOAD_LIBRARIES="pgaudit,pg_cron,pg_search,pg_analytics"

# We set a default password to enable users to get started quickly, as it is a required
# environment variable for the Bitnami image.
#
# Note: This *will* (and should) get overriden by passing the POSTGRESQL_PASSWORD environment
# varialbe via `docker-compose` or `docker run -e POSTGRESQL_PASSWORD`. All production usage
# should use a unique, secure password and not rely on the default settings.
ENV POSTGRESQL_PASSWORD=postgres
