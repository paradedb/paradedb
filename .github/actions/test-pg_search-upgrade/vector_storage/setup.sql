-- Vector storage upgrade coverage requires the vector operator class.
CREATE TABLE vector_upgrade_state (has_vector_indexes boolean NOT NULL);

DO $body$
DECLARE
    supported boolean;
BEGIN
    SELECT EXISTS (
        SELECT 1 FROM pg_opclass c JOIN pg_am a ON a.oid = c.opcmethod
        WHERE c.opcname = 'vector_l2_ops' AND a.amname = 'bm25'
    ) INTO supported;
    INSERT INTO vector_upgrade_state VALUES (supported);
    IF NOT supported THEN
        RAISE NOTICE 'Vector storage upgrade case skipped: vector indexes are unavailable';
        RETURN;
    END IF;

    CREATE TABLE vector_upgrade_docs (id integer PRIMARY KEY, embedding vector(3));
    INSERT INTO vector_upgrade_docs VALUES
        (1, '[1,0,0]'), (2, '[0,1,0]'), (3, '[0,0,1]');
    CREATE INDEX vector_upgrade_idx ON vector_upgrade_docs
        USING bm25 (id, embedding vector_l2_ops) WITH (key_field = 'id');
END
$body$;
