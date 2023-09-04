###############################################
# First Stage: Base
###############################################

FROM ubuntu:22.04 as base

ARG PG_VERSION_MAJOR

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

ENV DEBIAN_FRONTEND=noninteractive
ENV TZ=UTC

RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    wget \
    sudo \
    gnupg \
    software-properties-common \
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
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:$PATH"

# Set the PGX_HOME environment variable
ENV PGX_HOME=/usr/lib/postgresql/${PG_VERSION_MAJOR}

# Install cargo-pgrx
RUN cargo install cargo-pgrx

# ####################
# # 0. pg_bm25
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
# 1. pg_ivm
######################

FROM builder as builder-pg_ivm

ARG PG_IVM_VERSION

ADD "https://github.com/sraoss/pg_ivm/archive/refs/tags/v${PG_IVM_VERSION}.tar.gz" /tmp/pg_ivm.tar.gz

RUN tar -xvf /tmp/pg_ivm.tar.gz -C /tmp

WORKDIR /tmp/pg_ivm-${PG_IVM_VERSION}

RUN make -j && checkinstall -D --nodoc --install=no --fstrans=no --backup=no --pakdir=/tmp

####################
# 2. pg_graphql
####################

FROM builder as builder-pg_graphql

ARG PG_VERSION_MAJOR
ARG PG_GRAPHQL_VERSION
ARG TARGETARCH

ADD "https://github.com/supabase/pg_graphql/releases/download/v${PG_GRAPHQL_VERSION}/pg_graphql-v${PG_GRAPHQL_VERSION}-pg${PG_VERSION_MAJOR}-$TARGETARCH-linux-gnu.deb" /tmp/pg_graphql.deb

####################
# 3. pgtap
####################

FROM builder as builder-pgtap

ARG PGTAP_VERSION

ADD "https://github.com/theory/pgtap/archive/refs/tags/v${PGTAP_VERSION}.tar.gz" /tmp/pgtap.tar.gz

RUN tar -xvf /tmp/pgtap.tar.gz -C /tmp

WORKDIR /tmp/pgtap-${PGTAP_VERSION}

RUN make -j && checkinstall -D --nodoc --install=no --fstrans=no --backup=no --pakdir=/tmp

####################
# 4. pgvector
####################

FROM builder as builder-pgvector

ARG PGVECTOR_VERSION

ADD "https://github.com/pgvector/pgvector/archive/refs/tags/v${PGVECTOR_VERSION}.tar.gz" /tmp/pgvector.tar.gz

RUN tar -xvf /tmp/pgvector.tar.gz -C /tmp

WORKDIR /tmp/pgvector-${PGVECTOR_VERSION}

RUN make -j && checkinstall -D --nodoc --install=no --fstrans=no --backup=no --pakdir=/tmp

####################
# 5. pgml
####################

FROM builder as builder-pgml

ARG PG_VERSION_MAJOR
ARG PGML_VERSION

# TODO is lsb-release needed?
# TODO: can probably install python only once in base instead
RUN apt-get update && apt-fast install -y --no-install-recommends \
    libopenblas-dev \
    libssl-dev \
    bison \
    flex \
    cmake \
    libreadline-dev \
    libz-dev \
    lsb-release \
    tzdata \
    libpq-dev \
    libclang-dev \
    python3.10 \
    python3-pip \
    libpython3.10-dev \
    python3.10-dev \
    lld \
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
ARG PG_VERSION_MINOR
ARG PG_BM25_VERSION
ARG PG_IVM_VERSION
ARG PG_GRAPHQL_VERSION
ARG PGTAP_VERSION
ARG PGVECTOR_VERSION
ARG PGML_VERSION

# Set the working directory
WORKDIR /usr/src/app

RUN apt-get update && apt-fast install -y --no-install-recommends \
    ca-certificates \
    libgomp1 \
    libopenblas-dev \
    python3.10 \
    python3-pip \
    libpython3.10-dev \
    python3.10-dev \
    && rm -rf /var/lib/apt/lists/*

# Install pgml runtime dependencies
RUN pip3 install --no-cache-dir xgboost lightgbm scikit-learn

# # Copy the pg_bm25 extension from the builder stage
COPY --from=builder-pg_bm25 /usr/src/app/target/release/pg_bm25-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/* /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder-pg_bm25 /usr/src/app/target/release/pg_bm25-pg${PG_VERSION_MAJOR}/usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/* /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/

# # Copy the pgml extension from the builder stage
COPY --from=builder-pgml /tmp/postgresml/pgml-extension/target/release/pgml-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/* /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder-pgml /tmp/postgresml/pgml-extension/target/release/pgml-pg${PG_VERSION_MAJOR}/usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/* /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/

# Load extensions
COPY --from=builder-pg_ivm /tmp/*.deb /tmp/
COPY --from=builder-pg_graphql /tmp/*.deb /tmp/
COPY --from=builder-pgtap /tmp/*.deb /tmp/
COPY --from=builder-pgvector /tmp/*.deb /tmp/

# Install the Postgres extensions
RUN apt-get update && apt-fast install -y --no-install-recommends \
    /tmp/*.deb \
    && rm -rf /var/lib/apt/lists/* /tmp/*

# Copy the database initialization script into the container
COPY ./docker/postgresql.conf /etc/postgresql/${PG_VERSION_MAJOR}/main/postgresql.conf
COPY ./docker/pg_hba.conf /etc/postgresql/${PG_VERSION_MAJOR}/main/pg_hba.conf
COPY ./docker/pg_ident.conf /etc/postgresql/${PG_VERSION_MAJOR}/main/pg_ident.conf

# Copy the entrypoint script into the container
COPY ./entrypoint.sh /usr/src/app

# Make the entrypoint script executable
# TODO: can i remove this?
RUN chmod +x /usr/src/app/entrypoint.sh

# Expose the PostgreSQL port
# TODO: can i remove this?
EXPOSE 5432

# Set the entrypoint script
ENTRYPOINT ["/usr/src/app/entrypoint.sh"]
