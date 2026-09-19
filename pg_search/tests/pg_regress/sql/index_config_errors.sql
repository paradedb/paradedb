-- Invalid column names, tokenizer types, and input types produce useful errors.
\echo 'Test: Index configuration errors'

CREATE TABLE test_index_config_errors (id bigserial PRIMARY KEY, name text);

CREATE INDEX idx_chunks ON test_index_config_errors
USING paradedb (id, name, (some_wrong_key::pdb.unicode_words));

CREATE INDEX idx_chunks ON test_index_config_errors
USING paradedb (id, (name::pdb.some_wrong_type));

CREATE INDEX idx_chunks ON test_index_config_errors
USING paradedb ((id::pdb.unicode_words), name);

CREATE INDEX idx_chunks ON test_index_config_errors USING paradedb (id, name);
CREATE INDEX idx_chunks_configured ON test_index_config_errors
USING paradedb ((id::pdb.unicode_words), name);

DROP TABLE test_index_config_errors CASCADE;
