-- Recall@10 fixtures: the held-out query set and the precomputed exact top-10 ground truth. The
-- harness creates these tables and loads each from parquet (via DuckDB) right after its CREATE. It
-- then measures recall in Rust: for each held-out vector it sets `cohere.qvec` and runs the actual
-- latency query (queries/<query>.sql) verbatim -- same statement, same plan the benchmark picks --
-- and intersects the returned top-10 with recall_gt. (Running the query with `cohere.qvec` as a
-- per-vector constant, rather than a join parameter, is what keeps recall's plan identical to the
-- latency query's; a lateral parameter can tip the planner to a different, e.g. pre-filter, plan.)
DROP TABLE IF EXISTS cohere_queries;
CREATE TABLE cohere_queries (id int, emb vector(1024));
DROP TABLE IF EXISTS recall_gt;
CREATE TABLE recall_gt (query_id int, gt_ids text[]);
