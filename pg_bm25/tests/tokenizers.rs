mod fixtures;

use fixtures::*;
use sqlx::PgConnection;

#[rstest]
async fn basic_test(#[future] conn: PgConnection) -> sqlx::Result<()> {
    let row: (i32,) = sqlx::query_as("SELECT $1")
        .bind(150)
        .fetch_one(&mut conn.await)
        .await?;

    assert_eq!(row.0, 150);

    Ok(())
}

#[rstest]
async fn basic_test2(#[future] conn: PgConnection) -> sqlx::Result<()> {
    let row: (i32,) = sqlx::query_as("SELECT $1")
        .bind(42)
        .fetch_one(&mut conn.await)
        .await?;

    assert_eq!(row.0, 42);

    Ok(())
}

#[rstest]
async fn basic_test3(#[future] conn: PgConnection) -> sqlx::Result<()> {
    let row: (i32,) = sqlx::query_as("SELECT $1")
        .bind(1)
        .fetch_one(&mut conn.await)
        .await?;

    assert_eq!(row.0, 1);

    Ok(())
}
