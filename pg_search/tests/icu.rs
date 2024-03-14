#![cfg(feature = "icu")]

mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn test_icu_arabic_tokenizer(mut conn: PgConnection) {
    IcuArabicPostsTable::setup().execute(&mut conn);
    r#"
    CALL paradedb.create_bm25(
	    index_name => 'idx_arabic',
	    table_name => 'icu_arabic_posts',
	    key_field => 'id',
	    text_fields => '{
            author:  {tokenizer: {type: "icu"},},
            title:   {tokenizer: {type: "icu"},},
            message: {tokenizer: {type: "icu"},}
        }'
    )"#
    .execute(&mut conn);

    let columns: IcuArabicPostsTableVec =
        r#"SELECT * FROM idx_arabic.search('author:"محمد"', stable_sort => true)"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![2]);

    let columns: IcuArabicPostsTableVec =
        r#"SELECT * FROM idx_arabic.search('title:"السوق"', stable_sort => true)"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![2]);

    let columns: IcuArabicPostsTableVec =
        r#"SELECT * FROM idx_arabic.search('message:"في"', stable_sort => true)"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![2, 1, 3]);
}

#[rstest]
fn test_icu_amharic_tokenizer(mut conn: PgConnection) {
    IcuAmharicPostsTable::setup().execute(&mut conn);
    r#"
    CALL paradedb.create_bm25(
	    index_name => 'idx_amharic',
	    table_name => 'icu_amharic_posts',
	    key_field => 'id',
	    text_fields => '{
            author:  {tokenizer: {type: "icu"},},
            title:   {tokenizer: {type: "icu"},},
            message: {tokenizer: {type: "icu"},}
        }'
    )"#
    .execute(&mut conn);

    let columns: IcuAmharicPostsTableVec =
        r#"SELECT * FROM idx_amharic.search('author:"አለም"', stable_sort => true)"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![3]);

    let columns: IcuAmharicPostsTableVec =
        r#"SELECT * FROM idx_amharic.search('title:"ለመማር"', stable_sort => true)"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![3]);

    let columns: IcuAmharicPostsTableVec =
        r#"SELECT * FROM idx_amharic.search('message:"ዝናብ"', stable_sort => true)"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![2, 1]);
}

#[rstest]
fn test_icu_greek_tokenizer(mut conn: PgConnection) {
    IcuGreekPostsTable::setup().execute(&mut conn);
    r#"
    CALL paradedb.create_bm25(
	    index_name => 'idx_greek',
	    table_name => 'icu_greek_posts',
	    key_field => 'id',
	    text_fields => '{
            author:  {tokenizer: {type: "icu"},},
            title:   {tokenizer: {type: "icu"},},
            message: {tokenizer: {type: "icu"},}
        }'
    )"#
    .execute(&mut conn);

    let columns: IcuGreekPostsTableVec =
        r#"SELECT * FROM idx_greek.search('author:"Σοφία"', stable_sort => true)"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![2]);

    let columns: IcuGreekPostsTableVec =
        r#"SELECT * FROM idx_greek.search('title:"επιτυχία"', stable_sort => true)"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![3]);

    let columns: IcuGreekPostsTableVec =
        r#"SELECT * FROM idx_greek.search('message:"συμβουλές"', stable_sort => true)"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![3]);
}
