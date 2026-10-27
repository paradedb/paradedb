-- A multi-key `ORDER BY ... LIMIT` over a `GROUP BY` join must reach the
-- DataFusion TopK the same way a single key does. Researchers 1 and 2 share
-- the same `MAX(item_date)`, so the tie-breaker key decides the result.
CREATE EXTENSION IF NOT EXISTS pg_search;

CREATE TABLE multicolumn_topk_researchers (
    researcher_id INT PRIMARY KEY,
    bio TEXT
);

CREATE TABLE multicolumn_topk_work_items (
    work_id INT PRIMARY KEY,
    researcher_id INT NOT NULL,
    category TEXT NOT NULL,
    item_date DATE NOT NULL,
    abstract_text TEXT NOT NULL
);

CREATE INDEX multicolumn_topk_researchers_idx ON multicolumn_topk_researchers
USING bm25 (researcher_id, bio);

CREATE INDEX multicolumn_topk_work_items_idx ON multicolumn_topk_work_items
USING bm25 (
    work_id,
    researcher_id,
    (category::pdb.literal),
    item_date,
    abstract_text
);

-- Two segments per table, so MPP has something to split.
SET paradedb.global_mutable_segment_rows = 0;

INSERT INTO multicolumn_topk_researchers (researcher_id, bio) VALUES
    (1, 'cancer immunology'),
    (2, 'cancer genomics'),
    (3, 'cancer proteomics');
INSERT INTO multicolumn_topk_researchers (researcher_id, bio) VALUES
    (4, 'cancer metabolism'),
    (5, 'cancer therapeutics');

INSERT INTO multicolumn_topk_work_items (work_id, researcher_id, category, item_date, abstract_text) VALUES
    (101, 1, 'Publications',   '2025-06-01', 'cancer study alpha'),
    (102, 1, 'Tradeshows',     '2025-05-01', 'cancer poster alpha'),
    (103, 2, 'Publications',   '2025-06-01', 'cancer study beta'),
    (104, 2, 'ClinicalTrials', '2025-04-01', 'cancer trial beta');
INSERT INTO multicolumn_topk_work_items (work_id, researcher_id, category, item_date, abstract_text) VALUES
    (105, 3, 'Tradeshows',     '2025-05-15', 'cancer poster gamma'),
    (106, 4, 'Funding',        '2025-03-10', 'cancer grant delta'),
    (107, 5, 'Publications',   '2025-01-20', 'cancer review epsilon');

RESET paradedb.global_mutable_segment_rows;

SET paradedb.enable_aggregate_custom_scan TO on;
SET paradedb.enable_join_custom_scan TO on;
SET max_parallel_workers_per_gather TO 0;

-- Single aggregate key, no ties.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT w.researcher_id
FROM multicolumn_topk_work_items w
JOIN multicolumn_topk_researchers r ON w.researcher_id = r.researcher_id
WHERE w.abstract_text @@@ 'cancer' OR r.bio @@@ 'cancer'
GROUP BY w.researcher_id
ORDER BY MIN(w.item_date) ASC
LIMIT 3;

SELECT w.researcher_id
FROM multicolumn_topk_work_items w
JOIN multicolumn_topk_researchers r ON w.researcher_id = r.researcher_id
WHERE w.abstract_text @@@ 'cancer' OR r.bio @@@ 'cancer'
GROUP BY w.researcher_id
ORDER BY MIN(w.item_date) ASC
LIMIT 3;

-- Aggregate key plus a group column tie-breaker.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT w.researcher_id
FROM multicolumn_topk_work_items w
JOIN multicolumn_topk_researchers r ON w.researcher_id = r.researcher_id
WHERE w.abstract_text @@@ 'cancer' OR r.bio @@@ 'cancer'
GROUP BY w.researcher_id
ORDER BY MAX(w.item_date) DESC NULLS LAST, w.researcher_id ASC
LIMIT 3;

SELECT w.researcher_id
FROM multicolumn_topk_work_items w
JOIN multicolumn_topk_researchers r ON w.researcher_id = r.researcher_id
WHERE w.abstract_text @@@ 'cancer' OR r.bio @@@ 'cancer'
GROUP BY w.researcher_id
ORDER BY MAX(w.item_date) DESC NULLS LAST, w.researcher_id ASC
LIMIT 3;

-- The tie sits on the LIMIT boundary, so only the second key picks the row.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT w.researcher_id
FROM multicolumn_topk_work_items w
JOIN multicolumn_topk_researchers r ON w.researcher_id = r.researcher_id
WHERE w.abstract_text @@@ 'cancer' OR r.bio @@@ 'cancer'
GROUP BY w.researcher_id
ORDER BY MAX(w.item_date) DESC NULLS LAST, w.researcher_id DESC
LIMIT 1;

SELECT w.researcher_id
FROM multicolumn_topk_work_items w
JOIN multicolumn_topk_researchers r ON w.researcher_id = r.researcher_id
WHERE w.abstract_text @@@ 'cancer' OR r.bio @@@ 'cancer'
GROUP BY w.researcher_id
ORDER BY MAX(w.item_date) DESC NULLS LAST, w.researcher_id DESC
LIMIT 1;

-- Three keys, the first one a filtered count.
SELECT explain_redact_oid($$
SELECT w.researcher_id
FROM multicolumn_topk_work_items w
JOIN multicolumn_topk_researchers r ON w.researcher_id = r.researcher_id
WHERE w.abstract_text @@@ 'cancer' OR r.bio @@@ 'cancer'
GROUP BY w.researcher_id
ORDER BY
  COUNT(*) FILTER (WHERE w.category = 'Tradeshows' AND w.abstract_text @@@ 'cancer') DESC,
  MAX(w.item_date) DESC NULLS LAST,
  w.researcher_id ASC
LIMIT 3
$$, 'COSTS OFF, VERBOSE, TIMING OFF');

SELECT w.researcher_id
FROM multicolumn_topk_work_items w
JOIN multicolumn_topk_researchers r ON w.researcher_id = r.researcher_id
WHERE w.abstract_text @@@ 'cancer' OR r.bio @@@ 'cancer'
GROUP BY w.researcher_id
ORDER BY
  COUNT(*) FILTER (WHERE w.category = 'Tradeshows' AND w.abstract_text @@@ 'cancer') DESC,
  MAX(w.item_date) DESC NULLS LAST,
  w.researcher_id ASC
LIMIT 3;

-- OFFSET adds to the fetch; the keys stay in the order written.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT w.researcher_id, COUNT(*)
FROM multicolumn_topk_work_items w
JOIN multicolumn_topk_researchers r ON w.researcher_id = r.researcher_id
WHERE w.abstract_text @@@ 'cancer' OR r.bio @@@ 'cancer'
GROUP BY w.researcher_id
ORDER BY COUNT(*) DESC, w.researcher_id DESC
LIMIT 2 OFFSET 1;

SELECT w.researcher_id, COUNT(*)
FROM multicolumn_topk_work_items w
JOIN multicolumn_topk_researchers r ON w.researcher_id = r.researcher_id
WHERE w.abstract_text @@@ 'cancer' OR r.bio @@@ 'cancer'
GROUP BY w.researcher_id
ORDER BY COUNT(*) DESC, w.researcher_id DESC
LIMIT 2 OFFSET 1;

-- One key that cannot be pushed keeps the whole sort in Postgres. A TopK on
-- the first key alone could drop researcher 2 before the tie-breaker runs.
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT w.researcher_id
FROM multicolumn_topk_work_items w
JOIN multicolumn_topk_researchers r ON w.researcher_id = r.researcher_id
WHERE w.abstract_text @@@ 'cancer' OR r.bio @@@ 'cancer'
GROUP BY w.researcher_id
ORDER BY MAX(w.item_date) DESC NULLS LAST, (w.researcher_id % 2) ASC
LIMIT 1;

SELECT w.researcher_id
FROM multicolumn_topk_work_items w
JOIN multicolumn_topk_researchers r ON w.researcher_id = r.researcher_id
WHERE w.abstract_text @@@ 'cancer' OR r.bio @@@ 'cancer'
GROUP BY w.researcher_id
ORDER BY MAX(w.item_date) DESC NULLS LAST, (w.researcher_id % 2) ASC
LIMIT 1;

-- An aggregate key under a collation that is not byte-ordered keeps the
-- whole sort in Postgres as well. The aggregate itself is still pushed.
CREATE COLLATION IF NOT EXISTS multicolumn_topk_icu (provider = icu, locale = 'en-US');

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT w.researcher_id
FROM multicolumn_topk_work_items w
JOIN multicolumn_topk_researchers r ON w.researcher_id = r.researcher_id
WHERE w.abstract_text @@@ 'cancer' OR r.bio @@@ 'cancer'
GROUP BY w.researcher_id
ORDER BY
  MAX(w.item_date) DESC NULLS LAST,
  STRING_AGG(w.category COLLATE "multicolumn_topk_icu", ',' ORDER BY w.category) ASC
LIMIT 1;

SELECT w.researcher_id
FROM multicolumn_topk_work_items w
JOIN multicolumn_topk_researchers r ON w.researcher_id = r.researcher_id
WHERE w.abstract_text @@@ 'cancer' OR r.bio @@@ 'cancer'
GROUP BY w.researcher_id
ORDER BY
  MAX(w.item_date) DESC NULLS LAST,
  STRING_AGG(w.category COLLATE "multicolumn_topk_icu", ',' ORDER BY w.category) ASC
LIMIT 1;

-- Under MPP every task bounds its own output with the multi-key TopK before
-- the rows cross to the leader.
SET paradedb.mpp_min_rows TO 0;
SET max_parallel_workers_per_gather TO 2;
SET max_parallel_workers TO 8;
SET min_parallel_table_scan_size TO 0;
SET parallel_setup_cost TO 0;
SET parallel_tuple_cost TO 0;

EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT w.researcher_id
FROM multicolumn_topk_work_items w
JOIN multicolumn_topk_researchers r ON w.researcher_id = r.researcher_id
WHERE w.abstract_text @@@ 'cancer' OR r.bio @@@ 'cancer'
GROUP BY w.researcher_id
ORDER BY MAX(w.item_date) DESC NULLS LAST, w.researcher_id DESC
LIMIT 1;

SELECT w.researcher_id
FROM multicolumn_topk_work_items w
JOIN multicolumn_topk_researchers r ON w.researcher_id = r.researcher_id
WHERE w.abstract_text @@@ 'cancer' OR r.bio @@@ 'cancer'
GROUP BY w.researcher_id
ORDER BY MAX(w.item_date) DESC NULLS LAST, w.researcher_id DESC
LIMIT 1;

RESET paradedb.mpp_min_rows;
RESET max_parallel_workers_per_gather;
RESET max_parallel_workers;
RESET min_parallel_table_scan_size;
RESET parallel_setup_cost;
RESET parallel_tuple_cost;
RESET paradedb.enable_aggregate_custom_scan;
RESET paradedb.enable_join_custom_scan;

DROP COLLATION multicolumn_topk_icu;
DROP TABLE multicolumn_topk_work_items;
DROP TABLE multicolumn_topk_researchers;
