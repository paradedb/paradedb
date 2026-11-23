-- Add the native vector type while preserving existing pgvector index objects.

ALTER EXTENSION pg_search DROP OPERATOR CLASS public.vector_l2_ops USING bm25;
ALTER EXTENSION pg_search DROP OPERATOR FAMILY public.vector_l2_ops USING bm25;
ALTER EXTENSION pg_search DROP OPERATOR CLASS public.vector_cosine_ops USING bm25;
ALTER EXTENSION pg_search DROP OPERATOR FAMILY public.vector_cosine_ops USING bm25;
ALTER EXTENSION pg_search DROP OPERATOR CLASS public.vector_ip_ops USING bm25;
ALTER EXTENSION pg_search DROP OPERATOR FAMILY public.vector_ip_ops USING bm25;
ALTER EXTENSION pg_search DROP OPERATOR CLASS public.vector_l2_ops USING paradedb;
ALTER EXTENSION pg_search DROP OPERATOR FAMILY public.vector_l2_ops USING paradedb;
ALTER EXTENSION pg_search DROP OPERATOR CLASS public.vector_cosine_ops USING paradedb;
ALTER EXTENSION pg_search DROP OPERATOR FAMILY public.vector_cosine_ops USING paradedb;
ALTER EXTENSION pg_search DROP OPERATOR CLASS public.vector_ip_ops USING paradedb;
ALTER EXTENSION pg_search DROP OPERATOR FAMILY public.vector_ip_ops USING paradedb;

CREATE  FUNCTION "vector_typmod_out"(
	"typmod" INT /* i32 */
) RETURNS cstring /* CString */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'vector_typmod_out_wrapper';

CREATE  FUNCTION "vector_typmod_in"(
	"parts" cstring[] /* Array < & CStr > */
) RETURNS INT /* i32 */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'vector_typmod_in_wrapper';

CREATE SCHEMA IF NOT EXISTS pdb; CREATE TYPE pdb.vector;

CREATE  FUNCTION "vector_in"(
	"input" cstring, /* & CStr */
	"_typoid" oid, /* pg_sys :: Oid */
	"typmod" INT /* i32 */
) RETURNS pdb.vector /* PgVector */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'vector_in_wrapper';


CREATE  FUNCTION "vector_out"(
	"input" pdb.vector /* PgVector */
) RETURNS cstring /* CString */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'vector_out_wrapper';


CREATE  FUNCTION "vector_recv"(
	"input" internal, /* Internal */
	"_typoid" oid, /* pg_sys :: Oid */
	"typmod" INT /* i32 */
) RETURNS pdb.vector /* PgVector */
IMMUTABLE PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'vector_recv_wrapper';


CREATE  FUNCTION "vector_send"(
	"input" pdb.vector /* PgVector */
) RETURNS bytea /* Vec < u8 > */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'vector_send_wrapper';





CREATE TYPE pdb.vector (
    INPUT = paradedb.vector_in,
    OUTPUT = paradedb.vector_out,
    RECEIVE = paradedb.vector_recv,
    SEND = paradedb.vector_send,
    TYPMOD_IN = paradedb.vector_typmod_in,
    TYPMOD_OUT = paradedb.vector_typmod_out,
    INTERNALLENGTH = VARIABLE,
    ALIGNMENT = int4,
    STORAGE = external
);

CREATE  FUNCTION "vector_ge"(
	"left" pdb.vector, /* PgVector */
	"right" pdb.vector /* PgVector */
) RETURNS bool /* bool */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'vector_ge_wrapper';

CREATE  FUNCTION "vector_negative_inner_product"(
	"left" pdb.vector, /* PgVector */
	"right" pdb.vector /* PgVector */
) RETURNS double precision /* f64 */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'vector_negative_inner_product_wrapper';

CREATE  FUNCTION "vector_cmp"(
	"left" pdb.vector, /* PgVector */
	"right" pdb.vector /* PgVector */
) RETURNS INT /* i32 */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'vector_cmp_wrapper';

CREATE  FUNCTION "vector_cosine_distance"(
	"left" pdb.vector, /* PgVector */
	"right" pdb.vector /* PgVector */
) RETURNS double precision /* f64 */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'vector_cosine_distance_wrapper';

CREATE  FUNCTION "vector_gt"(
	"left" pdb.vector, /* PgVector */
	"right" pdb.vector /* PgVector */
) RETURNS bool /* bool */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'vector_gt_wrapper';

CREATE  FUNCTION "vector_eq"(
	"left" pdb.vector, /* PgVector */
	"right" pdb.vector /* PgVector */
) RETURNS bool /* bool */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'vector_eq_wrapper';

CREATE  FUNCTION "vector_l2_distance"(
	"left" pdb.vector, /* PgVector */
	"right" pdb.vector /* PgVector */
) RETURNS double precision /* f64 */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'vector_l2_distance_wrapper';

CREATE  FUNCTION "vector_le"(
	"left" pdb.vector, /* PgVector */
	"right" pdb.vector /* PgVector */
) RETURNS bool /* bool */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'vector_le_wrapper';

CREATE  FUNCTION "float8_array_to_vector"(
	"input" double precision[], /* Array < f64 > */
	"typmod" INT, /* i32 */
	"_explicit" bool /* bool */
) RETURNS pdb.vector /* PgVector */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'float8_array_to_vector_wrapper';

DO $migration$
DECLARE vector_schema text;
BEGIN
    SELECT n.nspname INTO vector_schema
    FROM pg_extension e JOIN pg_namespace n ON n.oid = e.extnamespace
    WHERE e.extname = 'vector';
    PERFORM set_config('search_path', format('pg_catalog, %I, paradedb, pg_temp', vector_schema), true);
END;
$migration$;
DROP FUNCTION IF EXISTS paradedb.vector_estimator_info(regclass, text, vector[]);
SET LOCAL search_path = paradedb, pg_temp;


CREATE FUNCTION paradedb.vector_estimator_info(
    index regclass,
    field text,
    queries pdb.vector[] DEFAULT NULL
) RETURNS TABLE(
    depth integer,
    bias real,
    spread real,
    sample_rows integer,
    query_count integer,
    query_source text
)
STABLE PARALLEL UNSAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'vector_estimator_info_internal_wrapper';

CREATE  FUNCTION "vector_ne"(
	"left" pdb.vector, /* PgVector */
	"right" pdb.vector /* PgVector */
) RETURNS bool /* bool */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'vector_ne_wrapper';

CREATE  FUNCTION "vector_coerce"(
	"input" pdb.vector, /* PgVector */
	"typmod" INT, /* i32 */
	"_explicit" bool /* bool */
) RETURNS pdb.vector /* PgVector */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'vector_coerce_wrapper';

CREATE  FUNCTION "float4_array_to_vector"(
	"input" real[], /* Array < f32 > */
	"typmod" INT, /* i32 */
	"_explicit" bool /* bool */
) RETURNS pdb.vector /* PgVector */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'float4_array_to_vector_wrapper';

CREATE  FUNCTION "vector_to_float4_array"(
	"input" pdb.vector /* PgVector */
) RETURNS real[] /* Vec < f32 > */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'vector_to_float4_array_wrapper';

CREATE  FUNCTION "vector_lt"(
	"left" pdb.vector, /* PgVector */
	"right" pdb.vector /* PgVector */
) RETURNS bool /* bool */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'vector_lt_wrapper';

CREATE  FUNCTION "vector_dims"(
	"input" pdb.vector /* PgVector */
) RETURNS INT /* i32 */
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c /* Rust */
AS 'MODULE_PATHNAME', 'vector_dims_wrapper';



CREATE CAST (pdb.vector AS pdb.vector)
    WITH FUNCTION paradedb.vector_coerce(pdb.vector, integer, boolean) AS IMPLICIT;
CREATE CAST (real[] AS pdb.vector)
    WITH FUNCTION paradedb.float4_array_to_vector(real[], integer, boolean) AS ASSIGNMENT;
CREATE CAST (double precision[] AS pdb.vector)
    WITH FUNCTION paradedb.float8_array_to_vector(double precision[], integer, boolean) AS ASSIGNMENT;
CREATE CAST (pdb.vector AS real[])
    WITH FUNCTION paradedb.vector_to_float4_array(pdb.vector);
CREATE OPERATOR pg_catalog.<-> (LEFTARG = pdb.vector, RIGHTARG = pdb.vector,
    FUNCTION = paradedb.vector_l2_distance, COMMUTATOR = OPERATOR(pg_catalog.<->));
CREATE OPERATOR pg_catalog.<#> (LEFTARG = pdb.vector, RIGHTARG = pdb.vector,
    FUNCTION = paradedb.vector_negative_inner_product, COMMUTATOR = OPERATOR(pg_catalog.<#>));
CREATE OPERATOR pg_catalog.<=> (LEFTARG = pdb.vector, RIGHTARG = pdb.vector,
    FUNCTION = paradedb.vector_cosine_distance, COMMUTATOR = OPERATOR(pg_catalog.<=>));
CREATE OPERATOR pg_catalog.< (LEFTARG = pdb.vector, RIGHTARG = pdb.vector,
    FUNCTION = paradedb.vector_lt, COMMUTATOR = OPERATOR(pg_catalog.>), NEGATOR = OPERATOR(pg_catalog.>=));
CREATE OPERATOR pg_catalog.<= (LEFTARG = pdb.vector, RIGHTARG = pdb.vector,
    FUNCTION = paradedb.vector_le, COMMUTATOR = OPERATOR(pg_catalog.>=), NEGATOR = OPERATOR(pg_catalog.>));
CREATE OPERATOR pg_catalog.= (LEFTARG = pdb.vector, RIGHTARG = pdb.vector,
    FUNCTION = paradedb.vector_eq, COMMUTATOR = OPERATOR(pg_catalog.=), NEGATOR = OPERATOR(pg_catalog.<>), MERGES);
CREATE OPERATOR pg_catalog.<> (LEFTARG = pdb.vector, RIGHTARG = pdb.vector,
    FUNCTION = paradedb.vector_ne, COMMUTATOR = OPERATOR(pg_catalog.<>), NEGATOR = OPERATOR(pg_catalog.=));
CREATE OPERATOR pg_catalog.>= (LEFTARG = pdb.vector, RIGHTARG = pdb.vector,
    FUNCTION = paradedb.vector_ge, COMMUTATOR = OPERATOR(pg_catalog.<=), NEGATOR = OPERATOR(pg_catalog.<));
CREATE OPERATOR pg_catalog.> (LEFTARG = pdb.vector, RIGHTARG = pdb.vector,
    FUNCTION = paradedb.vector_gt, COMMUTATOR = OPERATOR(pg_catalog.<), NEGATOR = OPERATOR(pg_catalog.<=));
CREATE OPERATOR CLASS pdb.vector_ops DEFAULT FOR TYPE pdb.vector USING btree AS
    OPERATOR 1 pg_catalog.<, OPERATOR 2 pg_catalog.<=, OPERATOR 3 pg_catalog.=, OPERATOR 4 pg_catalog.>=, OPERATOR 5 pg_catalog.>,
    FUNCTION 1 paradedb.vector_cmp(pdb.vector, pdb.vector);



CREATE OPERATOR CLASS pdb.vector_l2_ops DEFAULT FOR TYPE pdb.vector USING bm25 AS STORAGE pdb.vector;
CREATE OPERATOR CLASS pdb.vector_cosine_ops FOR TYPE pdb.vector USING bm25 AS STORAGE pdb.vector;
CREATE OPERATOR CLASS pdb.vector_ip_ops FOR TYPE pdb.vector USING bm25 AS STORAGE pdb.vector;
CREATE OPERATOR CLASS pdb.vector_l2_ops DEFAULT FOR TYPE pdb.vector USING paradedb AS STORAGE pdb.vector;
CREATE OPERATOR CLASS pdb.vector_cosine_ops FOR TYPE pdb.vector USING paradedb AS STORAGE pdb.vector;
CREATE OPERATOR CLASS pdb.vector_ip_ops FOR TYPE pdb.vector USING paradedb AS STORAGE pdb.vector;



CREATE FUNCTION pdb.setup_pgvector() RETURNS void
LANGUAGE plpgsql SET search_path = pg_catalog, pg_temp AS $compat$
DECLARE
    vector_schema text;
    vector_type oid;
    native_type oid := 'pdb.vector'::regtype;
    cast_row record;
    existing_cast record;
    metric text;
    method text;
BEGIN
    SELECT n.nspname INTO vector_schema
    FROM pg_catalog.pg_extension e
    JOIN pg_catalog.pg_namespace n ON n.oid = e.extnamespace
    WHERE e.extname = 'vector';
    IF vector_schema IS NULL THEN
        RETURN;
    END IF;
    vector_type := pg_catalog.format('%I.vector', vector_schema)::regtype;
    FOR cast_row IN SELECT * FROM (VALUES
        (vector_type, native_type, 'i', 'IMPLICIT'),
        (native_type, vector_type, 'a', 'ASSIGNMENT')
    ) AS casts(source_type, target_type, context, keyword)
    LOOP
        SELECT castmethod, castcontext INTO existing_cast
        FROM pg_catalog.pg_cast
        WHERE castsource = cast_row.source_type AND casttarget = cast_row.target_type;
        IF FOUND THEN
            IF existing_cast.castmethod <> 'b' OR existing_cast.castcontext <> cast_row.context THEN
                RAISE EXCEPTION 'incompatible existing cast between % and %',
                    cast_row.source_type::regtype, cast_row.target_type::regtype;
            END IF;
        ELSE
            EXECUTE pg_catalog.format('CREATE CAST (%s AS %s) WITHOUT FUNCTION AS %s',
                cast_row.source_type::regtype, cast_row.target_type::regtype, cast_row.keyword);
        END IF;
        IF EXISTS (
            SELECT FROM pg_catalog.pg_depend d
            JOIN pg_catalog.pg_cast c ON c.oid = d.objid
            WHERE d.classid = 'pg_catalog.pg_cast'::regclass AND d.deptype = 'e'
                AND d.refobjid = (SELECT oid FROM pg_catalog.pg_extension WHERE extname = 'pg_search')
                AND c.castsource = cast_row.source_type AND c.casttarget = cast_row.target_type
        ) THEN
            EXECUTE pg_catalog.format('ALTER EXTENSION pg_search DROP CAST (%s AS %s)',
                cast_row.source_type::regtype, cast_row.target_type::regtype);
        END IF;
    END LOOP;
    FOREACH method IN ARRAY ARRAY['bm25', 'paradedb'] LOOP
        FOREACH metric IN ARRAY ARRAY['l2', 'cosine', 'ip'] LOOP
            IF NOT EXISTS (
                SELECT FROM pg_catalog.pg_opclass c
                JOIN pg_catalog.pg_am a ON a.oid = c.opcmethod
                WHERE c.opcintype = vector_type AND c.opcname = 'vector_' || metric || '_ops'
                    AND a.amname = method
            ) THEN
                EXECUTE pg_catalog.format(
                    'CREATE OPERATOR CLASS %I.%I %s FOR TYPE %I.vector USING %I AS STORAGE %I.vector',
                    vector_schema, 'vector_' || metric || '_ops',
                    CASE WHEN metric = 'l2' THEN 'DEFAULT' ELSE '' END,
                    vector_schema, method, vector_schema);
            END IF;
            IF EXISTS (
                SELECT FROM pg_catalog.pg_depend d
                JOIN pg_catalog.pg_opclass c ON c.oid = d.objid
                JOIN pg_catalog.pg_namespace n ON n.oid = c.opcnamespace
                JOIN pg_catalog.pg_am a ON a.oid = c.opcmethod
                JOIN pg_catalog.pg_extension e ON e.oid = d.refobjid
                WHERE d.classid = 'pg_catalog.pg_opclass'::regclass AND d.deptype = 'e'
                    AND e.extname = 'pg_search' AND n.nspname = vector_schema
                    AND c.opcname = 'vector_' || metric || '_ops' AND a.amname = method
            ) THEN
                EXECUTE pg_catalog.format('ALTER EXTENSION pg_search DROP OPERATOR CLASS %I.%I USING %I',
                    vector_schema, 'vector_' || metric || '_ops', method);
            END IF;
            IF EXISTS (
                SELECT FROM pg_catalog.pg_depend d
                JOIN pg_catalog.pg_opfamily f ON f.oid = d.objid
                JOIN pg_catalog.pg_namespace n ON n.oid = f.opfnamespace
                JOIN pg_catalog.pg_am a ON a.oid = f.opfmethod
                JOIN pg_catalog.pg_extension e ON e.oid = d.refobjid
                WHERE d.classid = 'pg_catalog.pg_opfamily'::regclass AND d.deptype = 'e'
                    AND e.extname = 'pg_search' AND n.nspname = vector_schema
                    AND f.opfname = 'vector_' || metric || '_ops' AND a.amname = method
            ) THEN
                EXECUTE pg_catalog.format('ALTER EXTENSION pg_search DROP OPERATOR FAMILY %I.%I USING %I',
                    vector_schema, 'vector_' || metric || '_ops', method);
            END IF;
        END LOOP;
    END LOOP;
END;
$compat$;

SELECT pdb.setup_pgvector();
