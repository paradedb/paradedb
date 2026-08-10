CREATE INDEX stackoverflow_posts_idx ON stackoverflow_posts
USING paradedb (
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
    key_field = 'id'
);

CREATE INDEX badges_idx ON badges
USING paradedb (
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
USING paradedb (
    id,
    post_id,
    score,
    (text::pdb.unicode_words('columnar=true')),
    creation_date,
    (user_display_name::pdb.literal)
) WITH (
    key_field = 'id'
);

CREATE INDEX users_idx ON users
USING paradedb (
    id,
    (about_me::pdb.unicode_words('columnar=true')),
    (display_name::pdb.unicode_words('columnar=true')),
    reputation
) WITH (
    key_field = 'id'
);
