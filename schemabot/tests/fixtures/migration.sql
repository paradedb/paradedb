DROP FUNCTION IF EXISTS public.vector_info(index regclass, field text);
CREATE FUNCTION public.vector_info(index regclass, field text) RETURNS TABLE(name text, value bigint) LANGUAGE sql AS $$SELECT 'new', 1::bigint$$;
COMMENT ON ACCESS METHOD paradedb IS 'new';
