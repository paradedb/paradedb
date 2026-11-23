SET client_min_messages = warning;
CREATE EXTENSION IF NOT EXISTS pg_search;
SET max_parallel_workers_per_gather = 0;

SELECT '[1, 2.5, -3e-2]'::pdb.vector AS vector,
       format_type('pdb.vector'::regtype, 3) AS vector_type;
SELECT ARRAY[1, 2, 3]::real[]::pdb.vector(3)::real[] AS floats;
SELECT ARRAY[1, 2, 3]::double precision[]::pdb.vector(3) AS vector;
SELECT '[1,2]'::pdb.vector(3);
SELECT '[1,2]'::pdb.vector::pdb.vector(3);
SELECT '[]'::pdb.vector;
SELECT '[1,]'::pdb.vector;
SELECT '[NaN]'::pdb.vector;
SELECT '[Infinity]'::pdb.vector;
SELECT '[1] trailing'::pdb.vector;
SELECT ARRAY[1, NULL]::real[]::pdb.vector;
SELECT ARRAY[[1, 2], [3, 4]]::real[]::pdb.vector;
SELECT ARRAY[1e100]::double precision[]::pdb.vector;
SELECT '[1]'::pdb.vector(0);
SELECT '[1]'::pdb.vector(16001);
SELECT '[1]'::pdb.vector(1, 2);
SELECT '[1,2]'::pdb.vector < '[1,3]'::pdb.vector AS element_order,
       '[1,2]'::pdb.vector < '[1,2,0]'::pdb.vector AS prefix_order,
       '[0]'::pdb.vector = '[-0]'::pdb.vector AS equal_zero;
SELECT '[1,0]'::pdb.vector <-> '[4,4]' AS l2,
       '[1,2]'::pdb.vector <#> '[3,4]' AS negative_inner_product,
       '[1,0]'::pdb.vector <=> '[0,1]' AS cosine;
SELECT '[1,0]'::pdb.vector <-> '[1]';
SELECT encode(paradedb.vector_send('[1,2]'::pdb.vector), 'hex') AS binary;
SELECT '[1.2345678,1e-40,3.4028235e38]'::pdb.vector::text::pdb.vector
       = '[1.2345678,1e-40,3.4028235e38]'::pdb.vector AS text_roundtrip;

CREATE TABLE native_vectors (id integer PRIMARY KEY, label text, vec pdb.vector(3));
INSERT INTO native_vectors VALUES
    (1, 'one', '[1,0,0]'), (2, 'two', '[0,2,0]'),
    (3, 'three', '[-1,0,0]'), (4, 'four', '[2,1,0]');
CREATE INDEX native_vectors_idx ON native_vectors USING paradedb
    (id, label, vec pdb.vector_cosine_ops) WITH (target_segment_count = 1);
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT id FROM native_vectors WHERE id @@@ pdb.all() ORDER BY vec <=> '[1,0,0]' LIMIT 3;
SELECT id FROM native_vectors WHERE id @@@ pdb.all() ORDER BY vec <=> '[1,0,0]' LIMIT 3;
SET plan_cache_mode = force_generic_plan;
PREPARE native_nearest(pdb.vector) AS
SELECT id FROM native_vectors WHERE id @@@ pdb.all() ORDER BY vec <=> $1 LIMIT 3;
EXECUTE native_nearest('[1,0,0]');
EXECUTE native_nearest('[0.1,1,0]');
DEALLOCATE native_nearest;
RESET plan_cache_mode;
SELECT id FROM native_vectors WHERE id @@@ pdb.all()
ORDER BY vec <=> ARRAY[1,0,0]::real[]::pdb.vector LIMIT 3;
UPDATE native_vectors SET vec = '[3,0.5,0]' WHERE id = 2;
SELECT id FROM native_vectors WHERE id @@@ pdb.all() ORDER BY vec <=> '[0.1,1,0]' LIMIT 3;
REINDEX INDEX native_vectors_idx;
SELECT id FROM native_vectors WHERE id @@@ pdb.all() ORDER BY vec <=> '[0.1,1,0]' LIMIT 3;
DROP INDEX native_vectors_idx;
CREATE INDEX native_vectors_idx ON native_vectors USING paradedb (id, vec);
SELECT id FROM native_vectors WHERE id @@@ pdb.all() ORDER BY vec <-> '[1,0,0]' LIMIT 3;
DROP INDEX native_vectors_idx;
CREATE INDEX native_vectors_idx ON native_vectors USING bm25 (id, vec pdb.vector_ip_ops);
SELECT id FROM native_vectors WHERE id @@@ pdb.all() ORDER BY vec <#> '[1,0,0]' LIMIT 3;
DROP TABLE native_vectors;

CREATE TABLE native_vector_toast (vec pdb.vector(1024));
INSERT INTO native_vector_toast
SELECT array_agg((i % 13)::real ORDER BY i)::pdb.vector FROM generate_series(1,1024) i;
SELECT paradedb.vector_dims(vec) AS dimensions,
       vec = vec::text::pdb.vector AS text_roundtrip,
       vec <-> vec AS self_distance FROM native_vector_toast;
DROP TABLE native_vector_toast;
