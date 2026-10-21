-- Before #6873, any unicode_words typmod with an option (even remove_emojis=false)
-- was parsed as remove_emojis=true, and that tokenizer name is stored in the schema.
CREATE TABLE unicode_words_tokenizers (
    id INTEGER PRIMARY KEY,
    about_me TEXT,
    bio TEXT
);

INSERT INTO unicode_words_tokenizers VALUES (1, 'hello world 🚀', 'Hello World 🚀');

CREATE INDEX unicode_words_tokenizers_idx ON unicode_words_tokenizers
USING paradedb (
    id,
    (about_me::pdb.unicode_words('remove_emojis=false')),
    (bio::pdb.unicode_words('lowercase=false'))
) WITH (key_field = 'id');

CREATE TABLE unicode_words_schema_before_upgrade AS
SELECT name, tokenizer FROM paradedb.schema('unicode_words_tokenizers_idx');
