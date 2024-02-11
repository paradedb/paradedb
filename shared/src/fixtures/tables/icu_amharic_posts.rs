use soa_derive::StructOfArray;
use sqlx::FromRow;

#[derive(Debug, PartialEq, FromRow, StructOfArray, Default)]
pub struct IcuAmharicPostsTable {
    pub id: i32,
    pub author: String,
    pub title: String,
    pub message: String,
}

impl IcuAmharicPostsTable {
    pub fn setup() -> &'static str {
        ICU_AMHARIC_POSTS
    }
}

static ICU_AMHARIC_POSTS: &str = r#"
CREATE TABLE IF NOT EXISTS icu_amharic_posts (
    id SERIAL PRIMARY KEY,
    author TEXT,
    title TEXT,
    message TEXT
);
INSERT INTO icu_amharic_posts (author, title, message)
VALUES
    ('መሐመድ', 'መደመር ተጨማሪ', 'እንኳን ነበር በመደመር ተጨማሪ፣ በደስታ እና በልዩ ዝናብ ይከብዳል።'),
    ('ፋትስ', 'የምስሉ ማህበረሰብ', 'በዚህ ግዜ የምስሉ ማህበረሰብ እና እንደዚህ ዝናብ ይችላል።'),
    ('አለም', 'መረጃዎች ለመማር', 'እነዚህ መረጃዎች የምስሉ ለመማር በእያንዳንዱ ላይ ይመልከቱ።');
"#;
