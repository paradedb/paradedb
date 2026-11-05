\i common/common_setup.sql

SET paradedb.enable_custom_scan = on;
SET paradedb.enable_filter_pushdown = on;
-- The issue is about the Base Scan; `count(*)` would otherwise plan as the Aggregate Scan.
SET paradedb.enable_aggregate_custom_scan = off;

CREATE TABLE issue_6928_docs (
    id bigint PRIMARY KEY,
    body text
);

INSERT INTO issue_6928_docs
SELECT i, 'search document ' || i
FROM generate_series(1, 100) AS i;

CREATE INDEX issue_6928_docs_idx ON issue_6928_docs USING paradedb (id, body);

-- The lookup stays empty, so the scalar subquery evaluates to NULL while the
-- query runs. A literal `(SELECT NULL::paradedb.searchqueryinput)` would instead
-- be folded to a NULL Const and rejected earlier by qual inspection.
CREATE TABLE issue_6928_lookup (q paradedb.searchqueryinput);

-- Control: the index matches every row.
SELECT count(*) FROM issue_6928_docs WHERE body @@@ 'document';

-- `@@@ NULL` matches nothing.
SELECT count(*) FROM issue_6928_docs
WHERE body @@@ (SELECT q FROM issue_6928_lookup);

-- `NOT (... @@@ NULL)` is NULL, which also matches nothing. Before the fix this
-- returned all 100 rows.
EXPLAIN (COSTS OFF)
SELECT count(*) FROM issue_6928_docs
WHERE NOT (body @@@ (SELECT q FROM issue_6928_lookup));

SELECT count(*) FROM issue_6928_docs
WHERE NOT (body @@@ (SELECT q FROM issue_6928_lookup));

-- NULL AND true is NULL; NULL OR true is true.
SELECT count(*) FROM issue_6928_docs
WHERE body @@@ (SELECT q FROM issue_6928_lookup) AND body @@@ 'document';

SELECT count(*) FROM issue_6928_docs
WHERE body @@@ (SELECT q FROM issue_6928_lookup) OR body @@@ 'document';

-- A negated NULL must not admit rows through AND or OR.
SELECT count(*) FROM issue_6928_docs
WHERE NOT (body @@@ (SELECT q FROM issue_6928_lookup)) AND body @@@ 'document';

SELECT count(*) FROM issue_6928_docs
WHERE NOT (body @@@ (SELECT q FROM issue_6928_lookup)) OR body @@@ 'missing';

-- Negating compound predicates must preserve SQL three-valued logic.
SELECT count(*) FROM issue_6928_docs
WHERE NOT (body @@@ (SELECT q FROM issue_6928_lookup) AND body @@@ 'document');

SELECT count(*) FROM issue_6928_docs
WHERE NOT (body @@@ (SELECT q FROM issue_6928_lookup) OR body @@@ 'missing');

SELECT count(*) FROM issue_6928_docs
WHERE NOT (body @@@ (SELECT q FROM issue_6928_lookup)) OR body @@@ 'document';

-- A NULL parameter reaches the same `Null` query, through the planner's estimate
-- fold for a generic plan.
SET plan_cache_mode = force_generic_plan;

PREPARE issue_6928_positive(paradedb.searchqueryinput) AS
SELECT count(*) FROM issue_6928_docs WHERE body @@@ $1;

EXECUTE issue_6928_positive(NULL);
EXECUTE issue_6928_positive(paradedb.all());

DEALLOCATE issue_6928_positive;

PREPARE issue_6928_negated(paradedb.searchqueryinput) AS
SELECT count(*) FROM issue_6928_docs WHERE NOT (body @@@ $1);

EXECUTE issue_6928_negated(NULL);

DEALLOCATE issue_6928_negated;

RESET plan_cache_mode;

DROP TABLE issue_6928_lookup;
DROP TABLE issue_6928_docs;
