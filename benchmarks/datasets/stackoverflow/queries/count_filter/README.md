# count_filter

Note: Native PostgreSQL aggregation atop GIN indexes is omitted as it is unworkably slow on large datasets. ParadeDB also executes this query faster than PostgreSQL using the columnar base scan (`paradedb.enable_aggregate_custom_scan = off`). This directory retains `basescan.sql` as a representative single-table scalar aggregate baseline.
