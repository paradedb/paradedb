# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Overview

ParadeDB is a PostgreSQL extension (`pg_search`) that provides full-text search using the BM25 algorithm. Built on Tantivy (Rust-based Lucene alternative) and pgrx, it offers a modern Elasticsearch alternative directly in Postgres.

**Key Architecture:**

- `pg_search` is a PostgreSQL extension written in Rust using pgrx 0.16.1 (exact version pinned)
- Built on top of Tantivy (Rust alternative to Apache Lucene) for full-text search with BM25 ranking
- Supports PostgreSQL 14+ (all versions supported by PostgreSQL Global Development Group)
- Default development version is PostgreSQL 18

## Essential Commands

### Development Setup

```bash
# Install Rust toolchain (required)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup install stable

# Install PostgreSQL (example for macOS)
brew install postgresql@18

# Install and initialize pgrx (exact version required)
cargo install --locked cargo-pgrx --version 0.16.1

# Initialize pgrx (macOS arm64 example)
cargo pgrx init --pg18=/opt/homebrew/opt/postgresql@18/bin/pg_config

# For other platforms, see pg_search/README.md
```

### Running the Extension

```bash
# Start pgrx development environment
cargo pgrx run

# In the PostgreSQL shell that opens:
# CREATE EXTENSION pg_search;
```

### Building

```bash
# Build and package the extension
make package

# Or using cargo directly
cargo pgrx package --package pg_search --pg-config $(which pg_config)

# Install into your local Postgres
cargo pgrx install --package pg_search --release --pg-config $(which pg_config)
```

### Testing

```bash
# Set up DATABASE_URL (required for tests)
# Create a .env file with:
# DATABASE_URL=postgres://YOUR_USERNAME@localhost:28817/pg_search
# Port is 28800 + postgres version (e.g., 28817 for PG17, 28818 for PG18)

# Run all tests
cargo test --package tests

# Run a specific test
cargo test --package tests --test <testname>
```

### Stress Testing

```bash
cd stressgres

# Interactive TUI mode
cargo run -- ui suites/vanilla-postgres.toml

# Headless mode with logging
cargo run -- headless suites/vanilla-postgres.toml --runtime=300000 --log-file=logs/test.log
```

### Code Quality

```bash
# Install pre-commit hooks
pre-commit install

# Run pre-commit checks manually
pre-commit run --all-files
```

## Architecture Overview

`pg_search` is a PostgreSQL extension that provides full-text search capabilities using BM25, built on top of Tantivy (a Rust-based alternative to Apache Lucene) and implemented using pgrx.

### Core Components

**`pg_search/src/api/`** - Public SQL API

- `operator.rs` - The `@@@` search operator and related functions
- `admin.rs` - Administrative functions for index management
- `tokenizers/` - Tokenizer configurations (language-specific tokenization)
- `builder_fns/` - Helper functions for building search queries

**`pg_search/src/index/`** - Tantivy Index Management

- `directory/` - Index storage backends
- `reader/` - Index reading and search operations
- `writer/` - Index writing and maintenance
- `merge_policy.rs` - Controls how index segments are merged

**`pg_search/src/postgres/`** - PostgreSQL Integration Layer

- `customscan/` - Custom scan node implementation
  - `basescan/` - Basic search scan implementation
  - `aggregatescan/` - Aggregation and filtering with search
  - `hook.rs` - Query planner integration
  - `qual_inspect.rs` - Query qualification analysis
  - `projections.rs` - Column projection handling
- `storage/` - Storage layer integration with Postgres
- `build.rs`, `build_parallel.rs` - Index building (sequential and parallel)
- `insert.rs`, `delete.rs` - Data modification handlers
- `options.rs` - Index options and configuration
- `types.rs`, `types_arrow.rs` - Type conversion between Postgres and Tantivy

**`pg_search/src/query/`** - Query Parsing and Execution

- `builder/` - Query builder implementations
- `pdb_query.rs` - ParadeDB query DSL
- `estimate_tree.rs` - Query cost estimation

**`pg_search/src/schema/`** - Index Schema Configuration

- Maps Postgres column types to Tantivy field types
- Handles tokenizer and normalizer configuration

**`pg_search/src/parallel_worker/`** - Background Workers

- Manages background worker processes for parallel operations
- Message queue for inter-process communication

**`tests/`** - Integration Tests

- Comprehensive test suite using `cargo test`
- Tests are in `tests/tests/*.rs`

**`stressgres/`** - Stress Testing Tool

- TUI and headless modes for load testing
- Suites defined in `stressgres/suites/`

**`tokenizers/`** - Language Tokenizers

- Optional ICU tokenizer for Arabic, Amharic, Czech, Greek

**`benchmarks/`** - Performance Benchmarks

- BM25 search benchmarks
- Query performance testing

### How It Works

1. **Extension Initialization** (`lib.rs:_PG_init`):
   - Registers custom scan nodes with PostgreSQL planner
   - Initializes configuration (GUCs)
   - Sets up background worker hooks

2. **Index Creation** (`postgres/build.rs`, `postgres/build_parallel.rs`):
   - When `CREATE INDEX USING bm25` is executed
   - Tantivy index is created in `$PGDATA/base/paradedb/`
   - Can use parallel workers for faster building

3. **Query Execution** (`postgres/customscan/`):
   - Query planner hook intercepts queries with `@@@` operator
   - Creates custom scan node for Tantivy search
   - Custom scan executor reads from Tantivy index
   - Results merged with heap data

4. **Data Modification** (`postgres/insert.rs`, `postgres/delete.rs`):
   - INSERT/UPDATE/DELETE operations trigger index updates
   - Background worker processes handle writes asynchronously

### Important Configuration

**For PostgreSQL < 17**, `pg_search` MUST be in `shared_preload_libraries`:

```
shared_preload_libraries = 'pg_search'
```

This enables the background worker process for index writes. Without this, database connections will crash or hang when creating a `pg_search` index.

**For PostgreSQL >= 17**, this is not required as the extension uses new capabilities.

### Version Pinning

- **pgrx version**: `=0.16.1` (exact version required)
- **PostgreSQL support**: 14+ (all currently supported versions)
- **Default development version**: PostgreSQL 18

### Dependencies

The extension uses a **forked version of Tantivy** (`paradedb/tantivy`) with custom patches for ParadeDB's needs.

## Development Tips

- **Read before modifying**: Always read files before editing to understand existing patterns
- **Test coverage required**: PRs must include tests for new features
- **Documentation required**: Update `docs/` for new features
- **Pre-commit hooks**: Install with `pre-commit install`
- **Conventional commits**: PR titles must follow conventional commit spec

## Contributing

- All PRs must be associated with a GitHub issue
- Comment `/take` on an issue to self-assign
- Issues labeled `good first issue` are ideal for new contributors
- CLA signature required (automated via CLA Assistant)

## License

AGPL-3.0 with commercial licensing available
