-- Q1: scan-only COUNT
-- @id: q1_count
SELECT COUNT(*) FROM mpp_bench WHERE body @@@ 'term';

-- Q2: low-cardinality GROUP BY
-- @id: q2_groupby_low
SELECT category, COUNT(*) FROM mpp_bench WHERE body @@@ 'term' GROUP BY category ORDER BY category;

-- Q3: high-cardinality GROUP BY
-- @id: q3_groupby_high
SELECT user_id, COUNT(*) FROM mpp_bench WHERE body @@@ 'term' GROUP BY user_id ORDER BY user_id LIMIT 10;

-- Q4: TopN with predicate
-- @id: q4_topn
SELECT id, score FROM mpp_bench WHERE body @@@ 'term' ORDER BY score DESC LIMIT 10;
