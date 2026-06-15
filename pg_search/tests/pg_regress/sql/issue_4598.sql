-- setup
CREATE TABLE issue_4598_repro (
    id int,
    mock_text text,
    mock_hash text
);

INSERT INTO issue_4598_repro (id, mock_text, mock_hash)
SELECT i, 'test content ' || i, md5(i::text)
FROM generate_series(1, 1000) i;

CREATE INDEX issue_4598_idx ON issue_4598_repro USING bm25 (id, mock_text, mock_hash) WITH (key_field=id);

-- force parallel execution
SET max_parallel_workers_per_gather = 2;
SET parallel_setup_cost = 0;
SET parallel_tuple_cost = 0;
SET min_parallel_table_scan_size = 0;
SET parallel_leader_participation = off;
SET force_parallel_mode = on;

-- trigger InitPlan array parameter in parallel worker
SELECT COUNT(*) FROM issue_4598_repro
WHERE mock_text @@@ paradedb.all()
AND mock_hash = ANY(ARRAY(SELECT mock_hash FROM issue_4598_repro LIMIT 5));

-- trigger prepared statement PARAM_EXTERN in parallel worker
PREPARE issue_4598_prep(text) AS
SELECT COUNT(*) FROM issue_4598_repro
WHERE mock_text @@@ paradedb.all()
AND mock_hash = $1;

EXECUTE issue_4598_prep('098f6bcd4621d373cade4e832627b4f6');

-- trigger prepared statement PARAM_EXTERN parallel worker SIGSEGV
PREPARE issue_4598_prep(text) AS
SELECT COUNT(*) FROM issue_4598_repro
WHERE text_searchable @@@ paradedb.all()
AND doc_id_hash = $1;

EXECUTE issue_4598_prep('098f6bcd4621d373cade4e832627b4f6');

-- cleanup
DROP TABLE issue_4598_repro CASCADE;
