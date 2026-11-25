SET client_min_messages = warning;
SELECT current_database() AS orig_db \gset
CREATE DATABASE pdb_vector_lifecycle TEMPLATE template0;
\set QUIET 1
\c pdb_vector_lifecycle
\set QUIET 0
\set VERBOSITY terse
SET client_min_messages = warning;
SET max_parallel_workers_per_gather = 0;

CREATE EXTENSION pg_search;
SELECT EXISTS (SELECT FROM pg_extension WHERE extname = 'vector') AS pgvector_installed;
CREATE TABLE native_vectors (id integer PRIMARY KEY, embedding pdb.vector(3));
INSERT INTO native_vectors VALUES (1, '[1,0,0]'), (2, '[0,1,0]');
CREATE INDEX native_vectors_idx ON native_vectors
USING paradedb (id, embedding pdb.vector_cosine_ops);
SELECT id FROM native_vectors WHERE id @@@ pdb.all()
ORDER BY embedding <=> '[1,0,0]' LIMIT 2;

CREATE EXTENSION vector;
CREATE ROLE pdb_vector_dropper;
SET ROLE pdb_vector_dropper;
DROP OWNED BY CURRENT_USER;
RESET ROLE;
DROP ROLE pdb_vector_dropper;
SELECT count(*) AS extension_owned_casts
FROM pg_cast c JOIN pg_depend d ON d.classid = 'pg_cast'::regclass AND d.objid = c.oid
WHERE d.deptype = 'e' AND d.refobjid = (SELECT oid FROM pg_extension WHERE extname = 'pg_search')
  AND ((c.castsource = 'vector'::regtype AND c.casttarget = 'pdb.vector'::regtype)
    OR (c.castsource = 'pdb.vector'::regtype AND c.casttarget = 'vector'::regtype));
CREATE TABLE compatible_vectors (id integer PRIMARY KEY, embedding vector(3));
INSERT INTO compatible_vectors SELECT * FROM native_vectors;
CREATE INDEX compatible_vectors_idx ON compatible_vectors
USING paradedb (id, embedding vector_cosine_ops);
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT id FROM compatible_vectors WHERE id @@@ pdb.all()
ORDER BY embedding::pdb.vector <=> '[1,0,0]'::pdb.vector LIMIT 2;
SELECT id FROM compatible_vectors WHERE id @@@ pdb.all()
ORDER BY embedding::pdb.vector <=> '[1,0,0]'::pdb.vector LIMIT 2;

SELECT current_setting('data_directory') || '/pdb_vector_lifecycle.copy' AS vector_copy_path \gset
COPY compatible_vectors TO :'vector_copy_path' (FORMAT BINARY);
CREATE TABLE binary_vectors (id integer, embedding pdb.vector(3));
COPY binary_vectors FROM :'vector_copy_path' (FORMAT BINARY);
SELECT * FROM binary_vectors ORDER BY id;
CREATE TABLE wrong_dimensions (id integer, embedding pdb.vector(2));
COPY wrong_dimensions FROM :'vector_copy_path' (FORMAT BINARY);

DROP TABLE compatible_vectors;
DROP EXTENSION vector CASCADE;
SELECT id FROM native_vectors WHERE id @@@ pdb.all()
ORDER BY embedding <=> '[1,0,0]' LIMIT 2;
CREATE EXTENSION vector;
SELECT '[1,2,3]'::vector::pdb.vector AS native;
DROP EXTENSION pg_search CASCADE;
SELECT EXISTS (SELECT FROM pg_extension WHERE extname = 'vector') AS pgvector_preserved;

CREATE EXTENSION pg_search;
SELECT '[1,2,3]'::vector::pdb.vector AS native;
CREATE TABLE recreated_native_vectors (id integer PRIMARY KEY, embedding pdb.vector(3));
INSERT INTO recreated_native_vectors VALUES (1, '[1,0,0]');
CREATE INDEX recreated_native_vectors_idx ON recreated_native_vectors
USING paradedb (id, embedding pdb.vector_cosine_ops);
SELECT id FROM recreated_native_vectors WHERE id @@@ pdb.all()
ORDER BY embedding <=> '[1,0,0]' LIMIT 1;
DROP EXTENSION pg_search CASCADE;
DROP EXTENSION vector;

SET max_parallel_workers_per_gather = 0;
CREATE SCHEMA vector_ext;
CREATE EXTENSION vector WITH SCHEMA vector_ext;
CREATE EXTENSION pg_search;
CREATE SCHEMA vector_moved;
ALTER EXTENSION vector SET SCHEMA vector_moved;
SELECT '[1,2,3]'::vector_moved.vector::pdb.vector AS native;
ALTER EXTENSION vector SET SCHEMA vector_ext;
CREATE TABLE relocated_vectors (id integer PRIMARY KEY, embedding vector_ext.vector(3));
INSERT INTO relocated_vectors VALUES (1, '[1,0,0]'), (2, '[0,1,0]');
CREATE INDEX relocated_vectors_idx ON relocated_vectors
USING paradedb (id, embedding vector_ext.vector_cosine_ops);
CREATE TABLE relocated_native_vectors (id integer PRIMARY KEY, embedding pdb.vector(3));
INSERT INTO relocated_native_vectors SELECT * FROM relocated_vectors;
CREATE INDEX relocated_native_vectors_idx ON relocated_native_vectors
USING paradedb (id, embedding pdb.vector_cosine_ops);
SELECT vector_ext.vector_send('[1.2345678,1e-40,3.4028235e38]'::vector_ext.vector)
       = paradedb.vector_send('[1.2345678,1e-40,3.4028235e38]'::pdb.vector) AS binary_compatible;
EXPLAIN (COSTS OFF, TIMING OFF)
SELECT id FROM relocated_vectors WHERE id @@@ pdb.all()
ORDER BY embedding::pdb.vector <=> '[1,0,0]'::pdb.vector LIMIT 2;
SELECT id FROM relocated_vectors WHERE id @@@ pdb.all()
ORDER BY embedding::pdb.vector <=> '[1,0,0]'::pdb.vector LIMIT 2;
DROP EXTENSION vector CASCADE;
SELECT id FROM relocated_native_vectors WHERE id @@@ pdb.all()
ORDER BY embedding <=> '[1,0,0]' LIMIT 2;
CREATE EXTENSION vector WITH SCHEMA vector_ext;
SELECT '[1,2,3]'::vector_ext.vector::pdb.vector AS native;
DROP SCHEMA vector_ext CASCADE;
SELECT id FROM relocated_native_vectors WHERE id @@@ pdb.all()
ORDER BY embedding <=> '[1,0,0]' LIMIT 2;
CREATE SCHEMA vector_ext;
CREATE EXTENSION vector WITH SCHEMA vector_ext;
DROP EXTENSION pg_search CASCADE;
SELECT EXISTS (SELECT FROM pg_extension WHERE extname = 'vector') AS pgvector_preserved;

\set QUIET 1
\c :orig_db
\set QUIET 0
DROP DATABASE pdb_vector_lifecycle;
