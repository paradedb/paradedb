\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.21.7'" to load this file. \quit

CREATE CAST (text AS pdb.simple) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.simple) WITH INOUT AS IMPLICIT;
CREATE CAST (text AS pdb.whitespace) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.whitespace) WITH INOUT AS IMPLICIT;
CREATE CAST (text AS pdb.literal) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.literal) WITH INOUT AS IMPLICIT;
CREATE CAST (text AS pdb.literal_normalized) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.literal_normalized) WITH INOUT AS IMPLICIT;
CREATE CAST (text AS pdb.chinese_compatible) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.chinese_compatible) WITH INOUT AS IMPLICIT;
CREATE CAST (text AS pdb.lindera) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.lindera) WITH INOUT AS IMPLICIT;
CREATE CAST (text AS pdb.jieba) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.jieba) WITH INOUT AS IMPLICIT;
CREATE CAST (text AS pdb.source_code) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.source_code) WITH INOUT AS IMPLICIT;
CREATE CAST (text AS pdb.icu) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.icu) WITH INOUT AS IMPLICIT;
CREATE CAST (text AS pdb.ngram) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.ngram) WITH INOUT AS IMPLICIT;
CREATE CAST (text AS pdb.regex_pattern) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.regex_pattern) WITH INOUT AS IMPLICIT;
CREATE CAST (text AS pdb.unicode_words) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.unicode_words) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS paradedb.fieldname) WITH INOUT AS IMPLICIT;

DROP FUNCTION IF EXISTS score(_relation_reference anyelement);
CREATE OR REPLACE FUNCTION score(relation_reference anyelement) RETURNS pg_catalog.float4 AS 'MODULE_PATHNAME', 'paradedb_score_from_relation_wrapper' COST 1 LANGUAGE c PARALLEL SAFE STABLE STRICT;
DROP FUNCTION IF EXISTS pdb.score(_relation_reference anyelement);
CREATE OR REPLACE FUNCTION pdb.score(relation_reference anyelement) RETURNS pg_catalog.float4 AS 'MODULE_PATHNAME', 'score_from_relation_wrapper' COST 1 LANGUAGE c PARALLEL SAFE STABLE STRICT;
