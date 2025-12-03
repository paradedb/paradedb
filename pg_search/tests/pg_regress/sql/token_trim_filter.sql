\i common/common_setup.sql

-- Jieba without trim keeps whitespace tokens
SELECT 'this is a test.'::pdb.jieba::text[];

-- Jieba with trim removes whitespace tokens
SELECT 'this is a test.'::pdb.jieba('trim=true')::text[];

-- Whitespace-only input gets dropped entirely when trimming
SELECT '   '::pdb.jieba('trim=true')::text[];

-- Trim recognizes Unicode whitespace (non-breaking space)
SELECT (U&'\00A0foo\00A0')::pdb.jieba('trim=true')::text[];

-- Trim recognizes ideographic spaces used in East Asian text
SELECT (U&'\3000漢字\3000')::pdb.jieba('trim=true')::text[];
