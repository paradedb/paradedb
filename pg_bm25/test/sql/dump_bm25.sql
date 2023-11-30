SELECT * FROM paradedb.dump_bm25('idxdumpbm25') ORDER BY heap_tid;
SELECT * FROM paradedb.dump_bm25('idxmockitems') WHERE content->>'category' = 'Electronics' ORDER BY heap_tid;
