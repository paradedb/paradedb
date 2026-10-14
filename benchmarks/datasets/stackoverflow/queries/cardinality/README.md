# cardinality

Note: Native PostgreSQL aggregation atop GIN indexes is omitted as it is unworkably slow on large datasets. ParadeDB also executes these queries faster than PostgreSQL using the columnar base scan (`paradedb.enable_aggregate_custom_scan = off`). Redundant base scan variants (`basescan_group_by.sql` and `basescan_high_cardinality_group_by.sql`) were pruned to keep benchmark execution fast, while `basescan_distinct.sql` is retained as a representative baseline for `COUNT(DISTINCT ...)`.
