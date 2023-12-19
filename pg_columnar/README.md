# pg_columnar

This extension enables fast columnar access in PostgreSQL.

## Benchmarks

To run benchmarks locally for development, first enter the `pg_columnar/` directory before running `cargo clickbench`. This runs a minified version of the ClickBench benchmark suite on a purely in-memory version of `pg_columnar`. As of writing, this is the only functional benchmark suite as we haven't built persistence in our TableAM. Once we do, you can run the full suite using on-disk storage via `cargo clickbench_cold`.
