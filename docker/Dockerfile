###############################################
# Arguments
###############################################

ARG PG_VERSION_MAJOR
ARG PG_BM25_VERSION
ARG PG_IVM_VERSION
ARG PG_GRAPHQL_VERSION
ARG PGTAP_VERSION

###############################################
# First Stage: Builder
###############################################

FROM rust:latest as builder

ARG PG_VERSION_MAJOR

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    checkinstall \
    clang \
    postgresql-${PG_VERSION_MAJOR} \
    postgresql-server-dev-${PG_VERSION_MAJOR} \
    && rm -rf /var/lib/apt/lists/*

####################
# 0. pg_bm25
####################

FROM builder as builder-pg_bm25

ARG PG_VERSION_MAJOR
ARG PG_BM25_VERSION

# Set the PGX_HOME environment variable
ENV PGX_HOME=/usr/lib/postgresql/${PG_VERSION_MAJOR}

# Install cargo-pgrx
RUN cargo install cargo-pgrx

# Set the working directory
WORKDIR /usr/src/app

# Copy the pg_bm25 directory contents into the container at /usr/src/app
COPY pg_bm25/ /usr/src/app

# Use the build argument to update the version in Cargo.toml
RUN sed -i "s/version = \"[0-9]*\.[0-9]*\.[0-9]*\"/version = \"${PG_BM25_VERSION}\"/" Cargo.toml

# Initialize pgrx
RUN cargo pgrx init --pg${PG_VERSION_MAJOR}=/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config

# Build the extension
RUN cargo pgrx package

####################
# 1. pg_ivm
####################

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

###############################################
# Second Stage: PostgreSQL
###############################################

FROM postgres:latest

ARG PG_VERSION_MAJOR
ARG PG_BM25_VERSION
ARG PG_IVM_VERSION
ARG PG_GRAPHQL_VERSION
ARG PGTAP_VERSION

# Set the working directory
WORKDIR /usr/src/app

# Copy the built extension from the builder stage
# Copy the control file and shared library from the builder stage
COPY --from=builder-pg_bm25 /usr/src/app/target/release/pg_bm25-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/pg_bm25.control /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder-pg_bm25 /usr/src/app/target/release/pg_bm25-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/pg_bm25--${PG_BM25_VERSION}.sql /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder-pg_bm25 /usr/src/app/target/release/pg_bm25-pg${PG_VERSION_MAJOR}/usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/pg_bm25.so /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/

# Load extensions
COPY --from=builder-pg_ivm /tmp/*.deb /tmp/
COPY --from=builder-pg_graphql /tmp/*.deb /tmp/
COPY --from=builder-pgtap /tmp/*.deb /tmp/

# Install the Postgres extensions
RUN apt-get update && apt-get install -y --no-install-recommends \
    /tmp/*.deb \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/* /tmp/*

# Copy the entrypoint script into the container
COPY ./entrypoint.sh /usr/src/app

# Make the entrypoint script executable
RUN chmod +x /usr/src/app/entrypoint.sh

# Set the entrypoint script
ENTRYPOINT ["/usr/src/app/entrypoint.sh"]
