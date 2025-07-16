\i common/common_setup.sql

DROP TABLE IF EXISTS layer_sizes;
CREATE TABLE layer_sizes (id serial8 not null primary key);

-- 1 layer ✅
CREATE INDEX idxlayer_sizes ON layer_sizes USING bm25(id) WITH (key_field='id', layer_sizes = '1kb');
SELECT * FROM paradedb.layer_sizes('idxlayer_sizes');
SELECT * FROM paradedb.background_layer_sizes('idxlayer_sizes');
DROP INDEX idxlayer_sizes;

CREATE INDEX idxlayer_sizes ON layer_sizes USING bm25(id) WITH (key_field='id', background_layer_sizes = '1kb');
SELECT * FROM paradedb.layer_sizes('idxlayer_sizes');
SELECT * FROM paradedb.background_layer_sizes('idxlayer_sizes');
DROP INDEX idxlayer_sizes;

-- negative layer ❌
CREATE INDEX idxlayer_sizes ON layer_sizes USING bm25(id) WITH (key_field='id', layer_sizes = '-1kb');
CREATE INDEX idxlayer_sizes ON layer_sizes USING bm25(id) WITH (key_field='id', background_layer_sizes = '-1kb');

-- zero layer ✅
CREATE INDEX idxlayer_sizes ON layer_sizes USING bm25(id) WITH (key_field='id', layer_sizes = '0kb, 10kb');
SELECT * FROM paradedb.layer_sizes('idxlayer_sizes');
SELECT * FROM paradedb.background_layer_sizes('idxlayer_sizes');
DROP INDEX idxlayer_sizes;

CREATE INDEX idxlayer_sizes ON layer_sizes USING bm25(id) WITH (key_field='id', background_layer_sizes = '0kb, 10kb');
SELECT * FROM paradedb.layer_sizes('idxlayer_sizes');
SELECT * FROM paradedb.background_layer_sizes('idxlayer_sizes');
DROP INDEX idxlayer_sizes;

-- malformed layer ❌
CREATE INDEX idxlayer_sizes ON layer_sizes USING bm25(id) WITH (key_field='id', layer_sizes = '1kb, bob''s your uncle');
CREATE INDEX idxlayer_sizes ON layer_sizes USING bm25(id) WITH (key_field='id', background_layer_sizes = '1kb, bob''s your uncle');

-- multiple layers ✅
CREATE INDEX idxlayer_sizes ON layer_sizes USING bm25(id) WITH (key_field='id', layer_sizes = '1kb, 10kb, 100MB');
SELECT * FROM paradedb.layer_sizes('idxlayer_sizes');
SELECT * FROM paradedb.background_layer_sizes('idxlayer_sizes');
DROP INDEX idxlayer_sizes;

CREATE INDEX idxlayer_sizes ON layer_sizes USING bm25(id) WITH (key_field='id', background_layer_sizes = '1kb, 10kb, 100MB');
SELECT * FROM paradedb.layer_sizes('idxlayer_sizes');
SELECT * FROM paradedb.background_layer_sizes('idxlayer_sizes');
DROP INDEX idxlayer_sizes;

DROP TABLE layer_sizes;
