-- test that when index config isn't passed correctly, the meaningful errors are returned

\echo 'Test: Index configuration errors'

DROP TABLE IF EXISTS test_index_config_errors CASCADE;
CREATE TABLE test_index_config_errors
(
    id    serial8 not null primary key,
    name  text
);

CREATE INDEX idx_chunks_bm25 ON test_index_config_errors
    USING paradedb (id, (some_wrong_key::pdb.simple));


CREATE INDEX idx_chunks_bm25 ON test_index_config_errors
    USING paradedb (id, (name::pdb.some_wrong_type));


CREATE INDEX idx_chunks_bm25 ON test_index_config_errors
    USING paradedb (id, (name::pdb.simple('columnar=invalid')));



CREATE INDEX idx_chunks_bm25 ON test_index_config_errors USING paradedb (id, (name::pdb.simple));
CREATE INDEX idx_chunks_bm25_configured ON test_index_config_errors
    USING paradedb (id, (name::pdb.simple('alias=id')));


DROP TABLE test_index_config_errors CASCADE;
