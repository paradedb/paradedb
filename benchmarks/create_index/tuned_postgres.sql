CREATE INDEX message_gin ON benchmark_logs USING gin (to_tsvector('english', message));
CREATE INDEX country_gin ON benchmark_logs USING gin (to_tsvector('english', country));
CREATE INDEX severity_btree ON benchmark_logs USING btree (severity);
CREATE INDEX timestamp_btree ON benchmark_logs USING btree (timestamp);
CREATE INDEX metadata_label_gin ON benchmark_logs USING gin (to_tsvector('english', metadata -> 'label'));
