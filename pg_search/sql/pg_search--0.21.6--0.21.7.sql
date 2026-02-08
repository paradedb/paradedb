\echo Use "ALTER EXTENSION pg_search UPDATE TO '0.21.7'" to load this file. \quit

--- BEGIN SUGGESTED UPGRADE SCRIPT ---
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/mod.rs:230
-- requires:
--   text_to_fieldname

;
    CREATE CAST (varchar AS paradedb.fieldname) WITH INOUT AS IMPLICIT;
CREATE CAST (text AS paradedb.fieldname) WITH INOUT AS IMPLICIT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:563
-- requires:
--   regex_pattern_definition


        CREATE CAST (text AS pdb.regex_pattern) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.regex_pattern) WITH INOUT AS IMPLICIT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:484
-- requires:
--   lindera_definition


        CREATE CAST (text AS pdb.lindera) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.lindera) WITH INOUT AS IMPLICIT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:442
-- requires:
--   literal_definition


        CREATE CAST (text AS pdb.literal) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.literal) WITH INOUT AS IMPLICIT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:470
-- requires:
--   chinese_compatible_definition


        CREATE CAST (text AS pdb.chinese_compatible) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.chinese_compatible) WITH INOUT AS IMPLICIT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:543
-- requires:
--   ngram_definition


        CREATE CAST (text AS pdb.ngram) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.ngram) WITH INOUT AS IMPLICIT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:580
-- requires:
--   unicode_words_definition


        CREATE CAST (text AS pdb.unicode_words) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.unicode_words) WITH INOUT AS IMPLICIT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:456
-- requires:
--   literal_normalized_definition


        CREATE CAST (text AS pdb.literal_normalized) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.literal_normalized) WITH INOUT AS IMPLICIT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:529
-- requires:
--   icu_definition


        CREATE CAST (text AS pdb.icu) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.icu) WITH INOUT AS IMPLICIT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:515
-- requires:
--   source_code_definition


        CREATE CAST (text AS pdb.source_code) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.source_code) WITH INOUT AS IMPLICIT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:498
-- requires:
--   jieba_definition


        CREATE CAST (text AS pdb.jieba) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.jieba) WITH INOUT AS IMPLICIT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:428
-- requires:
--   whitespace_definition


        CREATE CAST (text AS pdb.whitespace) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.whitespace) WITH INOUT AS IMPLICIT;
/* </end connected objects> */

/* <begin connected objects> */
-- pg_search/src/api/tokenizers/definitions.rs:414
-- requires:
--   simple_definition


        CREATE CAST (text AS pdb.simple) WITH INOUT AS IMPLICIT;
CREATE CAST (varchar AS pdb.simple) WITH INOUT AS IMPLICIT;
--- END SUGGESTED UPGRADE SCRIPT ---

DROP FUNCTION IF EXISTS score(_relation_reference anyelement);
CREATE OR REPLACE FUNCTION score(relation_reference anyelement) RETURNS pg_catalog.float4 AS 'MODULE_PATHNAME', 'paradedb_score_from_relation_wrapper' COST 1 LANGUAGE c PARALLEL SAFE STABLE STRICT;
DROP FUNCTION IF EXISTS pdb.score(_relation_reference anyelement);
CREATE OR REPLACE FUNCTION pdb.score(relation_reference anyelement) RETURNS pg_catalog.float4 AS 'MODULE_PATHNAME', 'score_from_relation_wrapper' COST 1 LANGUAGE c PARALLEL SAFE STABLE STRICT;
