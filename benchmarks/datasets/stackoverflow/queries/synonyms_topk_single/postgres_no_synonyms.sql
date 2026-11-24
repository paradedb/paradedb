SET work_mem TO '4GB'; SELECT id FROM stackoverflow_posts_fts WHERE body_tsv @@ 'exception'::tsquery ORDER BY ts_rank(body_tsv, 'exception'::tsquery) DESC LIMIT 10;
