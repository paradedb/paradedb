# CLAUDE.md

This file provides guidance to Claude agents when working with code in this repository.

## Overview

ParadeDB is a Postgres extension that enables full-text search using the BM25 algorithm. It's built on top of Tantivy (the Rust alternative to Apache Lucene) using pgrx. ParadeDB implements a custom Postgres index access method and custom scan providers to integrate search and analytics capabilities seamlessly with SQL.

## Core Mandates

Follow these rules when working on this codebase:

- **Conventions**: Rigorously adhere to existing project conventions. Analyze surrounding code, tests, and configuration before making changes.
- **Libraries**: NEVER assume a library is available. Verify its usage in `Cargo.toml` before using it.
- **Style**: Mimic the style (formatting, naming), structure, and architectural patterns of existing code.
- **Comments**: Add comments sparingly. Focus on WHY something is done, not WHAT. Never use comments to communicate with the user.
- **Error Handling**: Use `pgrx::error!()` for fatal errors that abort transactions. Use `thiserror` for error types following patterns in `pg_search/src/query/mod.rs`.
- **Logging**: Use `pgrx::warning!()` for debug output (visible in regression test results). Use sparingly.
- **Do Not Revert**: Do not revert changes unless explicitly asked or if they caused an error.
- **Build Performance**: Do NOT use `--release` flag during development - it significantly slows compilation.

## Development Workflow

### Primary Workflow for Bug Fixes and Features

1. **Understand**: Read the issue, related code, and existing tests
2. **Plan**: Identify files to modify and test approach
3. **Implement**: Make changes following existing patterns
4. **Test**: Run regression tests, verify with EXPLAIN output
5. **Verify**: Run `pre-commit run --all-files` to check lints and formatting

### Build and Installation

```bash
# Install the specific version of pgrx (must match Cargo.toml)
cargo install --locked cargo-pgrx --version 0.16.1

# Initialize pgrx with your Postgres version (replace --pg18 with your version)
# macOS arm64
cargo pgrx init --pg18=/opt/homebrew/opt/postgresql@18/bin/pg_config
# Ubuntu
cargo pgrx init --pg18=/usr/lib/postgresql/18/bin/pg_config

# Build and install (DEBUG - use for development)
cargo pgrx install --package pg_search --pg-config <path-to-pg_config>


# Run development Postgres with the extension installed
cargo pgrx run
```

### Linting and Formatting

```bash
# Install pre-commit hooks (enforced by CI)
pre-commit install

# Run pre-commit checks manually (REQUIRED before commits)
pre-commit run --all-files

# Format Rust code
cargo fmt

# Run clippy
cargo clippy
```

## Testing

### Interactive SQL Development

Use `pg_search_run.sh` as an equivalent to `psql` with the extension loaded:

```bash
# Run SQL interactively
PGVER=18.1 ./scripts/pg_search_run.sh

# Run a specific SQL file
PGVER=18.1 ./scripts/pg_search_run.sh -f path/to/file.sql

# Example with full path
cd /path/to/paradedb && PGVER=18.1 ./scripts/pg_search_run.sh -f pg_search/tests/pg_regress/sql/my_test.sql
```

### Regression Tests (pg_regress)

SQL-based tests in `pg_search/tests/pg_regress/sql/`. Results go to `pg_search/tests/pg_regress/expected/`.

```bash
# Run a single regression test (without .sql extension)
cargo pgrx regress --package pg_search --resetdb --auto pg18 my_test_name

# Example: run the operators test
cargo pgrx regress --package pg_search --resetdb --auto pg18 operators
```

**Important**: The test fails if the result file changes, but this doesn't necessarily mean the test logic failed - review the diff to determine if the change is expected.

### Integration Tests (Rust)

```bash
# Run all integration tests using the helper script
PGVER=18.1 ./scripts/pg_search_test.sh

# Run a specific test file
PGVER=18.1 ./scripts/pg_search_test.sh --test sorting

# Or manually (requires DATABASE_URL to be set)
cargo test --package tests --package tokenizers

# Run a single test file
cargo test --package tests --test <testname>
```

### Environment Setup

```bash
# Required for tests - create .env file or export:
DATABASE_URL=postgres://<username>@localhost:<port>/pg_search

# Port calculation: 28800 + postgres_major_version
# PG15: 28815, PG16: 28816, PG17: 28817, PG18: 28818

# Enable verbose output
export RUST_BACKTRACE=1
```

### Writing Regression Tests

Follow the pattern in `pg_search/tests/pg_regress/sql/`:

```sql
\i common/common_setup.sql

-- Create test table and index
CALL paradedb.create_bm25_test_table(
    schema_name => 'public',
    table_name => 'mock_items'
);

CREATE INDEX idx_mock_items ON mock_items
    USING bm25 (id, description, category)
    WITH (key_field='id');

-- For each query: run EXPLAIN first, then the query itself
-- This ensures both the plan and results appear in output
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF) SELECT * FROM mock_items WHERE description @@@ 'shoes';
SELECT * FROM mock_items WHERE description @@@ 'shoes' ORDER BY id;

-- Always use ORDER BY for deterministic results
-- Always clean up
DROP TABLE mock_items;
```

Key conventions:

- Start with `\i common/common_setup.sql`
- Include both `EXPLAIN` and actual query for each test case
- Use `ORDER BY` for deterministic output
- Use `COSTS OFF, TIMING OFF` in EXPLAIN to avoid flaky output
- Clean up tables at the end

### Restarting pgrx Postgres

If you need to restart the pgrx-managed Postgres:

```bash
cargo pgrx stop --package pg_search
cargo pgrx install --package pg_search --pg-config <path>
cargo pgrx start --package pg_search
```

## Code Conventions

### Error Handling

Use `thiserror` for error types with descriptive messages:

```rust
#[derive(thiserror::Error, Debug)]
pub enum QueryError {
    #[error("wrong field type for field: {0}")]
    WrongFieldType(FieldName),

    #[error("could not build regex with pattern '{1}': {0}")]
    RegexError(#[source] tantivy::TantivyError, String),

    #[error(transparent)]
    TantivyError(#[from] tantivy::TantivyError),
}
```

- `pgrx::error!()` - Fatal errors that abort the transaction
- `Result<T, E>` - Recoverable errors
- `anyhow::Result` - Internal operations
- `#[source]` - Chain errors for context

### Logging

```rust
// Fatal error - aborts transaction
pgrx::error!("bucket_limit must be a positive integer");

// Warning - visible in regression test output, useful for debugging
pgrx::warning!("Query has LIMIT {} but is not using TopN scan", limit);

// Info log - use sparingly
pgrx::log!("Background worker started");
```

### Naming Conventions

- Functions/variables: `snake_case`
- Types/structs/enums: `PascalCase`
- Constants: `UPPER_SNAKE_CASE`
- Error types: `*Error` suffix (e.g., `QueryError`, `SchemaError`)

### File Headers

Every Rust file starts with:

```rust
// Copyright (c) 2023-2026 ParadeDB, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.
```

## Architecture

### Repository Structure

- **pg_search/**: Main Postgres extension implementing BM25 full-text search
  - Core modules: `postgres/`, `query/`, `schema/`, `index/`, `api/`
  - `tests/pg_regress/`: SQL regression tests

- **tokenizers/**: Standalone tokenization library
  - Multi-language support: Jieba (Chinese), Lindera (Japanese/Korean), ICU, n-gram
  - Integrates with Tantivy's tokenizer API

- **macros/**: Procedural macros for code generation
  - Builder functions for search queries
  - SQL generation for tokenizers

- **tests/**: Rust integration test suite
  - Uses sqlx for Postgres integration
  - Tests cover BM25 search, joins, aggregations, mutations, sorting

- **benchmarks/**: Performance testing and micro-benchmarks

- **stressgres/**: TUI-based Postgres stress testing tool

### Core Architecture Layers

#### 1. Postgres Integration (`pg_search/src/postgres/`)

**Index Access Method (IAM)**:

- Implements custom BM25 index handler via `bm25_handler()` function
- Registers custom scan strategies for text queries
- Manages index lifecycle: validate, build, insert, delete, vacuum, parallel operations

**Custom Scan Execution Framework** - Three scan types:

- **BaseScan**: Low-level index scans against BM25 indexes
- **AggregateScan**: Query-level aggregations pushed down to the index
- **JoinScan**: Join operations optimized for search indexes

**Storage Subsystem** (`postgres/storage/`):

- Uses Postgres heap blocks to store Tantivy index segments
- Block layout:
  - Blocks 0-1: Merge and cleanup locks
  - Block 2: Schema metadata (serialized Tantivy schema)
  - Block 3: Settings metadata
  - Block 4: Segment metadata entries
  - Blocks 5+: Actual segment component data

**Parallel Query Support**:

- `ParallelScanState`: Shared state between leader and worker processes
- Work-stealing segment pool for dynamic load balancing
- Aggregation result merging across parallel workers

#### 2. Query & Search Layer (`pg_search/src/query/`)

**SearchQueryInput Enum** (Core Query DSL):
Main variants: All, Boolean, Boost, DisjunctionMax, ConstScore, Term, Range, Phrase, Regex, Fuzzy, Parse, etc.

**Query Processing Pipeline**:

- `builder.rs`: Converts Postgres expressions to Tantivy queries
- `estimate_tree.rs`: Query cost estimation for planner
- `proximity/`: Proximity searches with phrase scoring
- `more_like_this.rs`: MLT (More Like This) functionality
- `heap_field_filter.rs`: Row visibility filtering for MVCC compliance

#### 3. Schema & Index Configuration (`pg_search/src/schema/`)

**SearchFieldType Enum**: Text, Tokenized, Uuid, Inet, I64, F64, U64, Bool, Json, Date, Range

**Field Configuration** - Each field has a `SearchFieldConfig` specifying:

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

- `paradedb.enable_custom_scan`: Toggle index scans (default: true)
- `paradedb.enable_aggregate_custom_scan`: Toggle aggregate pushdown (default: false)
- `paradedb.enable_join_custom_scan`: Toggle join optimization (default: false)
- `paradedb.enable_fast_field_exec`: Use columnar storage (default: true)
- `paradedb.limit_fetch_multiplier`: TopN optimization factor (default: 1.0)

## Key Dependencies

- **pgrx**: Version 0.16.1 (must be exact match)
- **Tantivy**: Forked version with custom features
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

## Common Patterns

### Adding a New Tokenizer

1. Add tokenizer implementation in `tokenizers/src/`
2. Register in `tokenizers/src/manager.rs`
3. Add SQL generation macro in `macros/src/`
4. Update `SearchFieldConfig` to support the new tokenizer
5. Add tests in `tests/tests/`

### Adding a New Query Type

1. Add variant to `SearchQueryInput` enum in `pg_search/src/query/mod.rs`
2. Implement conversion to Tantivy query in `pg_search/src/query/builder.rs`
3. Add cost estimation logic in `pg_search/src/query/estimate_tree.rs`
4. Add tests in `tests/tests/`

### Modifying Index Storage Format

Changes to storage require careful handling:

1. Update block layout constants in `pg_search/src/postgres/storage/block.rs`
2. Update metadata serialization in `pg_search/src/postgres/storage/metadata.rs`
3. Consider migration path for existing indexes
4. Update vacuum and merge logic if needed

## Debugging Tips

- Use `cargo pgrx run` for interactive development with Postgres
- Check Postgres logs for custom scan execution details
- Use `EXPLAIN` to see query plans with custom scans
- Set `RUST_BACKTRACE=1` for detailed error traces
- For parallel query debugging, check worker process logs
- Use `pgrx::warning!()` to add debug output visible in regression test results

## Pull Request Requirements

- Follow [Conventional Commits](https://www.conventionalcommits.org/) for PR titles
- Add tests for new features (features without tests will not be merged)
- Add documentation in `docs/` for user-facing features
- Pre-commit hooks must pass
- Sign the CLA (enforced by CLA Assistant)

## Critical Design Decisions

1. **Tantivy Integration**: Uses forked Tantivy with ParadeDB-specific features
2. **Postgres Block Storage**: Index data stored in Postgres blocks (not external files) for durability
3. **Parallel by Default**: Parallel workers claim segments dynamically
4. **MVCC-Aware**: Respects Postgres transaction isolation
5. **Hook-Based**: Minimal Postgres core modification; primarily uses planner hooks
