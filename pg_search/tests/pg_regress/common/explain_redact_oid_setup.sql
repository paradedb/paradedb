-- EXPLAIN a query with index OIDs masked. Plans that fall back to a heap
-- filter embed the BM25 index OID, which changes on every database creation.
CREATE OR REPLACE FUNCTION explain_redact_oid(query text, options text DEFAULT 'COSTS OFF, TIMING OFF')
RETURNS SETOF text LANGUAGE plpgsql AS $$
DECLARE
    line text;
BEGIN
    FOR line IN EXECUTE 'EXPLAIN (' || options || ') ' || query LOOP
        RETURN NEXT regexp_replace(line, '"oid":\d+', '"oid":N', 'g');
    END LOOP;
END;
$$;
