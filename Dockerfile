# First Stage: Builder
FROM rust:latest as builder

# TODO don't hardcode the versions for Postgres and for the extension, so this doesn't break later

# Install the necessary build tools and PostgreSQL development files
RUN apt-get update \
    && apt-get install -y \
        build-essential \
        wget \
        gnupg \
        libclang-dev \
        clang \
        postgresql-15 \
        postgresql-server-dev-15 \
    && rm -rf /var/lib/apt/lists/*

# Set the PGX_HOME environment variable
ENV PGX_HOME=/usr/lib/postgresql/15

# Install cargo-pgrx
RUN cargo install cargo-pgrx

# Set the working directory
WORKDIR /usr/src/app

# Copy the pg_bm25 directory contents into the container at /usr/src/app
COPY pg_bm25/ /usr/src/app

# Check the PostgreSQL installation
RUN pg_config --version

# Initialize pgrx
RUN cargo pgrx init --pg15=/usr/lib/postgresql/15/bin/pg_config

# Build the extension
RUN cargo pgrx package

# Second Stage: PostgreSQL
FROM postgres:latest

# Set the working directory
WORKDIR /usr/src/app

# Copy the built extension from the builder stage
# Copy the control file and shared library from the builder stage
COPY --from=builder /usr/src/app/target/release/pg_bm25-pg15/usr/share/postgresql/15/extension/pg_bm25.control /usr/share/postgresql/15/extension/
COPY --from=builder /usr/src/app/target/release/pg_bm25-pg15/usr/share/postgresql/15/extension/pg_bm25--0.0.1.sql /usr/share/postgresql/15/extension/
COPY --from=builder /usr/src/app/target/release/pg_bm25-pg15/usr/lib/postgresql/15/lib/pg_bm25.so /usr/lib/postgresql/15/lib/

# Copy the entrypoint script into the container
COPY ./entrypoint.sh /usr/src/app

# Make the entrypoint script executable
RUN chmod +x /usr/src/app/entrypoint.sh

# Set the entrypoint script
ENTRYPOINT ["/usr/src/app/entrypoint.sh"]
