-- Default tokenizer
CREATE INDEX idxtokenizerconfig ON tokenizer_config USING bm25 ((tokenizer_config.*)) WITH (text_fields='{"description": {}}');
SELECT * FROM tokenizer_config WHERE tokenizer_config @@@ 'description:earbud';
DROP INDEX idxtokenizerconfig;

-- en_stem
CREATE INDEX idxtokenizerconfig ON tokenizer_config USING bm25 ((tokenizer_config.*)) WITH (text_fields='{"description": {"tokenizer": "en_stem"}}');
SELECT * FROM tokenizer_config WHERE tokenizer_config @@@ 'description:earbud';
DROP INDEX idxtokenizerconfig;