mod fixtures;

use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn single_fkey_on_both(mut conn: PgConnection) {
    single_fkey_test(&mut conn, true, true);
}

#[rstest]
fn single_fkey_on_parquet(mut conn: PgConnection) {
    single_fkey_test(&mut conn, false, true);
}

#[rstest]
fn single_fkey_on_heap(mut conn: PgConnection) {
    single_fkey_test(&mut conn, true, false);
}

#[rstest]
fn composite_fkey_on_both(mut conn: PgConnection) {
    composite_fkey_test(&mut conn, true, true);
}

#[rstest]
fn composite_fkey_on_parquet(mut conn: PgConnection) {
    single_fkey_test(&mut conn, false, true);
}

#[rstest]
fn composite_fkey_on_heap(mut conn: PgConnection) {
    composite_fkey_test(&mut conn, true, false);
}

#[inline]
fn single_fkey_test(conn: &mut PgConnection, primary_parquet: bool, foreign_parquet: bool) {
    let primary_am = if primary_parquet { "parquet" } else { "heap" };
    let foreign_am = if foreign_parquet { "parquet" } else { "heap" };

    let create_sql = format!(
        r#"
        CREATE TABLE users (
            user_id SERIAL PRIMARY KEY,
            username VARCHAR(255) NOT NULL,
            email VARCHAR(255) UNIQUE NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        ) USING {};
        
        CREATE TABLE orders (
            order_id SERIAL PRIMARY KEY,
            user_id INT NOT NULL,
            order_total DECIMAL(10, 2) NOT NULL,
            order_date TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES users(user_id)
        ) USING {};
        "#,
        primary_am, foreign_am
    );

    create_sql.execute(conn);

    r#"                        
        INSERT INTO users (username, email) 
        VALUES ('User1', 'user1@gmail.com'), ('User2', 'user2@gmail.com'), ('User3', 'user3@gmail.com'), ('User4', 'user4@gmail.com');
    "#
    .execute(conn);

    r#"
        INSERT INTO orders (user_id, order_total) VALUES (1, 100.00), (2, 200.00);
    "#
    .execute(conn);

    let count: (i64,) = "SELECT COUNT(*) FROM users".fetch_one(conn);
    assert_eq!(count, (4,));

    let count: (i64,) = "SELECT COUNT(*) FROM orders".fetch_one(conn);
    assert_eq!(count, (2,));

    match "INSERT INTO orders (user_id, order_total) VALUES (6, 600.00)".execute_result(conn) {
        Err(err) => assert!(err.to_string().contains("violates foreign key constraint")),
        _ => panic!("Foreign key constraint violated"),
    };

    match "INSERT INTO orders (user_id, order_total) VALUES (3, 300.00), (6, 600.0)"
        .execute_result(conn)
    {
        Err(err) => assert!(err.to_string().contains("violates foreign key constraint")),
        _ => panic!("Foreign key constraint violated"),
    };

    let rows: Vec<(i32,)> = r#"
        SELECT u.user_id FROM orders o
        INNER JOIN users u ON o.user_id = u.user_id
        ORDER BY u.user_id;
    "#
    .fetch(conn);

    let user_ids = [1, 2];
    assert!(rows.iter().map(|r| r.0).eq(user_ids));
}

#[inline]
fn composite_fkey_test(conn: &mut PgConnection, primary_parquet: bool, foreign_parquet: bool) {
    let primary_am = if primary_parquet { "using parquet" } else { "" };
    let foreign_am = if foreign_parquet { "using parquet" } else { "" };

    let create_tables_sql = format!(
        r#"
            CREATE TABLE users (
                user_id SERIAL PRIMARY KEY,
                username VARCHAR(255) NOT NULL,
                email VARCHAR(255) NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                UNIQUE (username, email)
            ) {};
            
            CREATE TABLE orders (
                order_id SERIAL PRIMARY KEY,
                username VARCHAR(255) NOT NULL,
                email VARCHAR(255) NOT NULL,
                order_total DECIMAL(10, 2) NOT NULL,
                order_date TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (username, email) REFERENCES users(username, email)
            ) {};
        "#,
        primary_am, foreign_am
    );

    create_tables_sql.execute(conn);

    r#"                        
        INSERT INTO users (username, email) 
        VALUES ('User1', 'user1@gmail.com'), ('User2', 'user2@gmail.com'), ('User3', 'user3@gmail.com'), ('User4', 'user4@gmail.com');
    "#
    .execute(conn);

    r#"
        INSERT INTO orders (username, email, order_total) VALUES ('User1', 'user1@gmail.com', 100.00), ('User1', 'user1@gmail.com', 200.00);
    "#
    .execute(conn);

    let count: (i64,) = "SELECT COUNT(*) FROM users".fetch_one(conn);
    assert_eq!(count, (4,));

    let count: (i64,) = "SELECT COUNT(*) FROM orders".fetch_one(conn);
    assert_eq!(count, (2,));

    match "INSERT INTO orders (username, email, order_total) VALUES ('User1', 'user2@gmail.com', 100.00)".execute_result(conn) {
        Err(err) => assert!(err.to_string().contains("violates foreign key constraint")),
        _ => panic!("Foreign key constraint violated"),
    };

    match "INSERT INTO orders (username, email, order_total) VALUES ('User1', 'user1@gmail.com', 300.00), ('User1', 'user2@gmail.com', 100.00)"
        .execute_result(conn)
    {
        Err(err) => assert!(err.to_string().contains("violates foreign key constraint")),
        _ => panic!("Foreign key constraint violated"),
    };

    let rows: Vec<(i32, i32)> = r#"
        SELECT u.user_id, o.order_id FROM orders o 
        JOIN users u on o.username = u.username AND o.email = u.email;
    "#
    .fetch(conn);

    let user_ids = [1, 1];
    let order_ids = [1, 2];

    assert!(rows.iter().map(|r| r.0).eq(user_ids));
    assert!(rows.iter().map(|r| r.1).eq(order_ids));
}
