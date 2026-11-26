DROP AGGREGATE IF EXISTS pdb.term_set(bigint);
DROP FUNCTION IF EXISTS pdb.term_set_agg_i_64_term_set_agg_i_64_state(internal, bigint);
DROP FUNCTION IF EXISTS pdb.term_set_agg_i_64_term_set_agg_i_64_combine(internal, internal);
DROP FUNCTION IF EXISTS pdb.term_set_agg_i_64_term_set_agg_i_64_finalize(internal);
