CREATE EXTENSION IF NOT EXISTS pg_search;
DROP TABLE IF EXISTS pr2625;
CREATE TABLE pr2625 (
    id serial8,
    k text,
    v float8
);
CREATE INDEX idxpr2625 ON pr2625 USING bm25 (id, k, v) WITH (key_field = 'id', target_segment_count = 8, layer_sizes = '1gb, 10gb');

--
-- test with 1 segment
--
INSERT INTO pr2625(k, v) SELECT paradedb.random_words(1), x::float8 from generate_series(1, 1000) x;

SET parallel_leader_participation = true;
SELECT * FROM paradedb.aggregate(index=>'idxpr2625', query=>paradedb.all(), agg=>'{"average": { "avg": { "field": "v" }}}', solve_mvcc=>true);
SELECT * FROM paradedb.aggregate(index=>'idxpr2625', query=>paradedb.all(), agg=>'{"average": { "avg": { "field": "v" }}}', solve_mvcc=>false);
SET parallel_leader_participation = false;
SELECT * FROM paradedb.aggregate(index=>'idxpr2625', query=>paradedb.all(), agg=>'{"average": { "avg": { "field": "v" }}}', solve_mvcc=>true);


--
-- test with multiple segments
--
INSERT INTO pr2625(k, v) SELECT paradedb.random_words(1), x::float8 from generate_series(1, 1000) x;
INSERT INTO pr2625(k, v) SELECT paradedb.random_words(1), x::float8 from generate_series(1, 1000) x;
INSERT INTO pr2625(k, v) SELECT paradedb.random_words(1), x::float8 from generate_series(1, 1000) x;
INSERT INTO pr2625(k, v) SELECT paradedb.random_words(1), x::float8 from generate_series(1, 1000) x;
INSERT INTO pr2625(k, v) SELECT paradedb.random_words(1), x::float8 from generate_series(1, 1000) x;
INSERT INTO pr2625(k, v) SELECT paradedb.random_words(1), x::float8 from generate_series(1, 1000) x;

SET parallel_leader_participation = true;
SELECT * FROM paradedb.aggregate(index=>'idxpr2625', query=>paradedb.all(), agg=>'{"average": { "avg": { "field": "v" }}}', solve_mvcc=>true);
SELECT * FROM paradedb.aggregate(index=>'idxpr2625', query=>paradedb.all(), agg=>'{"average": { "avg": { "field": "v" }}}', solve_mvcc=>false);
SET parallel_leader_participation = false;
SELECT * FROM paradedb.aggregate(index=>'idxpr2625', query=>paradedb.all(), agg=>'{"average": { "avg": { "field": "v" }}}', solve_mvcc=>true);


--
-- this one will execute serially in the current backend
--
SET parallel_leader_participation = false;
SET max_parallel_workers = 0;
SELECT * FROM paradedb.aggregate(index=>'idxpr2625', query=>paradedb.all(), agg=>'{"average": { "avg": { "field": "v" }}}', solve_mvcc=>true);

RESET parallel_leader_participation;
RESET max_parallel_workers;
