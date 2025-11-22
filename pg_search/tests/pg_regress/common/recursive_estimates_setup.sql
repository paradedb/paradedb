-- Setup for recursive estimates tests
-- Ensures the GUC is enabled and creates test data if needed

-- First, run common setup for extension and GUCs
\i common/common_setup.sql

-- Create regress.mock_items table (drop first for idempotency)
CREATE SCHEMA IF NOT EXISTS regress;
DROP TABLE IF EXISTS regress.mock_items CASCADE;
CALL paradedb.create_bm25_test_table(
        schema_name => 'regress',
        table_name => 'mock_items'
     );
-- Create index without sku column to match original test setup
CREATE INDEX idxregress_mock_items
    ON regress.mock_items
        USING bm25 (id, description, rating, category, in_stock, metadata, created_at, last_updated_date, latest_available_time, weight_range)
    WITH (key_field='id');

-- Enable recursive estimates feature
SET paradedb.explain_recursive_estimates = ON;
