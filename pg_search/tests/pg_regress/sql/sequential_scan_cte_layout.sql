BEGIN;

CREATE TABLE sequential_scan_cte_layout (id text NOT NULL, body text, extra text);
INSERT INTO sequential_scan_cte_layout VALUES
    ('one', 'alpha', 'beta'),
    ('two', 'beta', 'alpha');
CREATE INDEX sequential_scan_cte_layout_idx ON sequential_scan_cte_layout
USING paradedb (id, body, extra);

WITH c AS MATERIALIZED (
    SELECT id, extra, body FROM sequential_scan_cte_layout
)
SELECT array_agg(id ORDER BY id) FROM c WHERE body = 'alpha';

WITH c AS MATERIALIZED (
    SELECT id, extra, body FROM sequential_scan_cte_layout
)
SELECT array_agg(id ORDER BY id) FROM c WHERE id @@@ 'body:alpha';

WITH c AS MATERIALIZED (SELECT * FROM sequential_scan_cte_layout)
SELECT array_agg(id ORDER BY id) FROM c WHERE c @@@ 'alpha';

WITH c AS MATERIALIZED (SELECT * FROM sequential_scan_cte_layout)
SELECT array_agg(id ORDER BY id) FROM c WHERE c @@@ 'body:alpha';

WITH c AS MATERIALIZED (SELECT * FROM sequential_scan_cte_layout)
SELECT array_agg(id ORDER BY id) FROM c WHERE c @@@ paradedb.term('extra', 'alpha');

SET LOCAL paradedb.enable_custom_scan = off;
SET LOCAL enable_indexscan = off;
SET LOCAL enable_indexonlyscan = off;
SET LOCAL enable_bitmapscan = off;

WITH c AS MATERIALIZED (
    SELECT id, extra, body FROM sequential_scan_cte_layout
)
SELECT array_agg(id ORDER BY id) FROM c WHERE id @@@ 'body:alpha';

WITH c AS MATERIALIZED (
    SELECT id, extra, body FROM sequential_scan_cte_layout
)
SELECT array_agg(id ORDER BY id) FROM c WHERE id @@@ 'extra:alpha';

WITH c (extra, id, body) AS MATERIALIZED (
    SELECT body, id, extra FROM sequential_scan_cte_layout
)
SELECT array_agg(id ORDER BY id) FROM c WHERE c @@@ 'body:alpha';

WITH c AS MATERIALIZED (
    SELECT id, extra, body FROM sequential_scan_cte_layout
), d AS MATERIALIZED (
    SELECT body, extra, id FROM c
)
SELECT array_agg(id ORDER BY id) FROM d WHERE d @@@ 'extra:alpha';

SELECT array_agg(id ORDER BY id)
FROM (SELECT extra, id, body FROM sequential_scan_cte_layout OFFSET 0) s
WHERE s @@@ 'body:alpha';

SELECT array_agg(id ORDER BY id)
FROM (SELECT extra, id, body FROM sequential_scan_cte_layout ORDER BY id DESC LIMIT 1) s
WHERE s @@@ 'body:alpha';

CREATE TABLE sequential_scan_cte_mixed (
    id integer NOT NULL, discarded text, body text, extra integer, enabled boolean
);
ALTER TABLE sequential_scan_cte_mixed DROP COLUMN discarded;
INSERT INTO sequential_scan_cte_mixed VALUES
    (1, 'ALPHA', 42, true),
    (2, 'BETA', 7, false),
    (3, NULL, 42, true);
CREATE INDEX sequential_scan_cte_mixed_idx ON sequential_scan_cte_mixed
USING paradedb (id, (lower(body)::pdb.literal('alias=normalized_body')), extra, enabled);

WITH c AS MATERIALIZED (
    SELECT id, enabled, extra, body, 'padding'::text AS padding
    FROM sequential_scan_cte_mixed
)
SELECT array_agg(id ORDER BY id) FROM c WHERE id @@@ 'normalized_body:alpha';

WITH c AS MATERIALIZED (
    SELECT id, enabled, extra, body, 'padding'::text AS padding
    FROM sequential_scan_cte_mixed
)
SELECT array_agg(id ORDER BY id) FROM c WHERE id @@@ 'extra:42';

WITH c AS MATERIALIZED (
    SELECT id, enabled, extra, body, 'padding'::text AS padding
    FROM sequential_scan_cte_mixed
)
SELECT array_agg(id ORDER BY id) FROM c WHERE NOT (id @@@ 'normalized_body:alpha');

WITH c AS MATERIALIZED (
    SELECT 'padding'::text AS padding, enabled, body, extra, id
    FROM sequential_scan_cte_mixed
)
SELECT array_agg(id ORDER BY id) FROM c WHERE c @@@ 'alpha';

WITH c AS MATERIALIZED (SELECT * FROM sequential_scan_cte_mixed)
SELECT v.id, c @@@ 'normalized_body:alpha' AS matches, c @@@ paradedb.all() AS all_match
FROM (VALUES (1), (3), (4)) v(id) LEFT JOIN c USING (id) ORDER BY v.id;

DROP INDEX sequential_scan_cte_mixed_idx;
CREATE INDEX sequential_scan_cte_mixed_idx ON sequential_scan_cte_mixed
USING paradedb (id, (lower(body)::pdb.literal('alias=normalized_body')), extra, enabled)
WHERE enabled;

WITH c AS MATERIALIZED (SELECT * FROM sequential_scan_cte_mixed)
SELECT array_agg(id ORDER BY id) FROM c WHERE c @@@ 'normalized_body:beta';

SET LOCAL plan_cache_mode = force_generic_plan;
PREPARE sequential_scan_cte_whole_row(text) AS
WITH c AS MATERIALIZED (SELECT * FROM sequential_scan_cte_mixed)
SELECT array_agg(id ORDER BY id) FROM c WHERE c @@@ $1;
EXECUTE sequential_scan_cte_whole_row('normalized_body:alpha');
EXECUTE sequential_scan_cte_whole_row('normalized_body:beta');
DEALLOCATE sequential_scan_cte_whole_row;

ROLLBACK;
