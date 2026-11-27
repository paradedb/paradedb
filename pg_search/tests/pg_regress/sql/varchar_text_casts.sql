\i common/common_setup.sql

-- Test text/varchar casts for tokenizer types
SELECT pdb.tokenize_unicode_words('good job'::varchar);
SELECT pdb.tokenize_unicode_words('good job'::text);
