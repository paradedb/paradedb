CREATE INDEX stackoverflow_posts_idx ON stackoverflow_posts 
USING bm25 (
    id,
    title,
    body,
    (tags::pdb.literal_normalized),
    post_type_id,
    score,
    creation_date,
    view_count,
    answer_count,
    comment_count,
    (owner_display_name::pdb.unicode_words('columnar=true'))
) WITH (
    key_field = 'id'
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
