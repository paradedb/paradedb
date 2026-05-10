DROP FUNCTION IF EXISTS version_info();

CREATE OR REPLACE FUNCTION version_info() RETURNS TABLE(version text, build_mode text) AS 'MODULE_PATHNAME', 'version_info_wrapper' LANGUAGE c STRICT;
