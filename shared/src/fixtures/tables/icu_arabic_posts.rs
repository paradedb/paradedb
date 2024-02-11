use soa_derive::StructOfArray;
use sqlx::FromRow;

#[derive(Debug, PartialEq, FromRow, StructOfArray, Default)]
pub struct IcuArabicPostsTable {
    pub id: i32,
    pub author: String,
    pub title: String,
    pub message: String,
}

impl IcuArabicPostsTable {
    pub fn setup() -> &'static str {
        ICU_ARABIC_POSTS
    }
}

static ICU_ARABIC_POSTS: &str = r#"
CREATE TABLE IF NOT EXISTS icu_arabic_posts (
    id SERIAL PRIMARY KEY,
    author TEXT,
    title TEXT,
    message TEXT
);

INSERT INTO icu_arabic_posts (author, title, message)
VALUES
    ('فاطمة', 'رحلة إلى الشرق', 'في هذا المقال، سنستكشف رحلة مثيرة إلى الشرق ونتعرف على ثقافات مختلفة وتاريخها الغني'),
    ('محمد','رحلة إلى السوق مع أبي', 'مرحباً بك في المقالة الأولى. أتمنى أن تجد المحتوى مفيدًا ومثيرًا للاهتمام'),
    ('أحمد', 'نصائح للنجاح', 'هنا نقدم لك بعض النصائح القيمة لتحقيق النجاح في حياتك المهنية والشخصية. استفد منها وحقق أهدافك');
"#;
