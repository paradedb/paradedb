-- Deliberately `USING bm25`, not `USING paradedb`. This SQL is executed against a
-- cluster restored from a pgBackRest heap snapshot (the `restore-heap` steps in
-- .github/workflows/benchmark-pg_search-queries.yml), which replaces the entire
-- data directory. The restored catalog therefore carries whatever pg_search
-- version the snapshot was captured with, not the version built from the branch,
-- and the `paradedb` access method only exists from 0.25.0 onward. `bm25` is the
-- permanent backwards-compatible alias and resolves on every version, so it is
-- the only name that is safe here until the snapshots are regenerated.

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
    (owner_display_name::pdb.unicode_words('columnar=true')),
    owner_user_id
) WITH (
    key_field = 'id',
    -- Join keys: comments.post_id = id, users.id = owner_user_id.
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
    key_field = 'id'
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
    key_field = 'id',
    partition_by = 'post_id'
);

CREATE INDEX users_idx ON users
USING bm25 (
    id,
    (about_me::pdb.unicode_words('columnar=true')),
    (display_name::pdb.unicode_words('columnar=true')),
    reputation
) WITH (
    key_field = 'id',
    partition_by = 'id'
);
