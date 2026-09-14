BEGIN;
SET LOCAL client_min_messages = error;

CREATE TABLE sequential_scan_unplanned (id int NOT NULL, body text NOT NULL);
INSERT INTO sequential_scan_unplanned VALUES (1, 'alpha'), (2, 'beta');
CREATE INDEX sequential_scan_unplanned_idx ON sequential_scan_unplanned
USING paradedb (id, body);

DO $$
DECLARE
    term text;
    error_message text;
    error_hint text;
BEGIN
    FOREACH term IN ARRAY ARRAY['alpha', 'missing'] LOOP
        BEGIN
            EXECUTE format(
                'CREATE INDEX sequential_scan_unplanned_partial ON sequential_scan_unplanned (id)
                 WHERE id @@@ paradedb.with_index(''sequential_scan_unplanned_idx'',
                     paradedb.term(''body'', %L))', term);
            RAISE EXCEPTION 'expected partial-index predicate to reject an unavailable row identity';
        EXCEPTION WHEN feature_not_supported THEN
            GET STACKED DIAGNOSTICS error_message = MESSAGE_TEXT, error_hint = PG_EXCEPTION_HINT;
            ASSERT error_message = 'search query requires row identity that is unavailable in this context';
            ASSERT error_hint = 'Apply the search operator in a table query. Use an ordinary SQL predicate to define a partial index.';
        END;
    END LOOP;
END;
$$;

SET LOCAL paradedb.enable_custom_scan = off;
SET LOCAL enable_indexscan = off;
SET LOCAL enable_indexonlyscan = off;
SET LOCAL enable_bitmapscan = off;

SELECT array_agg(id ORDER BY id) FROM sequential_scan_unplanned
WHERE id @@@ paradedb.with_index('sequential_scan_unplanned_idx', paradedb.term('body', 'alpha'));
SELECT array_agg(id ORDER BY id) FROM sequential_scan_unplanned
WHERE paradedb.search_with_query_input(id, paradedb.term('body', 'beta'));
SELECT paradedb.search_with_query_input(1, paradedb.empty()) AS empty_match,
       paradedb.search_with_query_input(1,
           paradedb.with_index('sequential_scan_unplanned_idx', paradedb.all())) AS all_match;

CREATE TABLE sequential_scan_unplanned_insert (id int NOT NULL, body text NOT NULL);
CREATE INDEX sequential_scan_unplanned_insert_idx ON sequential_scan_unplanned_insert
USING paradedb (id, body);
CREATE INDEX sequential_scan_unplanned_insert_partial ON sequential_scan_unplanned_insert (id)
WHERE id @@@ paradedb.with_index('sequential_scan_unplanned_insert_idx', paradedb.term('body', 'alpha'));

DO $$
BEGIN
    BEGIN
        INSERT INTO sequential_scan_unplanned_insert VALUES (1, 'alpha');
        RAISE EXCEPTION 'expected predicate evaluation during insert to reject an unavailable row identity';
    EXCEPTION WHEN feature_not_supported THEN
        NULL;
    END;
    ASSERT NOT EXISTS (SELECT FROM sequential_scan_unplanned_insert);
END;
$$;

ROLLBACK;
