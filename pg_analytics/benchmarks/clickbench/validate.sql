DO $$
DECLARE
    row_count INT;
    expected_count INT := 4997472;
BEGIN
    SELECT COUNT(*) INTO row_count FROM your_table_name;
    IF row_count != expected_count THEN
        RAISE EXCEPTION 'Row count does not match. Expected % but found %', expected_count, row_count;
    END IF;
END
$$;
