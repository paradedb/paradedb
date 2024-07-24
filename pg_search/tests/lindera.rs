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

#![allow(unused_variables, unused_imports)]
mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
async fn lindera_korean_tokenizer(mut conn: PgConnection) {
    r#"CREATE TABLE IF NOT EXISTS korean (
        id SERIAL PRIMARY KEY,
        author TEXT,
        title TEXT,
        message TEXT
    );

    INSERT INTO korean (author, title, message)
    VALUES
        ('김민준', '서울의 새로운 카페', '서울 중심부에 새로운 카페가 문을 열었습니다. 현대적인 디자인과 독특한 커피 선택이 특징입니다.'),
        ('이하은', '축구 경기 리뷰', '어제 열린 축구 경기에서 화려한 골이 터졌습니다. 마지막 순간의 반전이 경기의 하이라이트였습니다.'),
        ('박지후', '지역 축제 개최 소식', '이번 주말 지역 축제가 열립니다. 다양한 음식과 공연이 준비되어 있어 기대가 됩니다.');

    CALL paradedb.create_bm25(
    	index_name => 'korean',
    	table_name => 'korean',
    	key_field => 'id',
        text_fields => paradedb.field('author', tokenizer => paradedb.tokenizer('korean_lindera'), record => 'position') ||
                       paradedb.field('title', tokenizer => paradedb.tokenizer('korean_lindera'), record => 'position') ||
                       paradedb.field('message', tokenizer => paradedb.tokenizer('korean_lindera'), record => 'position')
    )"#
    .execute(&mut conn);

    let row: (i32,) = r#"SELECT id FROM korean.search('author:김민준', stable_sort => true)"#
        .fetch_one(&mut conn);
    assert_eq!(row.0, 1);

    let row: (i32,) =
        r#"SELECT id FROM korean.search('title:"경기"', stable_sort => true)"#.fetch_one(&mut conn);
    assert_eq!(row.0, 2);

    let row: (i32,) = r#"SELECT id FROM korean.search('message:"지역 축제"', stable_sort => true)"#
        .fetch_one(&mut conn);
    assert_eq!(row.0, 3);
}

#[rstest]
async fn lindera_chinese_tokenizer(mut conn: PgConnection) {
    r#"CREATE TABLE IF NOT EXISTS chinese (
        id SERIAL PRIMARY KEY,
        author TEXT,
        title TEXT,
        message TEXT
    );
    INSERT INTO chinese (author, title, message)
    VALUES
        ('李华', '北京的新餐馆', '北京市中心新开了一家餐馆，以其现代设计和独特的菜肴选择而闻名。'),
        ('张伟', '篮球比赛回顾', '昨日篮球比赛精彩纷呈，尤其是最后时刻的逆转成为了比赛的亮点。'),
        ('王芳', '本地文化节', '本周末将举行一个地方文化节，预计将有各种食物和表演。');
    CALL paradedb.create_bm25(
    	index_name => 'chinese',
    	table_name => 'chinese',
        key_field => 'id',
        text_fields => paradedb.field('author', tokenizer => paradedb.tokenizer('chinese_lindera'), record => 'position') ||
                       paradedb.field('title', tokenizer => paradedb.tokenizer('chinese_lindera'), record => 'position') ||
                       paradedb.field('message', tokenizer => paradedb.tokenizer('chinese_lindera'), record => 'position')
    )"#
    .execute(&mut conn);

    let row: (i32,) =
        r#"SELECT id FROM chinese.search('author:华', stable_sort => true)"#.fetch_one(&mut conn);
    assert_eq!(row.0, 1);

    let row: (i32,) =
        r#"SELECT id FROM chinese.search('title:北京', stable_sort => true)"#.fetch_one(&mut conn);
    assert_eq!(row.0, 1);

    let row: (i32,) = r#"SELECT id FROM chinese.search('message:文化节', stable_sort => true)"#
        .fetch_one(&mut conn);
    assert_eq!(row.0, 3);
}

#[rstest]
async fn lindera_japenese_tokenizer(mut conn: PgConnection) {
    r#"
    CREATE TABLE IF NOT EXISTS japanese (
        id SERIAL PRIMARY KEY,
        author TEXT,
        title TEXT,
        message TEXT
    );
    INSERT INTO japanese (author, title, message)
    VALUES
        ('佐藤健', '東京の新しいカフェ', '東京の中心部に新しいカフェがオープンしました。モダンなデザインとユニークなコーヒーが特徴です。'),
        ('鈴木一郎', 'サッカー試合レビュー', '昨日のサッカー試合では素晴らしいゴールが見られました。終了間際のドラマチックな展開がハイライトでした。'),
        ('高橋花子', '地元の祭り', '今週末に地元で祭りが開催されます。様々な食べ物とパフォーマンスが用意されています。');
    CALL paradedb.create_bm25(
    	index_name => 'japanese',
    	table_name => 'japanese',
        key_field => 'id',
        text_fields => paradedb.field('author', tokenizer => paradedb.tokenizer('japanese_lindera'), record => 'position') ||
                       paradedb.field('title', tokenizer => paradedb.tokenizer('japanese_lindera'), record => 'position') ||
                       paradedb.field('message', tokenizer => paradedb.tokenizer('japanese_lindera'), record => 'position')
    )"#
    .execute(&mut conn);

    let row: (i32,) = r#"SELECT id FROM japanese.search('author:佐藤', stable_sort => true)"#
        .fetch_one(&mut conn);
    assert_eq!(row.0, 1);

    let row: (i32,) = r#"SELECT id FROM japanese.search('title:サッカー', stable_sort => true)"#
        .fetch_one(&mut conn);
    assert_eq!(row.0, 2);

    let row: (i32,) = r#"SELECT id FROM japanese.search('message:祭り', stable_sort => true)"#
        .fetch_one(&mut conn);
    assert_eq!(row.0, 3);
}
