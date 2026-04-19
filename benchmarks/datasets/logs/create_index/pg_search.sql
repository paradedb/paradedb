CREATE INDEX benchmark_logs_idx ON benchmark_logs 
USING bm25 (
    id, 
    message, 
    (country::pdb.literal_normalized),
    severity, 
    timestamp, 
    (metadata::pdb.literal_normalized)
) WITH (
    key_field = 'id'
);
