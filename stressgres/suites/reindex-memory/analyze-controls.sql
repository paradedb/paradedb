-- Diagnostic controls for the disposable fixture, not production tuning.
DO $$ BEGIN
  IF current_database() <> 'merge_repro' THEN
    RAISE EXCEPTION 'This fixture requires the disposable merge_repro database';
  END IF;
END $$;

SET default_statistics_target = 100;
ANALYZE VERBOSE merge_memory_repro.ledger_transactions;

SET default_statistics_target = 10;
ANALYZE VERBOSE merge_memory_repro.ledger_transactions;

SET default_statistics_target = 100;
ANALYZE VERBOSE merge_memory_repro.ledger_transactions
  (id, description, metadata_json, organization_id, amount, updated_at);

-- Restore the full-sample statistics after the diagnostic controls.
ANALYZE VERBOSE merge_memory_repro.ledger_transactions;
RESET default_statistics_target;
