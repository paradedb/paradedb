-- This files creates some TPC-H tables in the `parquet` format 
-- from pg_analytics, and some in the standard Postgres `heap`
-- format, to simulate a HTAP workload.

-- nation
-- Row-oriented table:
-- While this table might be involved in some analytics queries, it's more likely
-- to be used in transactional scenarios where you may need to retrieve entire
-- nation records at once.
CREATE TABLE IF NOT EXISTS nation
(
  "n_nationkey"  INT,
  "n_name"       CHAR(25),
  "n_regionkey"  INT,
  "n_comment"    VARCHAR(152),
  "n_dummy"      VARCHAR(10),
  PRIMARY KEY ("n_nationkey")
);

-- region
-- Row-oriented table:
-- Similar to the nation table, it's more likely to be accessed in transactional scenarios
-- where you need to retrieve entire region records.
CREATE TABLE IF NOT EXISTS region
(
  "r_regionkey"  INT,
  "r_name"       CHAR(25),
  "r_comment"    VARCHAR(152),
  "r_dummy"      VARCHAR(10),
  PRIMARY KEY ("r_regionkey")
);

-- supplier
-- Row-oriented table:
-- This table contains supplier details which may be accessed in a variety of ways, including
-- both analytics and transactional queries. However, since supplier details might be accessed
-- as a whole in some scenarios, a row-oriented approach could be appropriate.
CREATE TABLE IF NOT EXISTS supplier
(
  "s_suppkey"     INT,
  "s_name"        CHAR(25),
  "s_address"     VARCHAR(40),
  "s_nationkey"   INT,
  "s_phone"       CHAR(15),
  "s_acctbal"     DECIMAL(15,2),
  "s_comment"     VARCHAR(101),
  "s_dummy"       VARCHAR(10),
  PRIMARY KEY ("s_suppkey")
);

-- customer
-- Row-oriented table:
-- Similar to the supplier table, customer details are likely to be accessed in various scenarios,
-- so a row-oriented approach is reasonable.
CREATE TABLE IF NOT EXISTS customer
(
  "c_custkey"     INT,
  "c_name"        VARCHAR(25),
  "c_address"     VARCHAR(40),
  "c_nationkey"   INT,
  "c_phone"       CHAR(15),
  "c_acctbal"     DECIMAL(15,2),
  "c_mktsegment"  CHAR(10),
  "c_comment"     VARCHAR(117),
  "c_dummy"       VARCHAR(10),
  PRIMARY KEY ("c_custkey")
);

-- part
-- Row-oriented table:
-- While parts might be involved in some analytics queries, they're also likely to be accessed in
-- transactional scenarios, making a row-oriented approach suitable.
CREATE TABLE IF NOT EXISTS part
(
  "p_partkey"     INT,
  "p_name"        VARCHAR(55),
  "p_mfgr"        CHAR(25),
  "p_brand"       CHAR(10),
  "p_type"        VARCHAR(25),
  "p_size"        INT,
  "p_container"   CHAR(10),
  "p_retailprice" DECIMAL(15,2) ,
  "p_comment"     VARCHAR(23) ,
  "p_dummy"       VARCHAR(10),
  PRIMARY KEY ("p_partkey")
);

-- partsupp
-- Column-oriented table:
-- This table is used in queries involving aggregation and filtering based on
-- supply-related attributes such as ps_supplycost, ps_availqty, etc.
CREATE TABLE IF NOT EXISTS partsupp
(
  "ps_partkey"     INT,
  "ps_suppkey"     INT,
  "ps_availqty"    INT,
  "ps_supplycost"  DECIMAL(15,2),
  "ps_comment"     VARCHAR(199),
  "ps_dummy"       VARCHAR(10),
  PRIMARY KEY ("ps_partkey")
)
USING parquet;

-- orders
-- Column-oriented table:
-- This table is used in queries involving aggregation and filtering based on
-- order-related attributes such as o_orderdate, o_totalprice, etc.
CREATE TABLE IF NOT EXISTS orders
(
  "o_orderkey"       INT,
  "o_custkey"        INT,
  "o_orderstatus"    CHAR(1),
  "o_totalprice"     DECIMAL(15,2),
  "o_orderdate"      DATE,
  "o_orderpriority"  CHAR(15),
  "o_clerk"          CHAR(15),
  "o_shippriority"   INT,
  "o_comment"        VARCHAR(79),
  "o_dummy"          VARCHAR(10),
  PRIMARY KEY ("o_orderkey")
)
USING parquet;

-- lineitem
-- Column-oriented table:
-- This table is heavily used in analytical queries involving aggregation, grouping,
-- and selection of specific columns such as l_extendedprice, l_discount, etc.
CREATE TABLE IF NOT EXISTS lineitem
(
  "l_orderkey"          INT,
  "l_partkey"           INT,
  "l_suppkey"           INT,
  "l_linenumber"        INT,
  "l_quantity"          DECIMAL(15,2),
  "l_extendedprice"     DECIMAL(15,2),
  "l_discount"          DECIMAL(15,2),
  "l_tax"               DECIMAL(15,2),
  "l_returnflag"        CHAR(1),
  "l_linestatus"        CHAR(1),
  "l_shipdate"          DATE,
  "l_commitdate"        DATE,
  "l_receiptdate"       DATE,
  "l_shipinstruct"      CHAR(25),
  "l_shipmode"          CHAR(10),
  "l_comment"           VARCHAR(44),
  "l_dummy"             VARCHAR(10)
)
USING parquet;
