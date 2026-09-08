BEGIN;
SET LOCAL paradedb.planner_warnings = off;
SET LOCAL client_min_messages = error;

CREATE TYPE required_fields_composite AS (upper_body text);
CREATE TABLE required_fields (id int NOT NULL, body text, divisor int NOT NULL);
INSERT INTO required_fields VALUES (1, 'alpha', 1), (2, 'alpha', 0);
CREATE INDEX required_fields_idx ON required_fields
USING paradedb (
    id, body,
    (substring(body FROM 1 FOR (10 / divisor))::pdb.simple('alias=danger')),
    (lower(body)::pdb.literal('alias=lower_body')),
    (ROW(upper(body))::required_fields_composite)
) WHERE divisor <> 0;

SET LOCAL paradedb.enable_custom_scan = off;
SET LOCAL enable_indexscan = off;
SET LOCAL enable_indexonlyscan = off;
SET LOCAL enable_bitmapscan = off;

SELECT array_agg(id ORDER BY id) FROM required_fields WHERE body === 'alpha';
SELECT array_agg(id ORDER BY id) FROM required_fields WHERE id @@@ 'body:alpha';
SELECT array_agg(id ORDER BY id) FROM required_fields
WHERE id @@@ paradedb.parse('body:alpha');

INSERT INTO required_fields VALUES (3, 'beta', 0), (4, NULL, 0);

-- Required expressions retain their original positions after skipping danger.
SELECT array_agg(id ORDER BY id) FROM required_fields WHERE lower(body) === 'alpha';
SELECT array_agg(id ORDER BY id) FROM required_fields WHERE id @@@ 'upper_body:ALPHA';
SELECT array_agg(id ORDER BY id) FROM required_fields
WHERE body @@@ 'alpha OR lower_body:beta';
SELECT array_agg(id ORDER BY id) FROM required_fields
WHERE id @@@ paradedb.parse('body:alpha OR lower_body:beta', lenient => true);
SELECT array_agg(id ORDER BY id) FROM required_fields
WHERE id @@@ paradedb.term_set(ARRAY[
    paradedb.term('body', 'alpha'), paradedb.term('lower_body', 'beta')
]);
SELECT array_agg(id ORDER BY id) FROM required_fields
WHERE id @@@ paradedb.boost(2.0, paradedb.boolean(
    must => paradedb.exists('lower_body'),
    must_not => paradedb.term('lower_body', 'beta')
));

-- The NULL guard still needs its field even when the query matches no documents.
SELECT id, lower(body) @@@ pdb.empty() AS empty_match,
       id @@@ paradedb.term('lower_body', 'alpha') AS term_match
FROM required_fields ORDER BY id;
SELECT array_agg(id ORDER BY id) FROM required_fields WHERE id @@@ paradedb.all();

SET LOCAL plan_cache_mode = force_generic_plan;
PREPARE required_fields_parse(text) AS
SELECT array_agg(id ORDER BY id) FROM required_fields WHERE id @@@ $1;
EXECUTE required_fields_parse('body:alpha');
EXECUTE required_fields_parse('lower_body:beta');
EXECUTE required_fields_parse('upper_body:ALPHA');
EXECUTE required_fields_parse('lower_body:[alpha TO beta]');
EXECUTE required_fields_parse('lower_body:IN [alpha beta]');
EXECUTE required_fields_parse('lower_body:*');
EXECUTE required_fields_parse('*');
DEALLOCATE required_fields_parse;

SAVEPOINT required_danger;
SELECT array_agg(id ORDER BY id) FROM required_fields WHERE id @@@ 'danger:alpha';
ROLLBACK TO required_danger;
SELECT array_agg(id ORDER BY id) FROM required_fields
WHERE id @@@ paradedb.parse('alpha', lenient => true);
ROLLBACK TO required_danger;

ROLLBACK;
