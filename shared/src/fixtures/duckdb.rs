pub fn setup_duckdb_types_table(table: &str) -> String {
    format!(
        "
        CREATE TABLE {table} (
            bool_col BOOLEAN,
            tinyint_col TINYINT,
            smallint_col SMALLINT,
            integer_col INTEGER,
            bigint_col BIGINT,
            utinyint_col UTINYINT,
            usmallint_col USMALLINT,
            uinteger_col UINTEGER,
            ubigint_col UBIGINT,
            float_col FLOAT,
            double_col DOUBLE,
            timestamp_col TIMESTAMP,
            date_col DATE,
            time_col TIME,
            interval_col INTERVAL,
            hugeint_col HUGEINT,
            uhugeint_col UHUGEINT,
            varchar_col VARCHAR,
            blob_col BLOB,
            decimal_col DECIMAL,
            timestamp_s_col TIMESTAMP_S,
            timestamp_ms_col TIMESTAMP_MS,
            timestamp_ns_col TIMESTAMP_NS,
            list_col INTEGER[],
            struct_col STRUCT(a VARCHAR, b INTEGER),
            array_col INTEGER[3],
            uuid_col UUID,
            time_tz_col TIMETZ,
            timestamp_tz_col TIMESTAMPTZ
        );
        
        INSERT INTO {table} VALUES (
            TRUE,
            127,
            32767,
            2147483647,
            9223372036854775807,
            255,
            65535,
            4294967295,
            18446744073709551615,
            1.23,
            2.34,
            '2023-06-27 12:34:56',
            '2023-06-27',
            '12:34:56',
            INTERVAL '1 day',
            12345678901234567890,
            12345678901234567890,
            'Example text',
            '\x41',
            12345.67,
            '2023-06-27 12:34:56',
            '2023-06-27 12:34:56.789',
            '2023-06-27 12:34:56.789123',
            [1, 2, 3],
            ROW('abc', 123),
            [1, 2, 3],
            '550e8400-e29b-41d4-a716-446655440000',
            '12:34:56+02',
            '2023-06-27 12:34:56+02'
        );
        "
    )
}
