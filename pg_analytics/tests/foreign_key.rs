mod fixtures;

use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn foreign_key_on_both(mut conn: PgConnection) {
    r#"
        CREATE TABLE users (
            user_id SERIAL PRIMARY KEY,
            username VARCHAR(255) NOT NULL,
            email VARCHAR(255) UNIQUE NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        ) using parquet;
        
        CREATE TABLE orders (
            order_id SERIAL PRIMARY KEY,
            user_id INT NOT NULL,
            order_total DECIMAL(10, 2) NOT NULL,
            order_date TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES users(user_id)
        ) using parquet;
    "#
    .execute(&mut conn);

    r#"                        
        INSERT INTO users (username, email) 
        VALUES ('User1', 'user1@example.com'), ('User2', 'user2@example.com');
    "#
    .execute(&mut conn);

    r#"
        INSERT INTO orders (user_id, order_total) VALUES (1, 100.00), (2, 200.00);
    "#
    .execute(&mut conn);

    let count: (i64,) = "SELECT COUNT(*) FROM users".fetch_one(&mut conn);
    assert_eq!(count, (2,));

    let count: (i64,) = "SELECT COUNT(*) FROM orders".fetch_one(&mut conn);
    assert_eq!(count, (2,));

    match "INSERT INTO orders (user_id, order_total) VALUES (3, 300.00)".execute_result(&mut conn) {
        Err(err) => assert!(err.to_string().contains("violates foreign key constraint")),
        _ => panic!("Foreign key constraint violated"),
    };
}

// #[rstest]
// fn foreign_key_on_heap(mut conn: PgConnection) {

// }

// #[rstest]
// fn foreign_key_on_parquet(mut conn: PgConnection) {

// }
