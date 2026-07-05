\i common/common_setup.sql

DROP TABLE IF EXISTS issue_5254;

CREATE TABLE issue_5254 (
    id SERIAL PRIMARY KEY,
    bar TEXT NOT NULL
);

INSERT INTO issue_5254 (bar) VALUES ('alpha'), ('beta'), ('gamma');

CREATE INDEX issue_5254_idx ON issue_5254
    USING bm25 (id, (bar::pdb.literal), (bar::pdb.literal_normalized('alias=bar_lower')))
    WITH (key_field = id);

-- Before the fix, this failed with:
-- ERROR: cannot execute INSERT in a read-only transaction
BEGIN TRANSACTION READ ONLY;
SELECT id, bar FROM issue_5254 WHERE bar::pdb.alias('bar_lower') @@@ 'alpha' ORDER BY id;
COMMIT;

-- Verify that re-parsing alias=bar_lower does not produce alias=alias=bar_lower.
BEGIN TRANSACTION READ ONLY;
SELECT id, bar FROM issue_5254 WHERE bar::pdb.alias('alias=bar_lower') @@@ 'beta' ORDER BY id;
COMMIT;

-- Extra args must be rejected.
DO $$
BEGIN
    PERFORM NULL::pdb.alias('foo', 'bar');
EXCEPTION WHEN OTHERS THEN
    RAISE WARNING '%', SQLERRM;
END $$;

DROP TABLE issue_5254;
