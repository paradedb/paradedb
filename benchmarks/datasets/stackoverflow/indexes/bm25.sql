-- Use the bm25 alias for benchmark runs against older refs.

CREATE INDEX stackoverflow_posts_idx ON stackoverflow_posts
USING bm25 (
    id,
    (title::pdb.unicode_words('columnar=true')),
    (body::pdb.unicode_words('columnar=true')),
    (tags::pdb.literal_normalized),
    post_type_id,
    score,
    creation_date,
    view_count,
    answer_count,
    comment_count,
    amount15,
    amount78,
    (owner_display_name::pdb.unicode_words('columnar=true')),
    owner_user_id
) WITH (
    mutable_segment_rows = 0,
    -- Join keys: comments.post_id = id, users.id = owner_user_id.
    -- TODO: Explore removing multi-key partitioning in the future once range-partitioning
    -- optimizations settle, but retain for now to benchmark 3-table joins.
    partition_by = 'id,owner_user_id'
);

CREATE INDEX badges_idx ON badges
USING bm25 (
    id,
    (name::pdb.unicode_words('columnar=true')),
    date,
    user_id,
    class,
    tag_based
) WITH (
    mutable_segment_rows = 0,
    partition_by = 'user_id'
);

CREATE INDEX comments_idx ON comments
USING bm25 (
    id,
    post_id,
    score,
    (text::pdb.unicode_words('columnar=true')),
    creation_date,
    (user_display_name::pdb.literal)
) WITH (
    mutable_segment_rows = 0,
    partition_by = 'post_id'
);

CREATE INDEX users_idx ON users
USING bm25 (
    id,
    (about_me::pdb.unicode_words('columnar=true')),
    (display_name::pdb.unicode_words('columnar=true')),
    reputation
) WITH (
    mutable_segment_rows = 0,
    partition_by = 'id'
);

-- Companion standard Postgres indexes for Top-K join baseline queries executed in the same pass.
-- See benchmarks/datasets/stackoverflow/README.md for the rationale on including Top-K join
-- baselines while omitting native Postgres aggregate baselines.

-- Foreign Key / Join Columns (Standard B-tree)
CREATE INDEX stackoverflow_posts_owner_user_id_idx ON stackoverflow_posts (owner_user_id);
CREATE INDEX comments_post_id_idx ON comments (post_id);

-- Full-Text Search Columns (GIN on tsvector)
CREATE INDEX stackoverflow_posts_title_fts_idx ON stackoverflow_posts USING gin (to_tsvector('english', title));
CREATE INDEX users_about_me_fts_idx ON users USING gin (to_tsvector('english', about_me));
CREATE INDEX users_display_name_fts_idx ON users USING gin (to_tsvector('english', display_name));
CREATE INDEX comments_text_fts_idx ON comments USING gin (to_tsvector('english', text));

-- Common Scalar Filter, Grouping, and Sort Columns (B-tree)
CREATE INDEX stackoverflow_posts_creation_date_idx ON stackoverflow_posts (creation_date DESC);
CREATE INDEX users_reputation_idx ON users (reputation);
CREATE INDEX users_display_name_idx ON users (display_name);
CREATE INDEX comments_score_idx ON comments (score);
CREATE INDEX comments_creation_date_id_idx ON comments (creation_date DESC, id DESC);

