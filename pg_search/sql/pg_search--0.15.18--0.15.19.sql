DROP FUNCTION IF EXISTS index_info(index regclass, show_invisible bool);
CREATE OR REPLACE FUNCTION index_info(index regclass, show_invisible bool DEFAULT false)
    RETURNS TABLE
            (
                index_name        text,
                visible           bool,
                recyclable        bool,
                xmax              xid,
                segno             text,
                byte_size         pg_catalog."numeric",
                num_docs          pg_catalog."numeric",
                num_deleted       pg_catalog."numeric",
                termdict_bytes    pg_catalog."numeric",
                postings_bytes    pg_catalog."numeric",
                positions_bytes   pg_catalog."numeric",
                fast_fields_bytes pg_catalog."numeric",
                fieldnorms_bytes  pg_catalog."numeric",
                store_bytes       pg_catalog."numeric",
                deletes_bytes     pg_catalog."numeric"
            )
AS
'MODULE_PATHNAME',
'index_info_wrapper' LANGUAGE c STRICT;
DROP FUNCTION IF EXISTS merge_info(index regclass);
CREATE OR REPLACE FUNCTION merge_info(index regclass)
    RETURNS TABLE
            (
                index_name text,
                pid        pg_catalog.int4,
                xmin       xid,
                segno      text
            )
AS
'MODULE_PATHNAME',
'merge_info_wrapper' LANGUAGE c STRICT;

ALTER FUNCTION paradedb.jsonb_to_searchqueryinput IMMUTABLE STRICT PARALLEL SAFE;
