use soa_derive::StructOfArray;
use sqlx::FromRow;
use chrono::{NaiveDate, NaiveDateTime};

#[derive(Debug, PartialEq, FromRow, StructOfArray, Default)]
pub struct SimpleProductsTable {
    pub id: i32,
    pub description: String,
    pub category: String,
    pub rating: i32,
    pub in_stock: bool,
    pub metadata: serde_json::Value,
    pub created_at: NaiveDateTime,
    pub last_updated_date: NaiveDate
}

impl SimpleProductsTable {
    pub fn setup() -> String {
        SIMPLE_PRODUCTS_TABLE_SETUP.replace("%s", "id")
    }

    pub fn setup_with_key_field(key_field: &str) -> String {
        SIMPLE_PRODUCTS_TABLE_SETUP.replace("%s", key_field)
    }
}

static SIMPLE_PRODUCTS_TABLE_SETUP: &str = r#"
BEGIN;
    CALL paradedb.create_bm25_test_table(table_name => 'bm25_search', schema_name => 'paradedb');

    CALL paradedb.create_bm25(
    	index_name => 'bm25_search',
        table_name => 'bm25_search',
    	schema_name => 'paradedb',
        key_field => '%s',
        text_fields => '{"description": {}, "category": {}}',
    	numeric_fields => '{"rating": {}}',
    	boolean_fields => '{"in_stock": {}}',
    	json_fields => '{"metadata": {}}',
        datetime_fields => '{"created_at": {}, "last_updated_date": {}}'
    );
COMMIT;
"#;
