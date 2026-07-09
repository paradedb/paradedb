// Copyright (c) 2023-2026 ParadeDB, Inc.
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

use pretty_assertions::assert_eq;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;
use tests::fixtures::*;

fn gather_workers_launched(plan: &Value) -> Option<i64> {
    match plan {
        Value::Object(object) => {
            if matches!(
                object.get("Node Type").and_then(Value::as_str),
                Some("Gather" | "Gather Merge")
            ) {
                if let Some(workers) = object.get("Workers Launched").and_then(Value::as_i64) {
                    return Some(workers);
                }
            }

            object.values().find_map(gather_workers_launched)
        }
        Value::Array(values) => values.iter().find_map(gather_workers_launched),
        _ => None,
    }
}

fn has_worker_instrumented_paradedb_scan(plan: &Value) -> bool {
    match plan {
        Value::Object(object) => {
            let is_paradedb_scan = object
                .get("Node Type")
                .and_then(Value::as_str)
                .is_some_and(|node_type| node_type == "Custom Scan")
                && object
                    .get("Custom Plan Provider")
                    .and_then(Value::as_str)
                    .is_some_and(|provider| provider == "ParadeDB Base Scan");

            let worker_instrumented = object
                .get("Workers")
                .and_then(Value::as_array)
                .is_some_and(|workers| !workers.is_empty());

            (is_paradedb_scan && worker_instrumented)
                || object.values().any(has_worker_instrumented_paradedb_scan)
        }
        Value::Array(values) => values.iter().any(has_worker_instrumented_paradedb_scan),
        _ => false,
    }
}

fn create_lindera_parallel_table(
    conn: &mut PgConnection,
    table: &str,
    index: &str,
    tokenizer: &str,
    text_a: &str,
    text_b: &str,
    text_c: &str,
) {
    format!(
        r#"
        DROP TABLE IF EXISTS {table};
        CREATE TABLE {table} (
            id SERIAL PRIMARY KEY,
            body TEXT
        );

        INSERT INTO {table} (body)
        SELECT CASE
            WHEN i % 3 = 0 THEN '{text_a}'
            WHEN i % 3 = 1 THEN '{text_b}'
            ELSE '{text_c}'
        END
        FROM generate_series(1, 1000) i;

        ALTER TABLE {table} SET (parallel_workers = 2);

        CREATE INDEX {index} ON {table}
        USING bm25 (id, body)
        WITH (
            key_field = 'id',
            text_fields = '{{"body": {{"tokenizer": {{"type": "{tokenizer}"}}, "record": "position"}}}}'
        );
        "#
    )
    .execute(conn);
}

fn assert_lindera_match_launches_workers(conn: &mut PgConnection, table: &str, query: &str) {
    let (count,) = format!(
        r#"
        SELECT count(*)
        FROM {table}
        WHERE body @@@ paradedb.match('body', '{query}')
        "#
    )
    .fetch_one::<(i64,)>(conn);
    assert!(count > 0, "{table} should match query {query:?}");

    let (plan,) = format!(
        r#"
        EXPLAIN (ANALYZE, VERBOSE, FORMAT JSON)
        SELECT id, paradedb.score(id)
        FROM {table}
        WHERE body @@@ paradedb.match('body', '{query}')
        ORDER BY paradedb.score(id) DESC
        LIMIT 10
        "#
    )
    .fetch_one::<(Value,)>(conn);

    eprintln!("{table} parallel Lindera plan: {plan:#?}");

    let workers = gather_workers_launched(&plan)
        .unwrap_or_else(|| panic!("{table} should use Gather/Gather Merge: {plan:#?}"));
    assert!(
        workers > 0,
        "{table} should launch parallel workers: {plan:#?}"
    );
    assert!(
        has_worker_instrumented_paradedb_scan(&plan),
        "{table} should execute ParadeDB Base Scan in a worker: {plan:#?}"
    );
}

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

        CREATE INDEX korean_idx ON korean
        USING bm25 (id, author, title, message)
        WITH (
            key_field = 'id',
            text_fields = '{
                "author": {
                    "tokenizer": {"type": "korean_lindera"},
                    "record": "position"
                },
                "title": {
                    "tokenizer": {"type": "korean_lindera"},
                    "record": "position"
                },
                "message": {
                    "tokenizer": {"type": "korean_lindera"},
                    "record": "position"
                }
            }'
        );
    "#
    .execute(&mut conn);

    let row: (i32,) = r#"SELECT id FROM korean WHERE korean @@@ 'author:김민준' ORDER BY id"#
        .fetch_one(&mut conn);
    assert_eq!(row.0, 1);

    let row: (i32,) =
        r#"SELECT id FROM korean WHERE korean @@@ 'title:"경기"' ORDER BY id"#.fetch_one(&mut conn);
    assert_eq!(row.0, 2);

    let row: (i32,) = r#"SELECT id FROM korean WHERE korean @@@ 'message:"지역 축제"' ORDER BY id"#
        .fetch_one(&mut conn);
    assert_eq!(row.0, 3);
}

#[rstest]
fn lindera_match_launches_parallel_workers(mut conn: PgConnection) {
    if pg_major_version(&mut conn) < 16 {
        // `debug_parallel_query` is needed to make worker launch deterministic.
        return;
    }

    r#"
        SET max_parallel_workers = 8;
        SET max_parallel_workers_per_gather = 2;
        SET min_parallel_table_scan_size = 0;
        SET min_parallel_index_scan_size = 0;
        SET parallel_setup_cost = 0;
        SET parallel_tuple_cost = 0;
        SET debug_parallel_query = on;
    "#
    .execute(&mut conn);

    create_lindera_parallel_table(
        &mut conn,
        "lindera_parallel_ko",
        "lindera_parallel_ko_idx",
        "korean_lindera",
        "서울은 한국의 수도이며 검색 테스트 문장입니다.",
        "부산은 항구와 해변으로 유명한 도시입니다.",
        "대구는 음식과 시장으로 잘 알려져 있습니다.",
    );
    create_lindera_parallel_table(
        &mut conn,
        "lindera_parallel_ja",
        "lindera_parallel_ja_idx",
        "japanese_lindera",
        "東京は日本の首都で検索テストの文章です。",
        "大阪は食文化と商業で知られる都市です。",
        "京都には古い寺院と静かな庭があります。",
    );
    create_lindera_parallel_table(
        &mut conn,
        "lindera_parallel_zh",
        "lindera_parallel_zh_idx",
        "chinese_lindera",
        "北京是中国的首都，也是搜索测试句子。",
        "上海拥有繁忙的港口和现代天际线。",
        "广州以美食和贸易闻名。",
    );

    assert_lindera_match_launches_workers(&mut conn, "lindera_parallel_ko", "서울");
    assert_lindera_match_launches_workers(&mut conn, "lindera_parallel_ja", "東京");
    assert_lindera_match_launches_workers(&mut conn, "lindera_parallel_zh", "北京");
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

    CREATE INDEX chinese_idx ON chinese
    USING bm25 (id, author, title, message)
    WITH (
        key_field = 'id',
        text_fields = '{
            "author": {
                "tokenizer": {"type": "chinese_lindera"},
                "record": "position"
            },
            "title": {
                "tokenizer": {"type": "chinese_lindera"},
                "record": "position"
            },
            "message": {
                "tokenizer": {"type": "chinese_lindera"},
                "record": "position"
            }
        }'
    ); 
    "#
    .execute(&mut conn);

    let row: (i32,) =
        r#"SELECT id FROM chinese WHERE chinese @@@ 'author:华' ORDER BY id"#.fetch_one(&mut conn);
    assert_eq!(row.0, 1);

    let row: (i32,) =
        r#"SELECT id FROM chinese WHERE chinese @@@ 'title:北京' ORDER BY id"#.fetch_one(&mut conn);
    assert_eq!(row.0, 1);

    let row: (i32,) = r#"SELECT id FROM chinese WHERE chinese @@@ 'message:文化节' ORDER BY id"#
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

    CREATE INDEX japanese_idx ON japanese
    USING bm25 (id, author, title, message)
    WITH (
        key_field = 'id',
        text_fields = '{
            "author": {
                "tokenizer": {"type": "japanese_lindera"},
                "record": "position"
            },
            "title": {
                "tokenizer": {"type": "japanese_lindera"},
                "record": "position"
            },
            "message": {
                "tokenizer": {"type": "japanese_lindera"},
                "record": "position"
            }
        }'
    );
    "#
    .execute(&mut conn);

    let row: (i32,) = r#"SELECT id FROM japanese WHERE japanese @@@ 'author:佐藤' ORDER BY id"#
        .fetch_one(&mut conn);
    assert_eq!(row.0, 1);

    let row: (i32,) = r#"SELECT id FROM japanese WHERE japanese @@@ 'title:サッカー' ORDER BY id"#
        .fetch_one(&mut conn);
    assert_eq!(row.0, 2);

    let row: (i32,) = r#"SELECT id FROM japanese WHERE japanese @@@ 'message:祭り' ORDER BY id"#
        .fetch_one(&mut conn);
    assert_eq!(row.0, 3);
}
