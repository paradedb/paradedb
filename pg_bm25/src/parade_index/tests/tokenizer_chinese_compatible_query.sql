SELECT paradedb.highlight_bm25(posts.id, 'idx_posts_fts', 'author') from posts where posts @@@ 'author:张';
