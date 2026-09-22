# bucket_expr_filter

Note: Native PostgreSQL aggregation atop GIN indexes is omitted as it is unworkably slow on large datasets. ParadeDB also executes this query faster than PostgreSQL using the columnar base scan (`paradedb.enable_aggregate_custom_scan = off`), but redundant `basescan.sql` variants were pruned from the suite to keep execution fast.
