# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

ParadeDB is a Postgres extension that enables full-text search using the BM25 algorithm. It's built on top of Tantivy (the Rust alternative to Apache Lucene) using pgrx. ParadeDB implements a custom Postgres index access method and custom scan providers to integrate search and analytics capabilities seamlessly with SQL.

## Common Development Commands

### Build and Installation

```bash
# Install the specific version of pgrx (must match Cargo.toml)
cargo install --locked cargo-pgrx --version 0.16.1

# Initialize pgrx with your Postgres version (replace --pg18 with your version)
# macOS arm64
cargo pgrx init --pg18=/opt/homebrew/opt/postgresql@18/bin/pg_config
# Ubuntu
cargo pgrx init --pg18=/usr/lib/postgresql/18/bin/pg_config

# Build and package the extension
cargo pgrx package --package pg_search --pg-config <path-to-pg_config>

# Install into Postgres
cargo pgrx install --package pg_search --release --pg-config <path-to-pg_config>

# Run development Postgres with the current commit of the extension installed
cargo pgrx run
```

### Testing

```bash
# Set up DATABASE_URL (required for tests)
# Create .env file with:
# DATABASE_URL=postgres://<username>@localhost:<port>/pg_search
# Port = 28800 + postgres version (e.g., 28818 for PG18)

# Run all tests
cargo test --package tests

# Run a single test file (without .rs extension)
cargo test --package tests --test <testname>

# With verbose output
export RUST_BACKTRACE=1
cargo test --package tests

# Restart pgrx Postgres for testing
cargo pgrx stop --package pg_search
cargo pgrx install --package pg_search --pg-config <path>
cargo pgrx start --package pg_search
```

### Linting and Formatting

```bash
# Install pre-commit hooks (enforced by CI)
pre-commit install

# Run pre-commit checks manually
pre-commit run --all-files

# Format Rust code
cargo fmt

# Run clippy
cargo clippy
```

## Architecture

### Repository Structure

- **pg_search/**: Main Postgres extension implementing BM25 full-text search
  - Core modules: `postgres/`, `query/`, `schema/`, `index/`, `api/`
  - ~95 Rust source files organized into subsystems

- **tokenizers/**: Standalone tokenization library
  - Multi-language support: Jieba (Chinese), Lindera (Japanese/Korean), ICU, n-gram
  - Integrates with Tantivy's tokenizer API

- **macros/**: Procedural macros for code generation
  - Builder functions for search queries
  - SQL generation for tokenizers

- **tests/**: Integration test suite
  - Uses sqlx for Postgres integration
  - Tests cover BM25 search, joins, aggregations, mutations, sorting

- **benchmarks/**: Performance testing and micro-benchmarks

- **stressgres/**: TUI-based Postgres stress testing tool

### Core Architecture Layers

#### 1. Postgres Integration (`pg_search/src/postgres/`)

##### Index Access Method (IAM)

- Implements custom BM25 index handler via `bm25_handler()` function
- Registers custom scan strategies for text queries
- Manages index lifecycle: validate, build, insert, delete, vacuum, parallel operations

##### Custom Scan Execution Framework

Three custom scan types:

- **BaseScan**: Low-level index scans against BM25 indexes
- **AggregateScan**: Query-level aggregations pushed down to the index
- **JoinScan**: Join operations optimized for search indexes

##### Storage Subsystem (`postgres/storage/`)

- Uses Postgres heap blocks to store Tantivy index segments
- Block layout:
  - Blocks 0-1: Merge and cleanup locks
  - Block 2: Schema metadata (serialized Tantivy schema)
  - Block 3: Settings metadata
  - Block 4: Segment metadata entries
  - Blocks 5+: Actual segment component data

##### Parallel Query Support

- `ParallelScanState`: Shared state between leader and worker processes
- Work-stealing segment pool for dynamic load balancing
- Aggregation result merging across parallel workers

#### 2. Query & Search Layer (`pg_search/src/query/`)

##### SearchQueryInput Enum (Core Query DSL)

Main variants: All, Boolean, Boost, DisjunctionMax, ConstScore, Term, Range, Phrase, Regex, Fuzzy, Parse, etc.

##### Query Processing Pipeline

- `builder.rs`: Converts Postgres expressions to Tantivy queries
- `estimate_tree.rs`: Query cost estimation for planner
- `proximity/`: Proximity searches with phrase scoring
- `more_like_this.rs`: MLT (More Like This) functionality
- `heap_field_filter.rs`: Row visibility filtering for MVCC compliance

#### 3. Schema & Index Configuration (`pg_search/src/schema/`)

##### SearchFieldType Enum

Supported types: Text, Tokenized, Uuid, Inet, I64, F64, U64, Bool, Json, Date, Range

##### Field Configuration

Each field has a `SearchFieldConfig` specifying:

- Tokenizer type
- Normalization settings
- Storage options (stored, indexed, fast)
- Fast field settings for columnar storage

#### 4. MVCC and Visibility (`pg_search/src/index/mvcc.rs`)

- Respects Postgres transaction isolation
- Filters out invisible rows in parallel workers
- Handles HOT (Heap Only Tuple) chains

### Execution Flow

```text
SQL Query with @@@ operator
    ↓
Planner Hook Recognition
    ↓
CustomPath Creation (BaseScan/AggregateScan/JoinScan)
    ↓
Cost Estimation
    ↓
CustomScan Builder
    ↓
Execution:
  - Load BM25 index from storage blocks
  - Execute Tantivy query
  - Fetch rows from heap with MVCC checks
  - Project scores/snippets
  - Apply LIMIT/ORDER BY
    ↓
Return results to Postgres
```

### GUC Configuration

Key tunables in `pg_search/src/postgres/gucs.rs`:

- `enable_custom_scan`: Toggle index scans (default: true)
- `enable_aggregate_custom_scan`: Toggle aggregate pushdown (default: false)
- `enable_join_custom_scan`: Toggle join optimization (default: false)
- `enable_fast_field_exec`: Use columnar storage (default: true)
- `limit_fetch_multiplier`: TopN optimization factor (default: 1.0)
- Segment management settings

## Key Dependencies

- **pgrx**: Version 0.16.1 (must be exact match)
- **Tantivy**: Forked version with custom features (columnar compression, quickwit support)
  - Fork: `https://github.com/paradedb/tantivy.git`
  - Features: `columnar-zstd-compression`, `lz4-compression`, `quickwit`, `stemmer`, `stopwords`
- **PostgreSQL**: Supports versions 15+

## Important Configuration Requirements

### postgresql.conf

For Postgres versions < 17, `pg_search` MUST be in `shared_preload_libraries`:

```conf
shared_preload_libraries = 'pg_search'
```

This enables the background worker process for index writes. Without this, database connections will crash or hang when creating a BM25 index.

### Environment Variables for Development

```bash
# Required for tests
DATABASE_URL=postgres://<username>@localhost:<port>/pg_search

# Port calculation: 28800 + postgres_version
# Example for PG18: 28818

# Helpful for debugging
RUST_BACKTRACE=1
```

## Common Patterns

### Adding a New Tokenizer

1. Add tokenizer implementation in `tokenizers/src/`
2. Register in `tokenizers/src/manager.rs`
3. Add SQL generation macro in `macros/src/`
4. Update `SearchFieldConfig` to support the new tokenizer
5. Add tests in `tests/src/`

### Adding a New Query Type

1. Add variant to `SearchQueryInput` enum in `pg_search/src/query/mod.rs`
2. Implement conversion to Tantivy query in `pg_search/src/query/builder.rs`
3. Add cost estimation logic in `pg_search/src/query/estimate_tree.rs`
4. Add tests in `tests/src/`

### Modifying Index Storage Format

Changes to storage require careful handling:

1. Update block layout constants in `pg_search/src/postgres/storage/block.rs`
2. Update metadata serialization in `pg_search/src/postgres/storage/metadata.rs`
3. Consider migration path for existing indexes
4. Update vacuum and merge logic if needed

## Testing Strategy

- Integration tests use real Postgres instances via pgrx
- Tests are organized by feature (bm25, joins, aggregates, etc.)
- Use rstest for fixture-based testing
- pgvector must be installed for hybrid search tests

## Pull Request Requirements

- Follow [Conventional Commits](https://www.conventionalcommits.org/) for PR titles
- Add tests for new features (features without tests will not be merged)
- Add documentation in `docs/` for user-facing features
- Pre-commit hooks must pass
- Sign the CLA (enforced by CLA Assistant)

## Debugging Tips

- Use `cargo pgrx run` for interactive development with Postgres
- Check Postgres logs for custom scan execution details
- Use `EXPLAIN` to see query plans with custom scans
- Set `RUST_BACKTRACE=1` for detailed error traces
- For parallel query debugging, check worker process logs

## Critical Design Decisions

1. **Tantivy Integration**: Uses forked Tantivy with ParadeDB-specific features
2. **Postgres Block Storage**: Index data stored in Postgres blocks (not external files) for durability
3. **Parallel by Default**: Parallel workers claim segments dynamically
4. **MVCC-Aware**: Respects Postgres transaction isolation
5. **Hook-Based**: Minimal Postgres core modification; primarily uses planner hooks
