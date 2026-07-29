SET vchordrq.probes={{ vchord_probes_unfiltered }}; SET vchordrq.epsilon=1.9; SELECT _id, title FROM cohere_wiki
ORDER BY emb <=> current_setting('cohere.qvec')::vector(1024)
LIMIT 10;
