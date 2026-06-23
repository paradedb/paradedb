-- Held-out query set for recall@k. The table schema only; vectors are loaded from
-- {data_source}/queries/*.parquet by `cargo run -- recall` (see run_recall in main.rs).
DROP TABLE IF EXISTS cohere_queries;
CREATE TABLE cohere_queries (id int, emb vector(1024));
