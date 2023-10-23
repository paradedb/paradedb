-- Default tokenizer
CREATE INDEX idxtokenizerconfig ON tokenizer_config USING bm25 ((tokenizer_config.*)) WITH (text_fields='{"description": {}}');
SELECT * FROM tokenizer_config WHERE tokenizer_config @@@ 'description:earbud';
DROP INDEX idxtokenizerconfig;

-- en_stem
CREATE INDEX idxtokenizerconfig ON tokenizer_config USING bm25 ((tokenizer_config.*)) WITH (text_fields='{"description": {"tokenizer": { "type": "en_stem" }}}');
SELECT * FROM tokenizer_config WHERE tokenizer_config @@@ 'description:earbud';
DROP INDEX idxtokenizerconfig;

-- ngram
CREATE INDEX idxtokenizerconfig ON tokenizer_config USING bm25 ((tokenizer_config.*)) WITH (text_fields='{"description": {"tokenizer": {"type": "ngram", "min_gram": 3, "max_gram": 8, "prefix_only": false}}}');
SELECT * FROM tokenizer_config WHERE tokenizer_config @@@ 'description:boa';
DROP INDEX idxtokenizerconfig;

-- chinese_compatible
CREATE INDEX idxtokenizerconfig  ON tokenizer_config  USING bm25 ((tokenizer_config.*))  WITH (text_fields='{"description": {"tokenizer": {"type": "chinese_compatible"}, "record": "position"}}');
INSERT INTO tokenizer_config (description, rating, category) VALUES ('电脑', 4, 'Electronics');
SELECT * FROM tokenizer_config WHERE tokenizer_config @@@ '电脑';
DROP INDEX idxtokenizerconfig;

