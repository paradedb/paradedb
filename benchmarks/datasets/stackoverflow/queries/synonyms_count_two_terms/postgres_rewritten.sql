SET work_mem TO '4GB'; SELECT count(*) FROM stackoverflow_posts_fts WHERE body_tsv @@ 'python | exception | failure | bug | crash | fault | problem | issue | fail | warning | mistake'::tsquery;
