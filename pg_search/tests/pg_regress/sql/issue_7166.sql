CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE issue_7166 (id integer PRIMARY KEY, payload jsonb);
INSERT INTO issue_7166 VALUES
    (1, '{"n": 1}'),
    (2, '{"n": 5}'),
    (3, '{"n": 5.5}'),
    (4, '{"n": 6}'),
    (5, '{"n": 6.5}'),
    (6, '{"n": 8}');
CREATE INDEX issue_7166_idx ON issue_7166
USING paradedb (id, (payload::pdb.unicode_words('columnar=true')));
ANALYZE issue_7166;

SET paradedb.planner_warnings = 'off';
SET paradedb.enable_custom_scan = on;

-- Integer bounds on a JSON path that also holds fractional values keep their kind
SELECT array_agg(id ORDER BY id) FROM issue_7166 WHERE (payload -> 'n') @@@ '[5 TO 5]';
SELECT array_agg(id ORDER BY id) FROM issue_7166 WHERE (payload -> 'n') @@@ '{5 TO 6]';
SELECT array_agg(id ORDER BY id) FROM issue_7166 WHERE (payload -> 'n') @@@ '{5 TO 6}';
SELECT array_agg(id ORDER BY id) FROM issue_7166 WHERE (payload -> 'n') @@@ '[5 TO 6}';

-- PostgreSQL's answers for the same ranges
SELECT array_agg(id ORDER BY id) FROM issue_7166 WHERE '[5,5]'::numrange @> (payload->>'n')::numeric;
SELECT array_agg(id ORDER BY id) FROM issue_7166 WHERE '(5,6]'::numrange @> (payload->>'n')::numeric;
SELECT array_agg(id ORDER BY id) FROM issue_7166 WHERE '(5,6)'::numrange @> (payload->>'n')::numeric;
SELECT array_agg(id ORDER BY id) FROM issue_7166 WHERE '[5,6)'::numrange @> (payload->>'n')::numeric;

-- Same results with the Custom Scan off
SET paradedb.enable_custom_scan = off;
SELECT array_agg(id ORDER BY id) FROM issue_7166 WHERE (payload -> 'n') @@@ '[5 TO 5]';
SELECT array_agg(id ORDER BY id) FROM issue_7166 WHERE (payload -> 'n') @@@ '{5 TO 6]';
SELECT array_agg(id ORDER BY id) FROM issue_7166 WHERE (payload -> 'n') @@@ '{5 TO 6}';
RESET paradedb.enable_custom_scan;
RESET paradedb.planner_warnings;

DROP TABLE issue_7166;
