-- Legacy JSON configuration preserves numeric term matching until tokenizer casts support it.
\i common/common_setup.sql

-- =========================================
-- Test 14: Special characters and edge cases
-- =========================================

-- Create table with special JSON keys
CREATE TABLE json_special_agg (
    id SERIAL PRIMARY KEY,
    payload JSONB
);

-- Insert data with special characters
INSERT INTO json_special_agg (payload) VALUES
    ('{"user-profile": {"first_name": "John", "email@work": "john@company.com", "settings.theme": "dark"}}'),
    ('{"user-profile": {"first_name": "Jane", "email@work": "jane@company.com", "settings.theme": "light"}}'),
    ('{"api-response": {"status_code": 200, "response.time": 150, "cache-hit": true}}'),
    ('{"api-response": {"status_code": 404, "response.time": 50, "cache-hit": false}}');

-- Create BM25 index
CREATE INDEX idx_json_special_agg ON json_special_agg
USING paradedb (id, payload)
WITH (
    json_fields = '{"payload": {"indexed": true, "fast": true, "expand_dots": true}}'
);

-- Test COUNT with special character fields
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*)
FROM json_special_agg
WHERE id @@@ paradedb.exists('payload.user-profile.email@work');

SELECT COUNT(*)
FROM json_special_agg
WHERE id @@@ paradedb.exists('payload.user-profile.email@work');

-- Test COUNT on API responses
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF, VERBOSE)
SELECT COUNT(*)
FROM json_special_agg
WHERE id @@@ paradedb.term('payload.api-response.status_code', '200');

SELECT COUNT(*)
FROM json_special_agg
WHERE id @@@ paradedb.term('payload.api-response.status_code', '200');

DROP TABLE json_special_agg;
