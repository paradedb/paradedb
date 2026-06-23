-- recall@k of the built ANN index against a held-out query set (cohere_queries, loaded by
-- recall_queries.sql). For each held-out query we compare the index's top-k to the exact top-k.
-- k comes from [params] in config.toml. The final statement returns the average recall@k.

-- Exact ground truth: force a sequential scan (no ANN index) so distances are exact.
SET enable_indexscan = off;
SET enable_bitmapscan = off;
CREATE TEMP TABLE recall_gt AS
  SELECT q.id AS qid,
         ARRAY(SELECT c._id FROM cohere_wiki c ORDER BY c.emb <=> q.emb LIMIT {{ recall_k }}) AS ids
  FROM cohere_queries q;
RESET enable_indexscan;
RESET enable_bitmapscan;

-- Approximate top-k via the ANN index (at the database-level ivfflat.probes / hnsw.ef_search set by
-- after_create_index.sql, i.e. the same effort as the latency queries), averaged into recall@k.
SELECT avg(
         cardinality(ARRAY(SELECT unnest(gt.ids) INTERSECT SELECT unnest(a.ids)))::float
         / {{ recall_k }}
       )
FROM recall_gt gt
CROSS JOIN LATERAL (
  SELECT ARRAY(
           SELECT c._id FROM cohere_wiki c
           ORDER BY c.emb <=> (SELECT emb FROM cohere_queries WHERE id = gt.qid)
           LIMIT {{ recall_k }}
         ) AS ids
) a;
