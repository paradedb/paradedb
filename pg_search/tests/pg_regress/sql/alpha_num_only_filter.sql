\i common/common_setup.sql

SELECT 'Hello World 123!'::pdb.simple::text[];
SELECT 'Hello World 123!'::pdb.simple('alpha_num_only=false')::text[];
SELECT 'Hello World 123!'::pdb.simple('alpha_num_only=true')::text[];

SELECT 'Hello World 123!'::pdb.ngram(3, 3)::text[];
SELECT 'Hello World 123!'::pdb.ngram(3, 3, 'alpha_num_only=false')::text[];
SELECT 'Hello World 123!'::pdb.ngram(3, 3, 'alpha_num_only=true')::text[];

SELECT 'Český člověk žlutý kůň příliš'::pdb.simple('alpha_num_only=true')::text[];
SELECT 'Český člověk žlutý kůň příliš'::pdb.simple('alpha_num_only=true', 'ascii_folding=true')::text[];
