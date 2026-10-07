# SchemaBot

Repository-local checks for migration fragments against the SQL emitted by
`cargo pgrx schema`. SchemaBot is a member of the repository Rust workspace.

Requires Rust, a C compiler, libclang, and protoc (on Ubuntu:
`apt install clang libclang-dev protobuf-compiler`).

```sh
cargo test -p schemabot --locked
cargo build -p schemabot --locked
cargo pgrx schema -p pg_search pg18 > head.sql
# Generate base.sql from the base revision with its matching cargo-pgrx version.
target/debug/schemabot diff base.sql head.sql > diff.sql
target/debug/schemabot check diff.sql pg_search/sql/unreleased/123.change.sql
```

`diff` compares parsed statements, ignoring source positions and function-option
order. It preserves source SQL for additions. Supported removals get suggested
DROP/REVOKE statements, with operators, casts, views and operator classes dropped
before routines and types. Routine signatures exclude output columns and defaults.
Schema-owner changes use ALTER SCHEMA. Unsupported replacements or removals fail
with a diagnostic so the tool cannot silently approve an unknown change.

The planner deliberately handles extension-oriented DDL rather than arbitrary
schema evolution. Enum, domain and type replacements need explicit support;
aggregate removals are unsupported. Installation actions such as DO, INSERT and
ALTER FUNCTION have no inferred inverse. Dependency ordering and migration
execution still require review; this tool does not inspect the database catalog.

`check` requires every suggested statement to appear in the fragments, independent
of order, comments, source positions, and parser-normalized syntax. Additional
statements are allowed. Function-option order and CREATE OR REPLACE are normalized.
Function bodies remain strings and must match. GRANT and REVOKE are checked too.
Files are parsed separately, and only the initial psql extension-script guard is
stripped, preserving quoted semicolons and comment-like strings in SQL bodies.

pg_query 6.2.1 uses PostgreSQL 17's grammar, so PostgreSQL 18-only syntax may require
a parser upgrade. Structural comparison does not prove semantic equivalence or
successful migration execution.
