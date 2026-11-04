DO $$
DECLARE
    actual_rows bigint;
BEGIN
    SELECT count(*) INTO actual_rows FROM msmarco;
    IF actual_rows <> 113520750 THEN
        RAISE EXCEPTION 'Expected 113520750 MS MARCO rows, found %', actual_rows;
    END IF;
END
$$;

VACUUM (FREEZE, ANALYZE) msmarco;
