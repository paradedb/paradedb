mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn both_parquet_views(mut conn: PgConnection) {
    view_test(&mut conn, false, true, true);
}

#[rstest]
fn left_parquet_view(mut conn: PgConnection) {
    view_test(&mut conn, false, true, false);
}

#[rstest]
fn right_parquet_view(mut conn: PgConnection) {
    view_test(&mut conn, false, false, true);
}

#[rstest]
fn both_heap_views(mut conn: PgConnection) {
    view_test(&mut conn, false, false, false);
}

#[rstest]
fn both_parquet_materialized_views(mut conn: PgConnection) {
    view_test(&mut conn, true, true, true);
}

#[rstest]
fn left_parquet_materialized_view(mut conn: PgConnection) {
    view_test(&mut conn, true, true, false);
}

#[rstest]
fn right_parquet_materialized_view(mut conn: PgConnection) {
    view_test(&mut conn, true, false, true);
}

#[rstest]
fn both_heap_materialized_views(mut conn: PgConnection) {
    view_test(&mut conn, true, false, false);
}

#[inline]
fn view_test(conn: &mut PgConnection, materialized: bool, left_parquet: bool, right_parquet: bool) {
    let left_am = if left_parquet { "parquet" } else { "heap" };
    let right_am = if right_parquet { "parquet" } else { "heap" };
    let view = if materialized {
        "MATERIALIZED VIEW"
    } else {
        "VIEW"
    };

    let create_tables_sql = format!(
        r#"
            CREATE TABLE users (
                user_id SERIAL PRIMARY KEY,
                username VARCHAR(255) NOT NULL,
                email VARCHAR(255) NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                UNIQUE (username, email)
            ) USING {};
            
            CREATE TABLE orders (
                order_id SERIAL PRIMARY KEY,
                username VARCHAR(255) NOT NULL,
                email VARCHAR(255) NOT NULL,
                order_total DECIMAL(10, 2) NOT NULL,
                order_date TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (username, email) REFERENCES users(username, email)
            ) USING {};

            CREATE {} user_orders AS 
            SELECT 
                u.user_id, u.username, u.email, u.created_at,
                o.order_id, o.order_total, o.order_date
            FROM users u
            JOIN orders o ON u.username = o.username AND u.email = o.email;
        "#,
        left_am, right_am, view
    );

    create_tables_sql.execute(conn);

    r#"                        
        INSERT INTO users (username, email) 
        VALUES 
            ('User1', 'user1@gmail.com'), 
            ('User2', 'user2@gmail.com'), 
            ('User3', 'user3@gmail.com'), 
            ('User4', 'user4@gmail.com');
    "#
    .execute(conn);

    r#"
        INSERT INTO orders (username, email, order_total) 
        VALUES 
            ('User1', 'user1@gmail.com', 100.00), 
            ('User1', 'user1@gmail.com', 200.00),
            ('User2', 'user2@gmail.com', 300.00);
    "#
    .execute(conn);

    if materialized {
        r#"REFRESH MATERIALIZED VIEW user_orders"#.execute(conn);
    }

    let count: (i64,) = "SELECT COUNT(*) FROM user_orders".fetch_one(conn);
    assert_eq!(count, (3,));

    r#"
        INSERT INTO orders (username, email, order_total) 
        VALUES ('User4', 'user4@gmail.com', 100.00);
    "#
    .execute(conn);

    if materialized {
        r#"REFRESH MATERIALIZED VIEW user_orders"#.execute(conn);
    }

    let count: (i64,) = "SELECT COUNT(*) FROM user_orders".fetch_one(conn);
    assert_eq!(count, (4,));
}
