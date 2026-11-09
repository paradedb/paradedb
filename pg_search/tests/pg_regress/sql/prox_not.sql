-- Tests pdb.prox_not, the proximity exclusion (Lucene's span_not)

CREATE EXTENSION IF NOT EXISTS pg_search;
DROP TABLE IF EXISTS prox_not;
CREATE TABLE prox_not (
    id serial8,
    text text
);
INSERT INTO prox_not (text) VALUES ('no more timeout');                                  -- 1
INSERT INTO prox_not (text) VALUES ('no more timeout then timeout on node 2');           -- 2
INSERT INTO prox_not (text) VALUES ('no more retry on timeout');                         -- 3
INSERT INTO prox_not (text) VALUES ('timeout fixed');                                    -- 4
INSERT INTO prox_not (text) VALUES ('timeout not fixed');                                -- 5
INSERT INTO prox_not (text) VALUES ('timeout error and timeout on node 2');              -- 6
INSERT INTO prox_not (text) VALUES ('timeout error');                                    -- 7
INSERT INTO prox_not (text) VALUES ('no more timeout reported');                         -- 8
INSERT INTO prox_not (text) VALUES ('disk read latency fixed');                          -- 9
INSERT INTO prox_not (text) VALUES ('disk read latency');                                -- 10
INSERT INTO prox_not (text) VALUES ('flaky test timeout');                               -- 11
CREATE INDEX idxprox_not ON prox_not USING paradedb (id, (text::pdb.simple('pnorms=true')));

--
-- `pre`: tokens allowed between the end of the exclusion and the occurrence
--

-- 2, 3, 4, 5, 6, 7, 11: an adjacent exclusion only
SELECT id FROM prox_not WHERE text @@@ pdb.prox_not('timeout', pdb.prox_clause_in_order('no', 0, 'more'), 0, -1) ORDER BY id;
SELECT id FROM prox_not WHERE text @@@ pdb.prox_not('timeout', pdb.prox_clause_in_order('no', 0, 'more'), 1, -1) ORDER BY id;
-- 3 leaves: two tokens sit between `no more` and `timeout`; so does the second `timeout` of 2
SELECT id FROM prox_not WHERE text @@@ pdb.prox_not('timeout', pdb.prox_clause_in_order('no', 0, 'more'), 2, -1) ORDER BY id;

--
-- `post`: tokens allowed between the occurrence and the start of the exclusion
--

-- 1, 2, 3, 5, 6, 7, 8, 11: only 4 has `fixed` right after `timeout`
SELECT id FROM prox_not WHERE text @@@ pdb.prox_not('timeout', 'fixed', post => 0) ORDER BY id;
-- 5 leaves too
SELECT id FROM prox_not WHERE text @@@ pdb.prox_not('timeout', 'fixed', post => 1) ORDER BY id;

--
-- overlap only, the defaults: an exclusion that contains the occurrence
--

-- 6 keeps its second `timeout`, 7 has none left
SELECT id FROM prox_not WHERE text @@@ pdb.prox_not('timeout', pdb.prox_clause_in_order('timeout', 0, 'error')) ORDER BY id;
-- an exclusion with a gap covers what lies inside it: 8 leaves, 1 does not
SELECT id FROM prox_not WHERE text @@@ pdb.prox_not('timeout', pdb.prox_clause_in_order('no', 3, 'reported')) ORDER BY id;

--
-- a multi-token include is excluded end to end
--

-- 10 only
SELECT id FROM prox_not WHERE text @@@ pdb.prox_not(
    pdb.prox_clause_in_order(pdb.prox_clause_in_order('disk', 0, 'read'), 0, 'latency'),
    'fixed', post => 0) ORDER BY id;

--
-- several exclusions
--

-- each element of an array is its own exclusion: 2, 3, 5, 6, 7, 11
SELECT id FROM prox_not WHERE text @@@ pdb.prox_not('timeout',
    pdb.prox_array(pdb.prox_clause_in_order('no', 0, 'more'), 'fixed'), 0, 0) ORDER BY id;
-- a chain applies every exclusion with its own window: 5, 6, 7
SELECT id FROM prox_not WHERE text @@@ pdb.prox_not(
    pdb.prox_not(pdb.prox_not('timeout', pdb.prox_clause_in_order('no', 0, 'more'), 2, -1), 'fixed', -1, 0),
    pdb.prox_clause_in_order('flaky', 0, 'test'), 0, -1) ORDER BY id;
-- an empty array excludes nothing: 1, 2, 3, 4, 5, 6, 7, 8, 11
SELECT id FROM prox_not WHERE text @@@ pdb.prox_not('timeout', pdb.prox_array(VARIADIC ARRAY[]::pdb.ProximityClause[])) ORDER BY id;
-- nor does it empty a proximity around it: 2, 6
SELECT id FROM prox_not WHERE text @@@ (pdb.prox_not('timeout', pdb.prox_array(VARIADIC ARRAY[]::pdb.ProximityClause[])) ##>1##> 'node') ORDER BY id;

--
-- regex exclusions
--

-- 1, 2, 3, 6, 7, 8, 11
SELECT id FROM prox_not WHERE text @@@ pdb.prox_not('timeout', pdb.prox_regex('fix.*'), -1, 1) ORDER BY id;
-- 1, 2, 3, 6, 7, 8, 11: an exclusion expands past `max_expansions`, like `##` does
SELECT id FROM prox_not WHERE text @@@ pdb.prox_not('timeout', pdb.prox_regex('fix.*|not', 1), -1, 1) ORDER BY id;

--
-- nested in a proximity: only surviving occurrences pair with `node`, 2 and 6
--

SELECT id FROM prox_not WHERE text @@@ (pdb.prox_not('timeout', pdb.prox_clause_in_order('no', 0, 'more'), 0, -1) ##>1##> 'node') ORDER BY id;
SELECT id FROM prox_not WHERE text @@@ pdb.proximity_in_order(pdb.prox_not('timeout', 'error', -1, 0), 1, 'node') ORDER BY id;

--
-- scoring, boosting, highlighting and planning
--

SELECT id, pdb.score(id) > 0 AS scored FROM prox_not WHERE text @@@ pdb.prox_not('timeout', pdb.prox_clause_in_order('no', 0, 'more'), 0, -1) ORDER BY id;
SELECT id, pdb.score(id) > 0 AS scored FROM prox_not WHERE text @@@ pdb.prox_not('timeout', 'fixed', post => 0)::pdb.boost(2) ORDER BY id;
SELECT id, pdb.score(id) = 1 AS constant FROM prox_not WHERE text @@@ pdb.prox_not('timeout', 'fixed', post => 0)::pdb.const(1) ORDER BY id;
-- excluded terms are not highlighted
SELECT id, pdb.snippet(text) FROM prox_not WHERE text @@@ pdb.prox_not('timeout', pdb.prox_clause_in_order('no', 0, 'more'), 0, -1) AND id = 2;
SELECT id FROM prox_not WHERE text @@@ pdb.proximity(pdb.prox_not('timeout', 'fixed', post => 0)) ORDER BY id;
EXPLAIN (COSTS OFF) SELECT id FROM prox_not WHERE text @@@ pdb.prox_not('timeout', 'fixed', post => 0);

--
-- json representation and argument checks
--

SELECT pdb.prox_not('a', 'b');
SELECT pdb.prox_not('a', pdb.prox_clause('b', 1, 'c'), 2, 3);
SELECT pdb.prox_not('a', 'b', -2, 0);   -- error
SELECT pdb.prox_not('a', 'b', 0, -2);   -- error
SELECT pdb.prox_not('a' ##3, 'b');      -- error: incomplete include
SELECT pdb.prox_not('a', 'b' ##3);      -- error: incomplete exclude

--
-- the score counts surviving occurrences: same document, two survivors beat one
--

SELECT (SELECT pdb.score(id) FROM prox_not WHERE text @@@ pdb.prox_not('timeout', 'nothing') AND id = 2)
     > (SELECT pdb.score(id) FROM prox_not WHERE text @@@ pdb.prox_not('timeout', pdb.prox_clause_in_order('no', 0, 'more'), 0, -1) AND id = 2)
    AS fewer_survivors_score_lower;

--
-- other entry points: a regex include, and the legacy `paradedb.proximity` builder
--

-- 1, 2, 3, 4, 5, 6, 8, 11
SELECT id FROM prox_not WHERE text @@@ pdb.prox_not(pdb.prox_regex('time.*'), pdb.prox_clause_in_order('timeout', 0, 'error')) ORDER BY id;
-- 1, 2, 3, 5, 6, 7, 8, 11
SELECT id FROM prox_not WHERE id @@@ paradedb.proximity('text', pdb.prox_not('timeout', 'fixed', post => 0)) ORDER BY id;

--
-- rows inserted after the index land in another segment
--

INSERT INTO prox_not (text) VALUES ('no more timeout on node 3');                        -- 12
INSERT INTO prox_not (text) VALUES ('timeout on node 4');                                -- 13
-- 2, 3, 4, 5, 6, 7, 11, 13
SELECT id FROM prox_not WHERE text @@@ pdb.prox_not('timeout', pdb.prox_clause_in_order('no', 0, 'more'), 0, -1) ORDER BY id;
-- 2, 6, 13
SELECT id FROM prox_not WHERE text @@@ (pdb.prox_not('timeout', pdb.prox_clause_in_order('no', 0, 'more'), 0, -1) ##>1##> 'node') ORDER BY id;

--
-- pairings
--

DROP TABLE IF EXISTS prox_not_pairs;
CREATE TABLE prox_not_pairs (
    id serial8,
    text text
);
INSERT INTO prox_not_pairs (text) VALUES ('shoes not red shoes');                         -- 1
INSERT INTO prox_not_pairs (text) VALUES ('a x c x b');                                   -- 2
INSERT INTO prox_not_pairs (text) VALUES ('a x x x b c');                                 -- 3
INSERT INTO prox_not_pairs (text) VALUES ('red shop');                                    -- 4
INSERT INTO prox_not_pairs (text) VALUES ('red shoes not');                               -- 5
INSERT INTO prox_not_pairs (text) VALUES ('red shoes');                                   -- 6
INSERT INTO prox_not_pairs (text) VALUES ('not red boots');                               -- 7
INSERT INTO prox_not_pairs (text) VALUES ('red boots');                                   -- 8
CREATE INDEX idxprox_not_pairs ON prox_not_pairs USING paradedb (id, (text::pdb.simple('pnorms=true')));

-- an exclusion rejects one pairing, not the others: 1 keeps `red shoes`, 1, 5, 6
SELECT id FROM prox_not_pairs WHERE text @@@ pdb.prox_not(pdb.prox_clause('red', 1, 'shoes'), 'not') ORDER BY id;

-- an exclusion that matches nothing changes nothing
-- 1, 5, 6
SELECT id FROM prox_not_pairs WHERE text @@@ ('red' ##1## 'shoes') ORDER BY id;
SELECT id FROM prox_not_pairs WHERE text @@@ pdb.prox_not(pdb.prox_clause('red', 1, 'shoes'), 'zzz') ORDER BY id;
-- 3: `c` inside `a … b` does not follow it
SELECT id FROM prox_not_pairs WHERE text @@@ (pdb.prox_clause_in_order('a', 5, 'b') ##>0##> 'c') ORDER BY id;
SELECT id FROM prox_not_pairs WHERE text @@@ pdb.prox_not(pdb.prox_clause_in_order('a', 5, 'b') ##>0##> 'c', 'zzz') ORDER BY id;
-- 1, 4, 5, 6: a regex include expands like it does in `##`
SELECT id FROM prox_not_pairs WHERE text @@@ (pdb.prox_regex('sho.*', 1) ##1## 'red') ORDER BY id;
SELECT id FROM prox_not_pairs WHERE text @@@ pdb.prox_not(pdb.prox_clause(pdb.prox_regex('sho.*', 1), 1, 'red'), 'zzz') ORDER BY id;

-- on the right of a proximity: 1, 6
SELECT id FROM prox_not_pairs WHERE text @@@ ('red' ##>0##> pdb.prox_not('shoes', 'not', -1, 0)) ORDER BY id;
-- in an array next to a term: 1, 6, 7, 8
SELECT id FROM prox_not_pairs WHERE text @@@ ('red' ##>0##> pdb.prox_array(pdb.prox_not('shoes', 'not', -1, 0), 'boots')) ORDER BY id;
-- in an exclusion: a `red` right after `not` no longer excludes the `shoes` that follows it, 1
SELECT id FROM prox_not_pairs WHERE text @@@ pdb.prox_not('shoes', pdb.prox_not('red', 'not', 0, -1), 0, -1) ORDER BY id;
-- error: incomplete include inside an array
SELECT pdb.prox_not(pdb.prox_array('x', 'a' ##3), 'b');

--
-- around `prox_not`, a nested match pairs by its ends, not by its inner positions
--

DROP TABLE IF EXISTS prox_not_spans;
CREATE TABLE prox_not_spans (
    id serial8,
    text text
);
INSERT INTO prox_not_spans (text) VALUES ('a c x x b');                                   -- 1
INSERT INTO prox_not_spans (text) VALUES ('a x x x b c');                                 -- 2
INSERT INTO prox_not_spans (text) VALUES ('c a x x b');                                   -- 3
CREATE INDEX idxprox_not_spans ON prox_not_spans USING paradedb (id, (text::pdb.simple('pnorms=true')));

-- 2: `c` inside `a … b` does not follow it, however deep the `prox_not`
SELECT id FROM prox_not_spans WHERE text @@@ (pdb.prox_not(pdb.prox_clause_in_order('a', 5, 'b'), 'zzz') ##>0##> 'c') ORDER BY id;
SELECT id FROM prox_not_spans WHERE text @@@ pdb.prox_not(pdb.prox_not(pdb.prox_clause_in_order('a', 5, 'b'), 'zzz') ##>0##> 'c', 'zzz') ORDER BY id;
-- 3: nor does it precede it
SELECT id FROM prox_not_spans WHERE text @@@ ('c' ##>2##> pdb.prox_not(pdb.prox_clause_in_order('a', 5, 'b'), 'zzz')) ORDER BY id;
