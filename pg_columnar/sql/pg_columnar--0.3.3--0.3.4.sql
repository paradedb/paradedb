\echo Use "ALTER EXTENSION pg_columnar UPDATE TO '0.3.4'" to load this file. \quit

DO $$
BEGIN
IF NOT EXISTS (SELECT FROM pg_catalog.pg_tables
                WHERE schemaname = 'paradedb' AND tablename = 'logs') THEN
    CREATE TABLE paradedb.logs (
        id SERIAL PRIMARY KEY,
        timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        level TEXT NOT NULL,
        module TEXT NOT NULL,
        file TEXT NOT NULL,
        line INTEGER NOT NULL,
        message TEXT NOT NULL,
        json JSON,
        pid INTEGER NOT NULL,
        backtrace TEXT
    );
    ELSE
        RAISE WARNING 'The table paradedb.logs already exists, skipping.';
    END IF;
END $$;
