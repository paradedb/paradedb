-- Test for disjunctive search queries executed as joins in ParadeDB (JoinScan).
-- Covers 2-table and 3-table transitive cross-table OR search queries,
-- compound predicates, and circuit breaker fallback scenarios.

SET max_parallel_workers = 0;
SET max_parallel_workers_per_gather = 0;
SET parallel_leader_participation = off;
SET enable_indexscan to OFF;

CREATE EXTENSION IF NOT EXISTS pg_search;

-- =============================================================================
-- 2-TABLE SETUP: Products and Suppliers
-- =============================================================================
DROP TABLE IF EXISTS products CASCADE;
DROP TABLE IF EXISTS suppliers CASCADE;

CREATE TABLE products (
    id INTEGER PRIMARY KEY,
    name TEXT,
    description TEXT,
    supplier_id INTEGER,
    price NUMERIC(10,2),
    stock INTEGER
);

CREATE TABLE suppliers (
    id INTEGER PRIMARY KEY,
    name TEXT,
    description TEXT,
    country TEXT,
    rating INTEGER
);

INSERT INTO products (id, name, description, supplier_id, price, stock) VALUES
(1, 'ThinkPad Laptop', 'High performance enterprise laptop computer with linux', 1, 1299.99, 15),
(2, 'Gaming Mouse', 'Wireless ergonomic mouse with high precision sensor', 1, 59.99, 80),
(3, 'Mechanical Keyboard', 'Custom mechanical keyboard with silent switches', 1, 149.99, 45),
(4, 'Curved Monitor', 'Ultra-wide 4K curved display monitor for productivity', 2, 699.99, 20),
(5, '4K Webcam', 'Ultra HD webcam for professional video conferencing', 2, 119.99, 60),
(6, 'Studio Headphones', 'Professional noise canceling studio monitor headphones', 3, 349.99, 25),
(7, 'Podcast Microphone', 'Cardioid condenser USB microphone for broadcasting', 3, 179.99, 35),
(8, 'Bluetooth Speaker', 'Waterproof portable bluetooth speaker with deep bass', 4, 99.99, 50),
(9, 'Drawing Tablet', 'Graphic drawing tablet with pressure sensitive stylus', 4, 499.99, 18),
(10, 'GaN Fast Charger', 'Multi-port USB-C fast wall charger 100W', 5, 49.99, 150),
(11, 'Desk Mat', 'Extra large felt desk mat and mouse pad', 5, 29.99, 90),
(12, 'USB-C Hub', 'Multi-functional aluminum USB-C hub with HDMI and ethernet', 5, 69.99, 70);

INSERT INTO suppliers (id, name, description, country, rating) VALUES
(1, 'LenovoTech', 'Leading global manufacturer of enterprise computer hardware and laptops', 'USA', 5),
(2, 'DisplayVision', 'Innovator in advanced display technology and monitor screens', 'Japan', 4),
(3, 'AudioCraft', 'Specialist in acoustic engineering and high-fidelity sound equipment', 'Germany', 5),
(4, 'DigitalGadgets', 'Distributor of consumer electronics, mobile accessories, and gadgets', 'China', 3),
(5, 'PowerWave', 'Provider of energy solutions, power adapters, and desktop accessories', 'USA', 4);

CREATE INDEX products_bm25_idx ON products USING paradedb (id, name, description, supplier_id, price, stock)
WITH (key_field = 'id', numeric_fields = '{"supplier_id": {"fast": true}, "price": {"fast": true}, "stock": {"fast": true}}');

CREATE INDEX suppliers_bm25_idx ON suppliers USING paradedb (id, name, description, country, rating)
WITH (key_field = 'id', numeric_fields = '{"rating": {"fast": true}}');

SET paradedb.enable_join_custom_scan = on;
SET paradedb.enable_aggregate_custom_scan = on;

-- =============================================================================
-- TEST 1: 2-table cross-table OR on search predicates
-- Matches: p.description matching 'laptop' (p1) OR s.description matching 'display' (s2 -> p4, p5)
-- =============================================================================
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE (p.description @@@ 'laptop' OR s.description @@@ 'display')
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE (p.description @@@ 'laptop' OR s.description @@@ 'display')
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- TEST 2: 2-table cross-table OR with multi-term disjunctions
-- Matches: p.description matching ('keyboard' OR 'microphone') OR s.description matching 'acoustic'
-- =============================================================================
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE (p.description @@@ 'keyboard OR microphone' OR s.description @@@ 'acoustic')
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.name, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE (p.description @@@ 'keyboard OR microphone' OR s.description @@@ 'acoustic')
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- TEST 3: 2-table cross-table OR combined with fast-field range filters
-- Matches: p.price < 100.00 AND (p.description @@@ 'mouse' OR s.description @@@ 'energy')
-- =============================================================================
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, p.price, s.name AS supplier_name, s.rating
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.price < 100.00
  AND (p.description @@@ 'mouse' OR s.description @@@ 'energy')
ORDER BY p.id
LIMIT 10;

SELECT p.id, p.name, p.price, s.name AS supplier_name, s.rating
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE p.price < 100.00
  AND (p.description @@@ 'mouse' OR s.description @@@ 'energy')
ORDER BY p.id
LIMIT 10;

-- =============================================================================
-- TEST 4: 2-table cross-table OR with ORDER BY and LIMIT
-- =============================================================================
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT p.id, p.name, p.price, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE (p.description @@@ 'wireless OR bluetooth OR microphone' OR s.description @@@ 'hardware')
ORDER BY p.price DESC
LIMIT 4;

SELECT p.id, p.name, p.price, s.name AS supplier_name
FROM products p
JOIN suppliers s ON p.supplier_id = s.id
WHERE (p.description @@@ 'wireless OR bluetooth OR microphone' OR s.description @@@ 'hardware')
ORDER BY p.price DESC
LIMIT 4;


-- =============================================================================
-- 3-TABLE SETUP: Work Items, Researchers, Organisations (Chain Schema)
-- w.researcher_id = r.id AND r.org_id = o.id
-- =============================================================================
DROP TABLE IF EXISTS work_items CASCADE;
DROP TABLE IF EXISTS researchers CASCADE;
DROP TABLE IF EXISTS organisations CASCADE;

CREATE TABLE organisations (
    id INTEGER PRIMARY KEY,
    name TEXT,
    location TEXT,
    field TEXT
);

CREATE TABLE researchers (
    id INTEGER PRIMARY KEY,
    name TEXT,
    bio TEXT,
    org_id INTEGER,
    citations INTEGER
);

CREATE TABLE work_items (
    id INTEGER PRIMARY KEY,
    title TEXT,
    abstract TEXT,
    researcher_id INTEGER,
    year INTEGER
);

INSERT INTO organisations (id, name, location, field) VALUES
(101, 'Stanford AI Lab', 'Stanford, CA', 'Artificial Intelligence and Machine Learning'),
(102, 'Max Planck Institute', 'Munich, Germany', 'Quantum Physics and Information Theory'),
(103, 'Cambridge Systems Group', 'Cambridge, UK', 'Operating Systems and Database Engines'),
(104, 'ETH Zurich Systems', 'Zurich, Switzerland', 'Distributed Systems and Cloud Architecture');

INSERT INTO researchers (id, name, bio, org_id, citations) VALUES
(201, 'Alice Smith', 'Specializes in neural network optimization and compiler construction', 101, 1500),
(202, 'Bob Jones', 'Focuses on quantum error correction and topological quantum computing', 102, 2800),
(203, 'Carol White', 'Expert in columnar database storage engines and vector indexing', 103, 3400),
(204, 'David Brown', 'Researching distributed consensus protocols and replication algorithms', 104, 950),
(205, 'Eve Black', 'Working on LLM reasoning architectures and knowledge representation', 101, 1200);

INSERT INTO work_items (id, title, abstract, researcher_id, year) VALUES
(301, 'Deep Learning Compilers', 'Automated optimization of tensor computation graphs for heterogeneous accelerators', 201, 2024),
(302, 'Neural Architecture Search', 'Reinforcement learning for discovering efficient neural network topologies', 201, 2023),
(303, 'Fault-Tolerant Quantum Gates', 'Implementation of surface code error correction on superconducting qubits', 202, 2024),
(304, 'Topological Quantum Memory', 'Braiding non-Abelian anyons for robust quantum information storage', 202, 2022),
(305, 'Columnar Indexing for BM25', 'Fast field bitmap indexing and SIMD vectorization for disjunctive search joins', 203, 2025),
(306, 'LSM-Tree Compaction Strategies', 'Adaptive leveling for write-heavy key-value storage engines', 203, 2023),
(307, 'Byzantine Agreement at Scale', 'Low-latency asynchronous consensus with zero-knowledge membership proofs', 204, 2024),
(308, 'Geo-Replicated State Machines', 'Causal consistency with bounded staleness for planetary-scale applications', 204, 2023),
(309, 'Graph-Augmented Language Models', 'Retrieval augmented generation over dynamic enterprise knowledge graphs', 205, 2025),
(310, 'Prompt Optimization via Gradient Guidance', 'Discrete prompt optimization for domain-adapted foundation models', 205, 2024);

CREATE INDEX organisations_bm25_idx ON organisations USING paradedb (id, name, location, field)
WITH (key_field = 'id');

CREATE INDEX researchers_bm25_idx ON researchers USING paradedb (id, name, bio, org_id, citations)
WITH (key_field = 'id', numeric_fields = '{"org_id": {"fast": true}, "citations": {"fast": true}}');

CREATE INDEX work_items_bm25_idx ON work_items USING paradedb (id, title, abstract, researcher_id, year)
WITH (key_field = 'id', numeric_fields = '{"researcher_id": {"fast": true}, "year": {"fast": true}}');

-- =============================================================================
-- TEST 5: 3-table transitive disjunctive search join (O -> R -> W)
-- Matches:
-- w.abstract @@@ 'compaction' (w306)
-- OR r.bio @@@ 'compiler' (r201 -> w301, w302)
-- OR o.location @@@ 'Munich' (o102 -> r202 -> w303, w304)
-- =============================================================================
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT w.id, w.title, r.name AS researcher_name, o.name AS org_name
FROM work_items w
JOIN researchers r ON w.researcher_id = r.id
JOIN organisations o ON r.org_id = o.id
WHERE (w.abstract @@@ 'compaction' OR r.bio @@@ 'compiler' OR o.location @@@ 'Munich')
ORDER BY w.id
LIMIT 10;

SELECT w.id, w.title, r.name AS researcher_name, o.name AS org_name
FROM work_items w
JOIN researchers r ON w.researcher_id = r.id
JOIN organisations o ON r.org_id = o.id
WHERE (w.abstract @@@ 'compaction' OR r.bio @@@ 'compiler' OR o.location @@@ 'Munich')
ORDER BY w.id
LIMIT 10;

-- =============================================================================
-- TEST 6: 3-table transitive search join with year filter and LIMIT
-- Matches: (w.title @@@ 'quantum' OR r.name @@@ 'Carol' OR o.name @@@ 'Stanford') AND w.year >= 2024
-- =============================================================================
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT w.id, w.title, w.year, r.name AS researcher_name, o.name AS org_name
FROM work_items w
JOIN researchers r ON w.researcher_id = r.id
JOIN organisations o ON r.org_id = o.id
WHERE (w.title @@@ 'quantum' OR r.name @@@ 'Carol' OR o.name @@@ 'Stanford')
  AND w.year >= 2024
ORDER BY w.id
LIMIT 10;

SELECT w.id, w.title, w.year, r.name AS researcher_name, o.name AS org_name
FROM work_items w
JOIN researchers r ON w.researcher_id = r.id
JOIN organisations o ON r.org_id = o.id
WHERE (w.title @@@ 'quantum' OR r.name @@@ 'Carol' OR o.name @@@ 'Stanford')
  AND w.year >= 2024
ORDER BY w.id
LIMIT 10;

-- =============================================================================
-- TEST 7: 3-table transitive search join with search predicates across all 3 tables
-- Matches: w.abstract @@@ 'consensus' OR r.bio @@@ 'distributed' OR o.field @@@ 'Cloud'
-- =============================================================================
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT w.id, w.title, r.name AS researcher_name, o.name AS org_name
FROM work_items w
JOIN researchers r ON w.researcher_id = r.id
JOIN organisations o ON r.org_id = o.id
WHERE (w.abstract @@@ 'consensus' OR r.bio @@@ 'distributed' OR o.field @@@ 'Cloud')
ORDER BY w.id
LIMIT 10;

SELECT w.id, w.title, r.name AS researcher_name, o.name AS org_name
FROM work_items w
JOIN researchers r ON w.researcher_id = r.id
JOIN organisations o ON r.org_id = o.id
WHERE (w.abstract @@@ 'consensus' OR r.bio @@@ 'distributed' OR o.field @@@ 'Cloud')
ORDER BY w.id
LIMIT 10;

-- =============================================================================
-- TEST 8: 3-table aggregate scan with cross-table disjunction
-- =============================================================================
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT count(*)
FROM work_items w
JOIN researchers r ON w.researcher_id = r.id
JOIN organisations o ON r.org_id = o.id
WHERE (w.abstract @@@ 'compaction' OR r.bio @@@ 'compiler' OR o.location @@@ 'Munich');

SELECT count(*)
FROM work_items w
JOIN researchers r ON w.researcher_id = r.id
JOIN organisations o ON r.org_id = o.id
WHERE (w.abstract @@@ 'compaction' OR r.bio @@@ 'compiler' OR o.location @@@ 'Munich');

-- =============================================================================
-- TEST 9: Disjunctive search join with empty match set on one table
-- Matches: (w.title @@@ 'nonexistent_quantum_xyz' OR r.name @@@ 'Carol') AND w.year >= 2024
-- =============================================================================
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT w.id, w.title, w.year, r.name AS researcher_name
FROM work_items w
JOIN researchers r ON w.researcher_id = r.id
WHERE (w.title @@@ 'nonexistent_quantum_xyz' OR r.name @@@ 'Carol')
  AND w.year >= 2024
ORDER BY w.id
LIMIT 10;

SELECT w.id, w.title, w.year, r.name AS researcher_name
FROM work_items w
JOIN researchers r ON w.researcher_id = r.id
WHERE (w.title @@@ 'nonexistent_quantum_xyz' OR r.name @@@ 'Carol')
  AND w.year >= 2024
ORDER BY w.id
LIMIT 10;

-- =============================================================================
-- TEST 10: 3-table disjunctive search join with non-matching branches on 2 tables
-- Matches: w.title @@@ 'NonExistentTitle' OR r.name @@@ 'Alice' OR o.name @@@ 'NonExistentOrg'
-- =============================================================================
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT w.id, w.title, r.name AS researcher_name, o.name AS org_name
FROM work_items w
JOIN researchers r ON w.researcher_id = r.id
JOIN organisations o ON r.org_id = o.id
WHERE (w.title @@@ 'NonExistentTitle' OR r.name @@@ 'Alice' OR o.name @@@ 'NonExistentOrg')
ORDER BY w.id
LIMIT 10;

SELECT w.id, w.title, r.name AS researcher_name, o.name AS org_name
FROM work_items w
JOIN researchers r ON w.researcher_id = r.id
JOIN organisations o ON r.org_id = o.id
WHERE (w.title @@@ 'NonExistentTitle' OR r.name @@@ 'Alice' OR o.name @@@ 'NonExistentOrg')
ORDER BY w.id
LIMIT 10;

-- =============================================================================
-- TEST 11: Compound disjunctive query with exact year filter
-- Matches: (w.title @@@ 'quantum' OR r.bio @@@ 'compiler') AND w.year = 2024
-- =============================================================================
EXPLAIN (COSTS OFF, VERBOSE, TIMING OFF)
SELECT w.id, w.title, w.year, r.name AS researcher_name
FROM work_items w
JOIN researchers r ON w.researcher_id = r.id
WHERE (w.title @@@ 'quantum' OR r.bio @@@ 'compiler')
  AND w.year = 2024
ORDER BY w.id
LIMIT 10;

SELECT w.id, w.title, w.year, r.name AS researcher_name
FROM work_items w
JOIN researchers r ON w.researcher_id = r.id
WHERE (w.title @@@ 'quantum' OR r.bio @@@ 'compiler')
  AND w.year = 2024
ORDER BY w.id
LIMIT 10;

-- =============================================================================
-- CLEANUP
-- =============================================================================
DROP TABLE products CASCADE;
DROP TABLE suppliers CASCADE;
DROP TABLE work_items CASCADE;
DROP TABLE researchers CASCADE;
DROP TABLE organisations CASCADE;
