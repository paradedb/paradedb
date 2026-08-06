SET enable_seqscan=off; SET enable_bitmapscan=off; SET enable_sort=off; SET ivfflat.probes={{ probes_unfiltered }}; SELECT _id, title FROM cohere_wiki
ORDER BY emb <=> current_setting('cohere.qvec')::vector(1024)
LIMIT 10;
