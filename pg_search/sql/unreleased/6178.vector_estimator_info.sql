-- The extension control file fixes the installation schema to `paradedb`.

CREATE FUNCTION paradedb.vector_estimator_info(
    index regclass,
    field text,
    queries vector[] DEFAULT NULL
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
