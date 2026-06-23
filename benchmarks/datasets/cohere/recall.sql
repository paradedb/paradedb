-- recall@k of the built ANN index against a held-out query set. For each held-out query we compare
-- the index's top-k to the precomputed exact top-k. k comes from [params] in config.toml; the final
-- statement returns the average recall@k.

-- Held-out query set + precomputed exact ground truth. The harness populates both from parquet
-- (DuckDB) right after each table is created, before the recall query below reads them. The ground
-- truth is exact nearest neighbors computed once offline, so recall needs no sequential scans.
DROP TABLE IF EXISTS cohere_queries;
CREATE TABLE cohere_queries (id int, emb vector(1024));
DROP TABLE IF EXISTS recall_gt;
CREATE TABLE recall_gt (query_id int, gt_ids text[]);

-- Approximate top-k via the ANN index (at the database-level ivfflat.probes / hnsw.ef_search set by
-- after_create_index.sql, i.e. the same effort as the latency queries), averaged into recall@k.
SELECT avg(
         cardinality(ARRAY(SELECT unnest(gt.gt_ids) INTERSECT SELECT unnest(a.ids)))::float
         / {{ recall_k }}
       )
FROM recall_gt gt
JOIN cohere_queries q ON q.id = gt.query_id
CROSS JOIN LATERAL (
  -- q.emb (a join parameter, not a correlated subquery) is what lets pgvector use the ANN index
  -- here; an `... <=> (SELECT emb ...)` operand falls back to an exact sequential scan.
  SELECT ARRAY(
           SELECT c._id FROM cohere_wiki c
           ORDER BY c.emb <=> q.emb
           LIMIT {{ recall_k }}
         ) AS ids
) a;
