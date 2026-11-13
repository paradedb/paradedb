# Vector operating-point cache

Cohere, HN, and MS MARCO invoke this action before their pdbench searches. It sets
`PDBENCH_OPERATING_POINTS_DIR` for the job, including router-quality subprocesses,
so all searches use pdbench's shared config and validation implementation.

Each run restores the newest available cache for its suite, OS, and architecture.
The run ID and attempt make the save key unique: Actions caches are immutable, so
this lets a successful run publish updated configs instead of retaining the first
selection forever. The cache action saves at job completion, after router-quality
searches have also finished. Normal Actions cache branch scope applies; PR caches
do not replace main's cache. Cache storage uses the existing runner setup and
requires no changes to the heap snapshot's read-only S3 role.

pdbench partitions versioned TOML files by profile, query, and a fingerprint of the
query, index configuration, tuning rules, query vectors, and ground truth. Every
cached value is measured again against the current index. A miss or failed recall
validation retunes only the affected targets. An unchanged below-target fallback
remains explicitly unreached. Latency, diagnostics, and downstream k6 measurements
are always fresh.

Manual vector workflows accept `tuning`: `auto` validates and repairs the cache,
`required` consumes it without sweeping, and `refresh` retunes all targets. Use
`refresh` to discover cheaper settings after an improvement that still leaves the
cached values above their recall targets. Local reproduction can use pdbench's
`--operating-points-in` and `--operating-points-out` flags directly.
