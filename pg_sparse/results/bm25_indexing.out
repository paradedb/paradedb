CREATE INDEX idxproducts ON products USING bm25 ((products.*));
-- Test indexing another schema
CREATE INDEX idxmockitems ON paradedb.mock_items USING bm25 ((mock_items.*));
