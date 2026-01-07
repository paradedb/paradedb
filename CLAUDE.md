# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

This is the core ParadeDB repository containing `pg_search`, a PostgreSQL extension for full-text search using the BM25 algorithm. It's built with Rust using pgrx and Tantivy (a Rust-based Lucene alternative).

## Extension Architecture

### Code Organization

- **`pg_search/`** - Main extension crate
  - **`src/api/`** - SQL API layer
    - `admin.rs` - Index administration functions (create_bm25, drop_bm25, etc.)
    - `operator.rs` - Search operator implementations (`@@@`, `@>`, etc.)
    - `tokenizers/` - Language-specific tokenizers
    - `builder_fns/` - Query builder functions
  - **`src/index/`** - Tantivy index management
    - `directory/` - Custom directory implementation for Postgres storage
    - `reader/` - Index reader with caching
    - `writer/` - Index writer with transaction support
    - `merge_policy.rs` - Custom merge strategies for Tantivy segments
  - **`src/postgres/`** - Postgres integration
    - `customscan/` - Custom scan node for query execution
    - `storage/` - Integration with Postgres storage layer
  - **`src/query/`** - Query DSL and execution
  - **`src/schema/` **- Index schema and field configuration
  - **`src/parallel_worker/`** - Background worker process
  - **`src/gucs.rs`** - PostgreSQL GUC (configuration) parameters

- **`tests/`** - Integration tests using pgrx test framework
- **`tokenizers/`** - Shared tokenizer implementations
- **`benchmarks/`** - Performance benchmarks
- **`stressgres/`** - Stress testing tool
- **`macros/`** - Procedural macros

### Key Architectural Concepts

**Custom Scan Integration:** The extension hooks into Postgres query planning via custom scan nodes (`src/postgres/customscan/hook.rs`). This allows BM25 searches to be optimized and pushed down into the scan.

**Background Worker:** A parallel worker process (`src/parallel_worker/`) handles index writes asynchronously, which is why `shared_preload_libraries = 'pg_search'` is required in postgresql.conf for Postgres < 17.

**Storage Layer:** Tantivy indexes are stored using a custom directory implementation that integrates with Postgres MVCC and WAL.

## Development Commands

### Setup

```bash
# Install pgrx (exact version match required)
cargo install --locked cargo-pgrx --version 0.16.1

# Initialize for your Postgres version (example for PG17 on macOS arm64)
cargo pgrx init --pg17=/opt/homebrew/opt/postgresql@17/bin/pg_config

# For pgvector (required for hybrid search tests)
git clone --branch v0.8.1 https://github.com/pgvector/pgvector.git
cd pgvector/
PG_CONFIG=/opt/homebrew/opt/postgresql@17/bin/pg_config make
sudo PG_CONFIG=/opt/homebrew/opt/postgresql@17/bin/pg_config make install
```

### Running

```bash
# Start development environment (from paradedb/ directory)
cargo pgrx run

# In the psql shell that opens:
CREATE EXTENSION pg_search;

# Test queries
SELECT * FROM search_idx.search('body:hello', limit_rows => 10);
```

### Building

```bash
# Package for distribution
cargo pgrx package --package pg_search --pg-config $(which pg_config)

# Or use Makefile
make package

# Install to local Postgres
cargo pgrx install --package pg_search --release --pg-config $(which pg_config)
```

### Testing

```bash
# Create .env file with DATABASE_URL
# Format: postgres://USERNAME@localhost:PORT/pg_search
# PORT = 28800 + postgres_version (e.g., 28817 for PG17)
echo "DATABASE_URL=postgres://$(whoami)@localhost:28817/pg_search" > .env

# Run all tests
cargo test --package tests

# Run specific test file
cargo test --package tests --test schema

# With ICU tokenizer support
cargo pgrx run --features icu
cargo test --package tests --features icu
```

### Stress Testing

```bash
cd stressgres

# Interactive mode
cargo run -- ui suites/read-write.toml

# Headless with logging
cargo run -- headless suites/read-write.toml --runtime=300000 --log-file=output.log

# Available suites in suites/:
# - vanilla-postgres.toml - Baseline Postgres (no pg_search)
# - read-write.toml - Mixed workload
# - logical-replication.toml - Replication testing
# - many-updates.toml - Update-heavy workload
```

## Development Workflow

### Modifying the Extension

1. Make code changes
2. Run `cargo pgrx run` to recompile
3. In psql: `DROP EXTENSION pg_search; CREATE EXTENSION pg_search;`
4. Test your changes

### Adding New SQL Functions

1. Add function in appropriate `src/api/*.rs` file with `#[pg_extern]` attribute
2. Rebuild with `cargo pgrx run`
3. Add tests in `tests/`

### Modifying Index Behavior

Index-related changes typically involve:
- `src/index/writer/` - Write operations
- `src/index/reader/` - Read operations
- `src/schema/` - Field schema changes
- `src/postgres/customscan/` - Query execution changes

## Testing Guidelines

- All new features require integration tests in `tests/`
- Tests use the pgrx testing framework with sqllogictest-style assertions
- Set `DATABASE_URL` environment variable before running tests
- Tests automatically start a dedicated Postgres instance

## Important Configuration

### postgresql.conf Requirements

For PostgreSQL < 17:
```
shared_preload_libraries = 'pg_search'
```

This is CRITICAL - without it, the background worker won't start and the database will crash when creating indexes.

### ICU Tokenizer (Optional)

Enables Arabic, Amharic, Czech, Greek tokenization:

```bash
# macOS
brew install icu4c
export PKG_CONFIG_PATH="/opt/homebrew/opt/icu4c/lib/pkgconfig"

# Build with ICU
cargo pgrx run --features icu
```

## Debugging

### Enable Detailed Logging

```bash
# Set environment variable before running
export RUST_LOG=pg_search=debug
cargo pgrx run
```

### Block Tracking (Memory Debugging)

```bash
# Enable block tracker feature
cargo pgrx run --features block_tracker
```

## Common Issues

**Database crashes on CREATE INDEX:** Check that `shared_preload_libraries` includes `pg_search`

**Tests fail with connection errors:** Verify `DATABASE_URL` in `.env` file and port matches your Postgres version (28800 + version)

**Build errors with ICU:** Ensure `PKG_CONFIG_PATH` is set and libicu is installed

**pgrx version mismatch:** This project requires exactly version 0.16.1 - install with `cargo install --locked cargo-pgrx --version 0.16.1`

## Version Support

- PostgreSQL: 14, 15, 16, 17, 18
- Default development version: PostgreSQL 17
- pgrx: 0.16.1 (exact match required)
- Rust: stable toolchain

## License

AGPL-3.0 with commercial licensing available
