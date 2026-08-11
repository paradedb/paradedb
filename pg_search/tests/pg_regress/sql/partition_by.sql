-- Tests for partition_by syntax in CREATE INDEX

\i common/common_setup.sql

-- SECTION 1: Basic syntax validation
\echo '=== SECTION 1: Basic syntax validation ==='

DROP TABLE IF EXISTS partition_by_test CASCADE;
CREATE TABLE partition_by_test (
    id SERIAL PRIMARY KEY,
    name TEXT,
    tenant_id INTEGER,
    created_at TIMESTAMP
);

INSERT INTO partition_by_test (name, tenant_id, created_at) VALUES
    ('Alice', 1, '2023-01-01'),
    ('Bob', 2, '2023-06-01'),
    ('Charlie', 1, '2023-12-01');

\echo 'Test 1.1: partition_by with single field'
CREATE INDEX partition_by_test_idx ON partition_by_test
    USING paradedb (id, name, tenant_id)
    WITH (key_field='id', partition_by='tenant_id');
DROP INDEX partition_by_test_idx;

\echo 'Test 1.2: partition_by with multiple fields'
CREATE INDEX partition_by_test_idx ON partition_by_test
    USING paradedb (id, name, tenant_id, created_at)
    WITH (key_field='id', partition_by='tenant_id, created_at');
DROP INDEX partition_by_test_idx;

\echo 'Test 1.3: partition_by with whitespace around fields'
CREATE INDEX partition_by_test_idx ON partition_by_test
    USING paradedb (id, name, tenant_id, created_at)
    WITH (key_field='id', partition_by=' tenant_id ,  created_at ');
DROP INDEX partition_by_test_idx;

DROP TABLE partition_by_test CASCADE;

-- SECTION 2: Error cases
\echo '=== SECTION 2: Error cases ==='

DROP TABLE IF EXISTS partition_by_test CASCADE;
CREATE TABLE partition_by_test (
    id SERIAL PRIMARY KEY,
    name TEXT,
    score INTEGER
);

\echo 'Test 2.1: partition_by with nonexistent field (should error)'
CREATE INDEX partition_by_test_idx ON partition_by_test
    USING paradedb (id, name, score)
    WITH (key_field='id', partition_by='nonexistent');

\echo 'Test 2.2: partition_by with invalid syntax / empty string (should error)'
CREATE INDEX partition_by_test_idx ON partition_by_test
    USING paradedb (id, name, score)
    WITH (key_field='id', partition_by='');
-- Empty string disables it, so it succeeds. We drop it.
DROP INDEX partition_by_test_idx;

\echo 'Test 2.3: partition_by with whitespace only (should error)'
CREATE INDEX partition_by_test_idx ON partition_by_test
    USING paradedb (id, name, score)
    WITH (key_field='id', partition_by='   ');

\echo 'Test 2.4: partition_by with just commas (should error)'
CREATE INDEX partition_by_test_idx ON partition_by_test
    USING paradedb (id, name, score)
    WITH (key_field='id', partition_by=',');

\echo 'Test 2.5: partition_by with commas and whitespace (should error)'
CREATE INDEX partition_by_test_idx ON partition_by_test
    USING paradedb (id, name, score)
    WITH (key_field='id', partition_by=' , ');

DROP TABLE partition_by_test CASCADE;

-- SECTION 3: Multi-valued fields (should error)
\echo '=== SECTION 3: Multi-valued fields ==='

DROP TABLE IF EXISTS partition_by_multi_test CASCADE;
CREATE TABLE partition_by_multi_test (
    id SERIAL PRIMARY KEY,
    tags TEXT[],
    meta JSONB,
    int_array INTEGER[]
);

\echo 'Test 3.1: partition_by with array field (should error)'
CREATE INDEX partition_by_multi_test_idx ON partition_by_multi_test
    USING paradedb (id, tags, meta)
    WITH (key_field='id', partition_by='tags');

\echo 'Test 3.2: partition_by with json field (should error)'
CREATE INDEX partition_by_multi_test_idx ON partition_by_multi_test
    USING paradedb (id, tags, meta)
    WITH (key_field='id', partition_by='meta');

\echo 'Test 3.3: partition_by with aliased array expression (should error)'
CREATE INDEX partition_by_multi_test_idx ON partition_by_multi_test
    USING paradedb (id, (int_array::pdb.alias('aliased_array')))
    WITH (key_field='id', partition_by='aliased_array');

DROP TABLE partition_by_multi_test CASCADE;
