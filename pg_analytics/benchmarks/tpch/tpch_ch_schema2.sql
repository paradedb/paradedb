
create table nation  ( n_nationkey  integer not null,
                            n_name       char(25) not null,
                            n_regionkey  integer not null,
                            n_comment    varchar(152)) engine MergeTree() order by n_nationkey;

create table region  ( r_regionkey  integer not null,
                            r_name       char(25) not null,
                            r_comment    varchar(152)) engine MergeTree() order by r_regionkey;

create table part  ( p_partkey     integer not null,
                          p_name        varchar(55) not null,
                          p_mfgr        char(25) not null,
                          p_brand       char(10) not null,
                          p_type        varchar(25) not null,
                          p_size        integer not null,
                          p_container   char(10) not null,
                          p_retailprice decimal(15,2) not null,
                          p_comment     varchar(23) not null ) engine MergeTree() order by p_partkey;

create table supplier ( s_suppkey     integer not null,
                             s_name        char(25) not null,
                             s_address     varchar(40) not null,
                             s_nationkey   integer not null,
                             s_phone       char(15) not null,
                             s_acctbal     decimal(15,2) not null,
                             s_comment     varchar(101) not null) engine MergeTree() order by s_suppkey;

create table partsupp ( ps_partkey     integer not null,
                             ps_suppkey     integer not null,
                             ps_availqty    integer not null,
                             ps_supplycost  decimal(15,2)  not null,
                             ps_comment     varchar(199) not null ) engine MergeTree() order by ps_partkey;

create table customer ( c_custkey     integer not null,
                             c_name        varchar(25) not null,
                             c_address     varchar(40) not null,
                             c_nationkey   integer not null,
                             c_phone       char(15) not null,
                             c_acctbal     decimal(15,2)   not null,
                             c_mktsegment  char(10) not null,
                             c_comment     varchar(117) not null) engine MergeTree() order by c_custkey;

create table orders  ( o_orderkey       bigint not null,
                           o_custkey        integer not null,
                           o_orderstatus    char(1) not null,
                           o_totalprice     decimal(15,2) not null,
                           o_orderdate      date not null,
                           o_orderpriority  char(15) not null,
                           o_clerk          char(15) not null,
                           o_shippriority   integer not null,
                           o_comment        varchar(79) not null) engine MergeTree() order by o_orderkey;

create table lineitem ( l_orderkey    bigint not null,
                             l_partkey     integer not null,
                             l_suppkey     integer not null,
                             l_linenumber  integer not null,
                             l_quantity    decimal(15,2) not null,
                             l_extendedprice  decimal(15,2) not null,
                             l_discount    decimal(15,2) not null,
                             l_tax         decimal(15,2) not null,
                             l_returnflag  char(1) not null,
                             l_linestatus  char(1) not null,
                             l_shipdate    date not null,
                             l_commitdate  date not null,
                             l_receiptdate date not null,
                             l_shipinstruct char(25) not null,
                             l_shipmode     char(10) not null,
                             l_comment      varchar(44) not null) engine MergeTree() order by l_orderkey;

-- SHOW TABLES;

SET format_csv_delimiter = '|';

INSERT INTO customer FROM INFILE 'TPC-H_V3.0.1/dbgen/customer.tbl' FORMAT CSV;
INSERT INTO lineitem FROM INFILE 'TPC-H_V3.0.1/dbgen/lineitem.tbl' FORMAT CSV;
INSERT INTO nation FROM INFILE 'TPC-H_V3.0.1/dbgen/nation.tbl' FORMAT CSV;
INSERT INTO orders FROM INFILE 'TPC-H_V3.0.1/dbgen/orders.tbl' FORMAT CSV;
INSERT INTO part FROM INFILE 'TPC-H_V3.0.1/dbgen/part.tbl' FORMAT CSV;
INSERT INTO partsupp FROM INFILE 'TPC-H_V3.0.1/dbgen/partsupp.tbl' FORMAT CSV;
INSERT INTO region FROM INFILE 'TPC-H_V3.0.1/dbgen/region.tbl' FORMAT CSV;
INSERT INTO supplier FROM INFILE 'TPC-H_V3.0.1/dbgen/supplier.tbl' FORMAT CSV;

-- SELECT * FROM customer LIMIT 5;



-- The two TPC-H queries with JOINs
-- We run each 3 times like we do in our own benchmarking framework
-- Q13
SELECT 
    c_count,
    COUNT(*) AS custdist 
FROM 
(
    SELECT 
        c_count,
        COUNT() AS custdist
    FROM 
    (
        SELECT 
            c_custkey,
            COUNT() AS c_count
        FROM 
            customer
        LEFT JOIN 
            orders 
        ON 
            c_custkey = o_custkey 
            AND o_comment NOT LIKE '%special%requests%'
        GROUP BY 
            c_custkey
    ) AS c_orders 
    GROUP BY 
        c_count
) 
GROUP BY 
    c_count 
ORDER BY 
    custdist DESC, 
    c_count DESC;

SELECT 
    c_count,
    COUNT(*) AS custdist 
FROM 
(
    SELECT 
        c_count,
        COUNT() AS custdist
    FROM 
    (
        SELECT 
            c_custkey,
            COUNT() AS c_count
        FROM 
            customer
        LEFT JOIN 
            orders 
        ON 
            c_custkey = o_custkey 
            AND o_comment NOT LIKE '%special%requests%'
        GROUP BY 
            c_custkey
    ) AS c_orders 
    GROUP BY 
        c_count
) 
GROUP BY 
    c_count 
ORDER BY 
    custdist DESC, 
    c_count DESC;

SELECT 
    c_count,
    COUNT(*) AS custdist 
FROM 
(
    SELECT 
        c_count,
        COUNT() AS custdist
    FROM 
    (
        SELECT 
            c_custkey,
            COUNT() AS c_count
        FROM 
            customer
        LEFT JOIN 
            orders 
        ON 
            c_custkey = o_custkey 
            AND o_comment NOT LIKE '%special%requests%'
        GROUP BY 
            c_custkey
    ) AS c_orders 
    GROUP BY 
        c_count
) 
GROUP BY 
    c_count 
ORDER BY 
    custdist DESC, 
    c_count DESC;
