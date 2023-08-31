###############################################
# First Stage: Builder
###############################################

FROM rust:latest as builder

ARG PG_VERSION_MAJOR
ARG PG_BM25_VERSION

# Install the necessary build tools and PostgreSQL development files
RUN apt-get update \
    && apt-get install -y \
        build-essential \
        wget \
        gnupg \
        libclang-dev \
        clang \
        postgresql-${PG_VERSION_MAJOR} \
        postgresql-server-dev-${PG_VERSION_MAJOR} \
    && rm -rf /var/lib/apt/lists/*

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

# Check the PostgreSQL installation
RUN pg_config --version

# Initialize pgrx
RUN cargo pgrx init --pg${PG_VERSION_MAJOR}=/usr/lib/postgresql/${PG_VERSION_MAJOR}/bin/pg_config

# Build the extension
RUN cargo pgrx package

###############################################
# Second Stage: PostgreSQL
###############################################

FROM postgres:latest

ARG PG_VERSION_MAJOR
ARG PG_BM25_VERSION

# Set the working directory
WORKDIR /usr/src/app

# Copy the built extension from the builder stage
# Copy the control file and shared library from the builder stage
COPY --from=builder /usr/src/app/target/release/pg_bm25-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/pg_bm25.control /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder /usr/src/app/target/release/pg_bm25-pg${PG_VERSION_MAJOR}/usr/share/postgresql/${PG_VERSION_MAJOR}/extension/pg_bm25--${PG_BM25_VERSION}.sql /usr/share/postgresql/${PG_VERSION_MAJOR}/extension/
COPY --from=builder /usr/src/app/target/release/pg_bm25-pg${PG_VERSION_MAJOR}/usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/pg_bm25.so /usr/lib/postgresql/${PG_VERSION_MAJOR}/lib/

# Copy the entrypoint script into the container
COPY ./entrypoint.sh /usr/src/app

# Make the entrypoint script executable
RUN chmod +x /usr/src/app/entrypoint.sh

# Set the entrypoint script
ENTRYPOINT ["/usr/src/app/entrypoint.sh"]
