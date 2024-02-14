use soa_derive::StructOfArray;
use sqlx::FromRow;

#[derive(Debug, PartialEq, FromRow, StructOfArray, Default)]
pub struct IcuGreekPostsTable {
    pub id: i32,
    pub author: String,
    pub title: String,
    pub message: String,
}

impl IcuGreekPostsTable {
    pub fn setup() -> &'static str {
        ICU_GREEK_POSTS
    }
}

static ICU_GREEK_POSTS: &str = r#"
CREATE TABLE IF NOT EXISTS icu_greek_posts (
    id SERIAL PRIMARY KEY,
    author TEXT,
    title TEXT,
    message TEXT
);
INSERT INTO icu_greek_posts (author, title, message)
VALUES
    ('Δημήτρης', 'Η πρώτη άρθρο', 'Καλώς ήρθες στο πρώτο άρθρο. Ελπίζω να βρεις το περιεχόμενο χρήσιμο και ενδιαφέρον.'),
    ('Σοφία', 'Ταξίδι στην Ανατολή', 'Σε αυτό το άρθρο, θα εξερευνήσουμε ένα συναρπαστικό ταξίδι στην Ανατολή και θα γνωρίσουμε διάφορες πολιτισμικές και ιστορικές πτυχές.'),
    ('Αλέξανδρος', 'Συμβουλές για την επιτυχία', 'Εδώ παρέχουμε μερικές πολύτιμες συμβουλές για την επίτευξη επιτυχίας στην επαγγελματική και προσωπική σας ζωή. Επωφεληθείτε από αυτές και επιτύχετε τους στόχους σας.');
"#;
