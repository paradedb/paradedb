use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use soa_derive::StructOfArray;
use sqlx::FromRow;
use time::Date;

#[derive(Debug, PartialEq, FromRow, StructOfArray, Serialize, Deserialize)]
pub struct UserSessionLogsTable {
    pub id: i32,
    pub event_date: Option<Date>,
    pub user_id: Option<i32>,
    pub event_name: Option<String>,
    pub session_duration: Option<i32>,
    pub page_views: Option<i32>,
    pub revenue: Option<BigDecimal>,
}

impl UserSessionLogsTable {
    pub fn setup_parquet() -> String {
        USER_SESSION_LOGS_TABLE_SETUP.replace("{}", "parquet")
    }

    pub fn setup_heap() -> String {
        USER_SESSION_LOGS_TABLE_SETUP.replace("{}", "heap")
    }
}

static USER_SESSION_LOGS_TABLE_SETUP: &str = r#"
CREATE TABLE user_session_logs (
    id SERIAL PRIMARY KEY,
    event_date TIMESTAMP,
    user_id INT,
    event_name VARCHAR(50),
    session_duration INT,
    page_views INT,
    revenue DECIMAL(10, 2)
) USING {};

INSERT INTO user_session_logs
(event_date, user_id, event_name, session_duration, page_views, revenue)
VALUES
('2024-01-01 10:23:54', 1, 'Login', 300, 5, 20.00),
('2024-01-02 10:23:54', 2, 'Purchase', 450, 8, 150.50),
('2024-01-03 10:23:54', 3, 'Logout', 100, 2, 0.00),
('2024-01-04 10:23:54', 4, 'Signup', 200, 3, 0.00),
('2024-01-05 10:23:54', 5, 'ViewProduct', 350, 6, 30.75),
('2024-01-06 10:23:54', 1, 'AddToCart', 500, 10, 75.00),
('2024-01-07 10:23:54', 2, 'RemoveFromCart', 250, 4, 0.00),
('2024-01-08 10:23:54', 3, 'Checkout', 400, 7, 200.25),
('2024-01-09 10:23:54', 4, 'Payment', 550, 11, 300.00),
('2024-01-10 10:23:54', 5, 'Review', 600, 9, 50.00),
('2024-01-11 10:23:54', 6, 'Login', 320, 3, 0.00),
('2024-01-12 10:23:54', 7, 'Purchase', 480, 7, 125.30),
('2024-01-13 10:23:54', 8, 'Logout', 150, 2, 0.00),
('2024-01-14 10:23:54', 9, 'Signup', 240, 4, 0.00),
('2024-01-15 10:23:54', 10, 'ViewProduct', 360, 5, 45.00),
('2024-01-16 10:23:54', 6, 'AddToCart', 510, 9, 80.00),
('2024-01-17 10:23:54', 7, 'RemoveFromCart', 270, 3, 0.00),
('2024-01-18 10:23:54', 8, 'Checkout', 430, 6, 175.50),
('2024-01-19 10:23:54', 9, 'Payment', 560, 12, 250.00),
('2024-01-20 10:23:54', 10, 'Review', 610, 10, 60.00);
"#;
