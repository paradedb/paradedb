\i common/common_setup.sql

-- Setup: table and BM25 index matching the ES migration docs examples
CREATE TABLE products (
  id SERIAL PRIMARY KEY,
  description TEXT,
  category TEXT,
  rating INT,
  price NUMERIC,
  created_at TIMESTAMP
);

INSERT INTO products (description, category, rating, price, created_at) VALUES
  ('Sleek running shoes for athletes', 'footwear', 5, 89.99, '2025-01-15'),
  ('Cheap running shoes on sale', 'footwear', 3, 29.99, '2025-02-20'),
  ('Premium leather boots', 'footwear', 4, 149.99, '2025-03-10'),
  ('Wireless bluetooth headphones', 'electronics', 4, 59.99, '2025-04-05'),
  ('Ergonomic mechanical keyboard', 'electronics', 5, 129.99, '2025-05-12'),
  ('Cotton running shorts', 'apparel', 4, 34.99, '2025-06-01'),
  ('Waterproof hiking jacket', 'apparel', 5, 199.99, '2025-07-18'),
  ('Stainless steel water bottle', 'accessories', 4, 24.99, '2025-08-22'),
  ('Yoga mat with carrying strap', 'fitness', 3, 39.99, '2025-09-30'),
  ('Digital fitness tracker watch', 'electronics', 4, 79.99, '2025-10-14');

CREATE INDEX search_idx ON products
USING bm25 (id, description, (category::pdb.literal), rating, price, created_at)
WITH (key_field = 'id');

-------------------------------------------------------------
-- Full-Text Queries
-------------------------------------------------------------

-- match OR (|||)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM products WHERE description ||| 'running shoes' ORDER BY id;
SELECT * FROM products WHERE description ||| 'running shoes' ORDER BY id;

-- match AND (&&&)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM products WHERE description &&& 'running shoes' ORDER BY id;
SELECT * FROM products WHERE description &&& 'running shoes' ORDER BY id;

-- match with fuzziness
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM products WHERE description ||| 'runing shose'::pdb.fuzzy(2) ORDER BY id;
SELECT * FROM products WHERE description ||| 'runing shose'::pdb.fuzzy(2) ORDER BY id;

-- match_phrase (###)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM products WHERE description ### 'running shoes' ORDER BY id;
SELECT * FROM products WHERE description ### 'running shoes' ORDER BY id;

-- match_phrase_prefix
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM products WHERE description @@@ pdb.phrase_prefix(ARRAY['running', 'sh']) ORDER BY id;
SELECT * FROM products WHERE description @@@ pdb.phrase_prefix(ARRAY['running', 'sh']) ORDER BY id;

-- multi_match (OR across fields)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM products WHERE description ||| 'running shoes' OR category ||| 'running shoes' ORDER BY id;
SELECT * FROM products WHERE description ||| 'running shoes' OR category ||| 'running shoes' ORDER BY id;

-- multi_match with disjunction_max
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM products
WHERE id @@@ paradedb.disjunction_max(
  disjuncts => ARRAY[
    paradedb.match('description', 'running shoes'),
    paradedb.match('category', 'running shoes')
  ],
  tie_breaker => 0.3
) ORDER BY id;
SELECT * FROM products
WHERE id @@@ paradedb.disjunction_max(
  disjuncts => ARRAY[
    paradedb.match('description', 'running shoes'),
    paradedb.match('category', 'running shoes')
  ],
  tie_breaker => 0.3
) ORDER BY id;

-- query_string (pdb.parse)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM products WHERE description @@@ pdb.parse('running AND shoes') ORDER BY id;
SELECT * FROM products WHERE description @@@ pdb.parse('running AND shoes') ORDER BY id;

-------------------------------------------------------------
-- Term-Level Queries
-------------------------------------------------------------

-- term (===)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM products WHERE category === 'electronics' ORDER BY id;
SELECT * FROM products WHERE category === 'electronics' ORDER BY id;

-- terms
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM products WHERE category === 'electronics' OR category === 'footwear' ORDER BY id;
SELECT * FROM products WHERE category === 'electronics' OR category === 'footwear' ORDER BY id;

-- range (SQL filter pushdown)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM products WHERE price >= 10 AND price <= 100 ORDER BY id;
SELECT * FROM products WHERE price >= 10 AND price <= 100 ORDER BY id;

-- exists (non-text field)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM products WHERE rating @@@ pdb.exists() ORDER BY id;
SELECT * FROM products WHERE rating @@@ pdb.exists() ORDER BY id;

-- fuzzy term
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM products WHERE description === 'shose'::pdb.fuzzy(2) ORDER BY id;
SELECT * FROM products WHERE description === 'shose'::pdb.fuzzy(2) ORDER BY id;

-- prefix
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM products WHERE description === 'runn'::pdb.fuzzy(0, t) ORDER BY id;
SELECT * FROM products WHERE description === 'runn'::pdb.fuzzy(0, t) ORDER BY id;

-- regexp
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM products WHERE description @@@ pdb.regex('run.*ing') ORDER BY id;
SELECT * FROM products WHERE description @@@ pdb.regex('run.*ing') ORDER BY id;

-- wildcard (via regex)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM products WHERE description @@@ pdb.regex('run.*') ORDER BY id;
SELECT * FROM products WHERE description @@@ pdb.regex('run.*') ORDER BY id;

-- ids
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM products WHERE id IN (1, 2, 3) ORDER BY id;
SELECT * FROM products WHERE id IN (1, 2, 3) ORDER BY id;

-------------------------------------------------------------
-- Compound Queries
-------------------------------------------------------------

-- bool (SQL)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM products
WHERE description ||| 'shoes'
  AND (category === 'footwear' OR TRUE)
  AND NOT (price >= 100)
  AND rating = 5
ORDER BY id;
SELECT * FROM products
WHERE description ||| 'shoes'
  AND (category === 'footwear' OR TRUE)
  AND NOT (price >= 100)
  AND rating = 5
ORDER BY id;

-- bool (paradedb.boolean)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM products
WHERE id @@@ paradedb.boolean(
  must => ARRAY[paradedb.match('description', 'shoes')],
  should => ARRAY[paradedb.term('category', 'footwear')]
)
AND NOT (price >= 100)
AND rating = 5
ORDER BY id;
SELECT * FROM products
WHERE id @@@ paradedb.boolean(
  must => ARRAY[paradedb.match('description', 'shoes')],
  should => ARRAY[paradedb.term('category', 'footwear')]
)
AND NOT (price >= 100)
AND rating = 5
ORDER BY id;

-- boosting
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM products
WHERE id @@@ paradedb.boolean(
  must => ARRAY[paradedb.match('description', 'shoes')],
  should => ARRAY[paradedb.boost(-0.5, paradedb.match('description', 'cheap'))]
) ORDER BY id;
SELECT * FROM products
WHERE id @@@ paradedb.boolean(
  must => ARRAY[paradedb.match('description', 'shoes')],
  should => ARRAY[paradedb.boost(-0.5, paradedb.match('description', 'cheap'))]
) ORDER BY id;

-- constant_score
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM products
WHERE id @@@ paradedb.const_score(
  score => 1.5,
  query => paradedb.term('category', 'electronics')
) ORDER BY id;
SELECT * FROM products
WHERE id @@@ paradedb.const_score(
  score => 1.5,
  query => paradedb.term('category', 'electronics')
) ORDER BY id;

-- dis_max
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM products
WHERE id @@@ paradedb.disjunction_max(
  disjuncts => ARRAY[
    paradedb.match('description', 'shoes'),
    paradedb.term('category', 'footwear')
  ],
  tie_breaker => 0.5
) ORDER BY id;
SELECT * FROM products
WHERE id @@@ paradedb.disjunction_max(
  disjuncts => ARRAY[
    paradedb.match('description', 'shoes'),
    paradedb.term('category', 'footwear')
  ],
  tie_breaker => 0.5
) ORDER BY id;

-------------------------------------------------------------
-- Specialized Queries
-------------------------------------------------------------

-- more_like_this
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM products
WHERE id @@@ pdb.more_like_this(1, ARRAY['description'])
ORDER BY id;
SELECT * FROM products
WHERE id @@@ pdb.more_like_this(1, ARRAY['description'])
ORDER BY id;

-- proximity unordered
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM products WHERE description @@@ ('running' ##3## 'shoes') ORDER BY id;
SELECT * FROM products WHERE description @@@ ('running' ##3## 'shoes') ORDER BY id;

-- proximity ordered
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM products WHERE description @@@ ('running' ##> 3 ##> 'shoes') ORDER BY id;
SELECT * FROM products WHERE description @@@ ('running' ##> 3 ##> 'shoes') ORDER BY id;

-- match_all
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM products WHERE id @@@ pdb.all() ORDER BY id;
SELECT * FROM products WHERE id @@@ pdb.all() ORDER BY id;

-- match_none
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT * FROM products WHERE id @@@ pdb.empty() ORDER BY id;
SELECT * FROM products WHERE id @@@ pdb.empty() ORDER BY id;

-------------------------------------------------------------
-- Scoring and Relevance
-------------------------------------------------------------

-- score
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, description, pdb.score(id)
FROM products WHERE description ||| 'shoes'
ORDER BY pdb.score(id) DESC;
SELECT id, description, pdb.score(id)
FROM products WHERE description ||| 'shoes'
ORDER BY pdb.score(id) DESC;

-- boost
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, description FROM products
WHERE description ||| 'shoes'::pdb.boost(2.0)
ORDER BY pdb.score(id) DESC;
SELECT id, description FROM products
WHERE description ||| 'shoes'::pdb.boost(2.0)
ORDER BY pdb.score(id) DESC;

-------------------------------------------------------------
-- Highlighting
-------------------------------------------------------------

-- snippet with custom tags
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, pdb.snippet(description, start_tag => '<em>', end_tag => '</em>')
FROM products WHERE description ||| 'shoes'
ORDER BY id;
SELECT id, pdb.snippet(description, start_tag => '<em>', end_tag => '</em>')
FROM products WHERE description ||| 'shoes'
ORDER BY id;

-- snippets (multiple)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, pdb.snippets(description, max_num_chars => 20)
FROM products WHERE description ||| 'running shoes'
ORDER BY id;
SELECT id, pdb.snippets(description, max_num_chars => 20)
FROM products WHERE description ||| 'running shoes'
ORDER BY id;

-- snippet_positions
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT id, pdb.snippet(description), pdb.snippet_positions(description)
FROM products WHERE description ||| 'shoes'
ORDER BY id;
SELECT id, pdb.snippet(description), pdb.snippet_positions(description)
FROM products WHERE description ||| 'shoes'
ORDER BY id;

-------------------------------------------------------------
-- Aggregations
-------------------------------------------------------------

-- terms
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT pdb.agg('{"terms": {"field": "category"}}')
FROM products WHERE id @@@ pdb.all();
SELECT pdb.agg('{"terms": {"field": "category"}}')
FROM products WHERE id @@@ pdb.all();

-- histogram
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT pdb.agg('{"histogram": {"field": "rating", "interval": 1}}')
FROM products WHERE id @@@ pdb.all();
SELECT pdb.agg('{"histogram": {"field": "rating", "interval": 1}}')
FROM products WHERE id @@@ pdb.all();

-- date_histogram (fixed_interval only)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT pdb.agg('{"date_histogram": {"field": "created_at", "fixed_interval": "30d"}}')
FROM products WHERE id @@@ pdb.all();
SELECT pdb.agg('{"date_histogram": {"field": "created_at", "fixed_interval": "30d"}}')
FROM products WHERE id @@@ pdb.all();

-- range
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT pdb.agg('{"range": {"field": "rating", "ranges": [{"to": 3}, {"from": 3, "to": 5}, {"from": 5}]}}')
FROM products WHERE id @@@ pdb.all();
SELECT pdb.agg('{"range": {"field": "rating", "ranges": [{"to": 3}, {"from": 3, "to": 5}, {"from": 5}]}}')
FROM products WHERE id @@@ pdb.all();

-- avg
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT pdb.agg('{"avg": {"field": "rating"}}')
FROM products WHERE id @@@ pdb.all();
SELECT pdb.agg('{"avg": {"field": "rating"}}')
FROM products WHERE id @@@ pdb.all();

-- sum
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT pdb.agg('{"sum": {"field": "rating"}}')
FROM products WHERE id @@@ pdb.all();
SELECT pdb.agg('{"sum": {"field": "rating"}}')
FROM products WHERE id @@@ pdb.all();

-- min
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT pdb.agg('{"min": {"field": "rating"}}')
FROM products WHERE id @@@ pdb.all();
SELECT pdb.agg('{"min": {"field": "rating"}}')
FROM products WHERE id @@@ pdb.all();

-- value_count
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT pdb.agg('{"value_count": {"field": "id"}}')
FROM products WHERE id @@@ pdb.all();
SELECT pdb.agg('{"value_count": {"field": "id"}}')
FROM products WHERE id @@@ pdb.all();

-- stats
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT pdb.agg('{"stats": {"field": "rating"}}')
FROM products WHERE id @@@ pdb.all();
SELECT pdb.agg('{"stats": {"field": "rating"}}')
FROM products WHERE id @@@ pdb.all();

-- percentiles
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT pdb.agg('{"percentiles": {"field": "rating"}}')
FROM products WHERE id @@@ pdb.all();
SELECT pdb.agg('{"percentiles": {"field": "rating"}}')
FROM products WHERE id @@@ pdb.all();

-- cardinality
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT pdb.agg('{"cardinality": {"field": "category"}}')
FROM products WHERE id @@@ pdb.all();
SELECT pdb.agg('{"cardinality": {"field": "category"}}')
FROM products WHERE id @@@ pdb.all();

-- top_hits (requires sort and docvalue_fields)
EXPLAIN (FORMAT TEXT, COSTS OFF, TIMING OFF)
SELECT pdb.agg('{"top_hits": {"size": 3, "sort": [{"created_at": "desc"}], "docvalue_fields": ["id", "created_at"]}}')
FROM products WHERE id @@@ pdb.all()
GROUP BY rating
ORDER BY rating;
SELECT pdb.agg('{"top_hits": {"size": 3, "sort": [{"created_at": "desc"}], "docvalue_fields": ["id", "created_at"]}}')
FROM products WHERE id @@@ pdb.all()
GROUP BY rating
ORDER BY rating;

-------------------------------------------------------------
-- Cleanup
-------------------------------------------------------------

DROP TABLE products CASCADE;
