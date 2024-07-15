// Copyright (c) 2023-2024 Retake, Inc.
//
// This file is part of ParadeDB - Postgres for Search and Analytics
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

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

#[rstest]
fn test_icu_czech_tokenizer(mut conn: PgConnection) {
    IcuCzechPostsTable::setup().execute(&mut conn);
    r#"
    CALL paradedb.create_bm25(
	    index_name => 'idx_czech',
	    table_name => 'icu_czech_posts',
	    key_field => 'id',
	    text_fields => '{
            author:  {tokenizer: {type: "icu"},},
            title:   {tokenizer: {type: "icu"},},
            message: {tokenizer: {type: "icu"},}
        }'
    )"#
    .execute(&mut conn);

    let columns: IcuCzechPostsTableVec =
        r#"SELECT * FROM idx_czech.search('author:"Tomáš"', stable_sort => true)"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1]);

    let columns: IcuCzechPostsTableVec =
        r#"SELECT * FROM idx_czech.search('title:"zdravý"', stable_sort => true)"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![2]);

    let columns: IcuCzechPostsTableVec =
        r#"SELECT * FROM idx_czech.search('message:"velký"~100', stable_sort => true)"#
            .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![3]);
}

#[rstest]
fn test_icu_czech_content_tokenizer(mut conn: PgConnection) {
    IcuCzechPostsTable::setup().execute(&mut conn);
    r#"
    CALL paradedb.create_bm25(
	    index_name => 'idx_czech_content',
	    table_name => 'icu_czech_posts',
	    key_field => 'id',
	    text_fields => '{message: {tokenizer: {type: "icu"}}}'
    )"#
    .execute(&mut conn);

    let columns: IcuCzechPostsTableVec =
        r#"SELECT *, paradedb.rank_bm25("id") FROM idx_czech_content.search(query => paradedb.phrase(field => 'message', phrases => ARRAY['šla', 'sbírat']));"#
            .fetch_collect(&mut conn);

    assert_eq!(columns.id, vec![1]);
}
