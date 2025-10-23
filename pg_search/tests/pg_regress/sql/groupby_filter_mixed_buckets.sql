\i common/groupby_filter_setup.sql

SELECT
    COUNT(*) FILTER (WHERE category @@@ 'electronics') AS f1,
    COUNT(*) FILTER (WHERE category @@@ 'clothing') AS f2,
    COUNT(*) FILTER (WHERE category @@@ 'books') AS f3,
    COUNT(*) FILTER (WHERE category @@@ 'sports') AS f4,
    COUNT(*) FILTER (WHERE brand @@@ 'Apple') AS f5,
    COUNT(*) FILTER (WHERE brand @@@ 'Samsung') AS f6,
    COUNT(*) FILTER (WHERE brand @@@ 'TechPress') AS f7,
    COUNT(*) FILTER (WHERE status @@@ 'available') AS f8,
    COUNT(*) FILTER (WHERE status @@@ 'sold') AS f9,
    COUNT(*) FILTER (WHERE rating >= 4) AS f10,
    COUNT(*) FILTER (WHERE rating >= 5) AS f11,
    COUNT(*) FILTER (WHERE in_stock = true) AS f12
FROM filter_agg_test;

DROP TABLE filter_agg_test CASCADE;
