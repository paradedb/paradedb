ARG PG_VERSION_MAJOR=16

###############################################
# First Stage: Base
###############################################

FROM postgres:${PG_VERSION_MAJOR}-bookworm as base

# Declare all the build arguments and set them to
# environment variables for use in build and runtime
ARG PG_VERSION_MAJOR
ARG POSTHOG_API_KEY
ARG POSTHOG_HOST
ARG COMMIT_SHA
ARG TELEMETRY
ARG BUILD_DATE
ARG RUST_VERSION=1.73.0
ARG PGRX_VERSION=0.11.2

ENV PG_VERSION_MAJOR=${PG_VERSION_MAJOR} \
    RUST_VERSION=${RUST_VERSION} \
    PGRX_VERSION=${PGRX_VERSION} \
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
    software-properties-common \
    ca-certificates \
    uuid-runtime \
    libssl-dev \
    curl \
    wget \
    && rm -rf /var/lib/apt/lists/*


###############################################
# Second Stage: Builder
###############################################

FROM base as builder

SHELL ["/bin/bash", "-o", "pipefail", "-c", "-e"]

# Add PostgreSQL's third party repository to get the latest versions
# also add apt-fast repository to speed up the build
RUN <<EOF
curl -o /usr/share/keyrings/postgresql.asc -s https://www.postgresql.org/media/keys/ACCC4CF8.asc
echo "deb [signed-by=/usr/share/keyrings/postgresql.asc] http://apt.postgresql.org/pub/repos/apt/ $(lsb_release -cs)-pgdg main" | tee /etc/apt/sources.list.d/pgdg.list

curl -o /usr/share/keyrings/apt-fast.asc -s https://keyserver.ubuntu.com/pks/lookup?op=get\&search=0xA2166B8DE8BDC3367D1901C11EE2FF37CA8DA16B
echo "deb [signed-by=/usr/share/keyrings/apt-fast.asc] http://ppa.launchpad.net/apt-fast/stable/ubuntu jammy main" | tee /etc/apt/sources.list.d/apt-fast.list
EOF

RUN <<EOF
apt-get update
apt-get install --no-install-recommends apt-fast -y

apt-fast update
apt-fast install -y --no-install-recommends \
    build-essential \
    clang \
    git \
    cmake \
    pkg-config \
    liblz4-dev \
    libcurl4-openssl-dev \
    libopenblas-dev \
    python3-dev \
    python3-pip \
    gnupg \
    gcc \
    make \
    "postgresql-server-dev-${PG_VERSION_MAJOR}"

rm -rf /var/lib/apt/lists/*
EOF

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    && /root/.cargo/bin/rustup default "${RUST_VERSION}"

ENV PATH="/root/.cargo/bin:$PATH" \
    PGX_HOME=/usr/lib/postgresql/${PG_VERSION_MAJOR}

RUN cargo install cargo-get
RUN cargo install --locked cargo-pgrx --version "${PGRX_VERSION}" && \
    cargo pgrx init "--pg${PG_VERSION_MAJOR}=/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config"

# debmake needs these to be set
ENV DEBEMAIL="support@paradedb.com"
ENV DEBFULLNAME="ParadeDB"

######################
# pg_bm25
######################

FROM builder as builder-pg_bm25

COPY pg_bm25/ /tmp/pg_bm25
COPY shared/ /tmp/shared

WORKDIR /tmp/pg_bm25
ENV PKGDIR=/tmp/pg_bm25/deb-pkg

# Use the build argument to update the version in Cargo.toml
RUN cargo pgrx package --features icu --pg-config "/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config" --out-dir $PKGDIR

WORKDIR $PKGDIR

RUN <<EOF
mkdir -p DEBIAN

cat <<EOT | sed 's/^ *//' | tee DEBIAN/control
    Package: $(cargo get package.name | sed 's/_/-/g')
    Description: $(cargo get package.description)
    Maintainer: $DEBFULLNAME <$DEBEMAIL>
    Version: $(cargo get package.version)
    Architecture: $(dpkg --print-architecture)
EOT

dpkg-deb --build $PKGDIR /tmp/pg-bm25.deb
EOF

######################
# pg_analytics
######################

FROM builder as builder-pg_analytics

WORKDIR /tmp/pg_analytics
ENV PKGDIR=/tmp/pg_analytics/deb-pkg

COPY pg_analytics/ /tmp/pg_analytics
COPY shared/ /tmp/shared

# Use the build argument to update the version in Cargo.toml
# Note: We require Rust nightly to build pg_analytics with SIMD
RUN rustup update nightly && \
    rustup override set nightly && \
    cargo install --locked cargo-pgrx --version 0.11.2 --force && \
    cargo pgrx package --pg-config "/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config" --out-dir $PKGDIR

WORKDIR $PKGDIR

RUN <<EOF
mkdir -p DEBIAN
cat <<EOT | sed 's/^ *//' | tee DEBIAN/control
    Package: $(cargo get package.name | sed 's/_/-/g')
    Description: $(cargo get package.description)
    Maintainer: $DEBFULLNAME <$DEBEMAIL>
    Version: $(cargo get package.version)
    Architecture: $(dpkg --print-architecture)
EOT

dpkg-deb --build $PKGDIR /tmp/pg-analytics.deb
EOF

######################
# pg_sparse
######################
FROM builder as builder-pg_sparse

WORKDIR /tmp/pg_sparse
COPY pg_sparse/ /tmp/pg_sparse

ENV PKGDIR=/tmp/pg_sparse/deb-pkg
ENV PKGNAME=pg-sparse

RUN make clean && \
    make USE_PGXS=1 OPTFLAGS="" -j && \
    make USE_PGXS=1 DESTDIR="$PKGDIR" install -j

WORKDIR $PKGDIR

RUN <<EOF
mkdir -p DEBIAN

cat <<EOT | sed 's/^ *//' | tee DEBIAN/control
    Package: pg-sparse
    Maintainer: $DEBFULLNAME <$DEBEMAIL>
    Version: $(awk -F "= '|\'" '/default_version/ {print $2}' ../svector.control)
    Architecture: $(dpkg --print-architecture)
EOT

dpkg-deb --build $PKGDIR /tmp/pg-sparse.deb
EOF

######################
# additional extensions
######################

FROM builder as builder-ext-misc

WORKDIR /tmp
RUN apt-get update && apt-get download "postgresql-${PG_VERSION_MAJOR}-pgvector"

###############################################
# Third Stage: PostgreSQL
###############################################

FROM base as paradedb

ARG TARGETARCH

# Needed for publishing to ArtifactHub
LABEL org.opencontainers.image.created="$BUILD_DATE"
LABEL org.opencontainers.image.description="PostgreSQL for search and analaytics"
LABEL io.artifacthub.package.readme-url="https://github.com/paradedb/paradedb/blob/main/README.md"

# Copy third party extensions
COPY --from=builder-ext-misc /tmp/*.deb /tmp/extensions-deb/

# Copy the ParadeDB extensions from their builder stages
COPY --from=builder-pg_sparse /tmp/pg-sparse*.deb /tmp/extensions-deb/
COPY --from=builder-pg_bm25 /tmp/pg-bm25*.deb /tmp/extensions-deb/
COPY --from=builder-pg_analytics /tmp/pg-analytics*.deb /tmp/extensions-deb/

RUN dpkg -i /tmp/extensions-deb/*.deb \
    && rm -rf /tmp/extensions-deb

# Copy entrypoint script, which will be handled by the official image initialization scipt
COPY ./docker/entrypoint.sh /docker-entrypoint-initdb.d/10_paradedb.sh

# Change the uid of postgres to 26
RUN usermod -u 26 postgres \
    && chown -R 26:999 /var/lib/postgresql \
    && chown -R 26:999 /var/run/postgresql \
    && chmod -R 700 /var/lib/postgresql

USER 26