DO $$ BEGIN
  IF current_database() <> 'merge_repro' THEN
    RAISE EXCEPTION 'This fixture requires the disposable merge_repro database';
  END IF;
END $$;
CREATE EXTENSION IF NOT EXISTS pg_search;
DROP SCHEMA IF EXISTS merge_memory_repro CASCADE;
CREATE SCHEMA merge_memory_repro;
SET search_path=merge_memory_repro,public;
SET maintenance_work_mem='32MB';
SET max_parallel_maintenance_workers=0;
CREATE TABLE ledger_transactions (
  id bigint PRIMARY KEY,
  description text,
  metadata_json jsonb,
  organization_id bigint,
  amount bigint,
  updated_at timestamptz
);
-- Interleaved id ranges in physical scan order. Explicit id sort intentionally
-- forces overlap; it does NOT claim the customer's default ctid sort did so.
INSERT INTO ledger_transactions
SELECT row_number*4+batch,
       repeat('common anchor ',512),
       jsonb_build_object('description',repeat('common anchor ',512),'status','posted'),
       row_number % 100, row_number, '2026-01-01'::timestamptz
FROM generate_series(0,3) batch
CROSS JOIN generate_series(1,10000) row_number
ORDER BY batch,row_number;
CREATE INDEX ledger_search ON ledger_transactions USING bm25
(id, (description::pdb.ngram('3','5')),
 (metadata_json::pdb.literal_normalized),
 (metadata_json::pdb.unicode_words('alias=metadata_json_words')),
 organization_id,amount,updated_at)
WITH (key_field='id',target_segment_count=1,sort_by='id ASC NULLS FIRST');
ANALYZE ledger_transactions;
CREATE FUNCTION verify() RETURNS bigint LANGUAGE plpgsql AS $$
DECLARE found bigint;
BEGIN
  SELECT count(*) INTO found FROM merge_memory_repro.ledger_transactions WHERE description ||| 'common';
  IF found <> 40000 THEN RAISE EXCEPTION 'Expected 40000 matches, found %',found; END IF;
  RETURN found;
END $$;
SELECT verify();
