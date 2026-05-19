//! Tests verifying BM25 k1 and b parameter configuration via typmod syntax.

mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[fixture]
fn setup(mut conn: PgConnection) -> PgConnection {
    sqlx::query("CREATE TABLE IF NOT EXISTS bm25_param_test (id SERIAL PRIMARY KEY, title TEXT, body TEXT)")
        .execute(&mut conn)
        .unwrap();
    sqlx::query("INSERT INTO bm25_param_test (title, body) VALUES ('hello world', 'hello hello hello world world world this is a longer body text'), ('world', 'short')")
        .execute(&mut conn)
        .unwrap();
    conn
}

#[rstest]
fn test_bm25_default_parameters(mut setup: PgConnection) {
    let result = sqlx::query_scalar::<_, i64>(
        "CREATE INDEX idx_bm25_default ON bm25_param_test USING bm25 (id, title text_ops); SELECT 1::bigint",
    )
    .fetch_one(&mut setup);
    assert!(result.is_ok());
    sqlx::query("DROP INDEX IF EXISTS idx_bm25_default")
        .execute(&mut setup)
        .unwrap();
}

#[rstest]
fn test_bm25_custom_k1_and_b(mut setup: PgConnection) {
    let result = sqlx::query_scalar::<_, i64>(
        "CREATE INDEX idx_bm25_custom ON bm25_param_test USING bm25 (id, title text_ops (k1 = 1.5, b = 0.5)); SELECT 1::bigint",
    )
    .fetch_one(&mut setup);
    assert!(result.is_ok());
    sqlx::query("DROP INDEX IF EXISTS idx_bm25_custom")
        .execute(&mut setup)
        .unwrap();
}

#[rstest]
fn test_bm25_k1_only(mut setup: PgConnection) {
    let result = sqlx::query_scalar::<_, i64>(
        "CREATE INDEX idx_bm25_k1 ON bm25_param_test USING bm25 (id, body text_ops (k1 = 0.5)); SELECT 1::bigint",
    )
    .fetch_one(&mut setup);
    assert!(result.is_ok());
    sqlx::query("DROP INDEX IF EXISTS idx_bm25_k1")
        .execute(&mut setup)
        .unwrap();
}

#[rstest]
fn test_bm25_b_only(mut setup: PgConnection) {
    let result = sqlx::query_scalar::<_, i64>(
        "CREATE INDEX idx_bm25_b ON bm25_param_test USING bm25 (id, body text_ops (b = 0.0)); SELECT 1::bigint",
    )
    .fetch_one(&mut setup);
    assert!(result.is_ok());
    sqlx::query("DROP INDEX IF EXISTS idx_bm25_b")
        .execute(&mut setup)
        .unwrap();
}

#[rstest]
fn test_bm25_zero_parameters(mut setup: PgConnection) {
    let result = sqlx::query_scalar::<_, i64>(
        "CREATE INDEX idx_bm25_zero ON bm25_param_test USING bm25 (id, title text_ops (k1 = 0.0, b = 0.0)); SELECT 1::bigint",
    )
    .fetch_one(&mut setup);
    assert!(result.is_ok());
    sqlx::query("DROP INDEX IF EXISTS idx_bm25_zero")
        .execute(&mut setup)
        .unwrap();
}

#[rstest]
fn test_bm25_multi_field_different_params(mut setup: PgConnection) {
    let result = sqlx::query_scalar::<_, i64>(
        "CREATE INDEX idx_bm25_multi ON bm25_param_test USING bm25 (id, title text_ops (k1 = 0.5, b = 0.25), body text_ops (k1 = 1.8, b = 0.75)); SELECT 1::bigint",
    )
    .fetch_one(&mut setup);
    assert!(result.is_ok());
    sqlx::query("DROP INDEX IF EXISTS idx_bm25_multi")
        .execute(&mut setup)
        .unwrap();
}
