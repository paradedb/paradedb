-- test that when index config isn't passed correctly, the meaningful errors are returned

\echo 'Test: Index configuration errors'

DROP TABLE IF EXISTS test_index_config_errors CASCADE;
CREATE TABLE test_index_config_errors
(
    id    serial8 not null primary key,
    name  text
);

CREATE INDEX idx_chunks_bm25 ON test_index_config_errors
    USING bm25 (id, name)
    WITH (
    key_field = 'id',
    text_fields ='{
        "some_wrong_key": {"tokenizer": {"type": "default"}}
    }'
    );


CREATE INDEX idx_chunks_bm25 ON test_index_config_errors
    USING bm25 (id, name)
    WITH (
    key_field = 'id',
    text_fields ='{
        "name": {"tokenizer": {"type": "some_wrong_type"}}
    }'
    );


CREATE INDEX idx_chunks_bm25 ON test_index_config_errors
    USING bm25 (id, name)
    WITH (
    key_field = 'id',
    text_fields ='{
        "id": {"tokenizer": {"type": "default"}}
    }'
    );



CREATE INDEX idx_chunks_bm25 ON test_index_config_errors USING bm25 (id, name);
CREATE INDEX idx_chunks_bm25 ON test_index_config_errors USING bm25 (id, name) WITH (text_fields ='{"id": {"tokenizer": {"type": "default"}}}');


DROP TABLE test_index_config_errors CASCADE;
