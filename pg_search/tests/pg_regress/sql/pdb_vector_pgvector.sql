SET client_min_messages = warning;
CREATE EXTENSION IF NOT EXISTS pg_search;
CREATE EXTENSION IF NOT EXISTS vector;
SET max_parallel_workers_per_gather = 0;
SELECT '[1,2,3]'::vector::pdb.vector AS native,
       '[1,2,3]'::pdb.vector::vector AS pgvector;
SELECT '[1,2,3]'::vector::pdb.vector(2);
SELECT '[1,2,3]'::pdb.vector::vector(2);
SELECT paradedb.vector_send('[1.2345678,1e-40,3.4028235e38]'::pdb.vector)
       = vector_send('[1.2345678,1e-40,3.4028235e38]'::vector) AS binary_compatible;
SELECT castmethod, castcontext FROM pg_cast
WHERE (castsource = 'vector'::regtype AND casttarget = 'pdb.vector'::regtype)
   OR (castsource = 'pdb.vector'::regtype AND casttarget = 'vector'::regtype)
ORDER BY castcontext;
CREATE TABLE compatible_vectors (id integer PRIMARY KEY, vec vector(3));
INSERT INTO compatible_vectors VALUES
    (1, '[1,0,0]'), (2, '[0,2,0]'), (3, '[-1,0,0]'), (4, '[2,1,0]');
CREATE INDEX compatible_vectors_idx ON compatible_vectors USING paradedb (id, vec vector_cosine_ops);
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT id FROM compatible_vectors WHERE id @@@ pdb.all() ORDER BY vec <=> '[1,0,0]' LIMIT 3;
SELECT id FROM compatible_vectors WHERE id @@@ pdb.all() ORDER BY vec <=> '[1,0,0]' LIMIT 3;
SET plan_cache_mode = force_generic_plan;
PREPARE compatible_nearest(vector) AS
SELECT id FROM compatible_vectors WHERE id @@@ pdb.all() ORDER BY vec <=> $1 LIMIT 3;
EXECUTE compatible_nearest('[1,0,0]');
DEALLOCATE compatible_nearest;
RESET plan_cache_mode;
DROP TABLE compatible_vectors;
