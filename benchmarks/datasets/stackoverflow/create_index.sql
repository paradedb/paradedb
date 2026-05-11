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
    sort_by = 'owner_user_id ASC NULLS FIRST'
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
    key_field = 'id',
    sort_by = 'user_id ASC NULLS FIRST'
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
    sort_by = 'post_id ASC NULLS FIRST'
);

CREATE INDEX users_idx ON users
USING bm25 (
    id,
    (about_me::pdb.unicode_words('columnar=true')),
    (display_name::pdb.unicode_words('columnar=true')),
    reputation
) WITH (
    key_field = 'id',
    sort_by = 'id ASC NULLS FIRST'
);
