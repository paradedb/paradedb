use pgrx::*;
use url::{form_urlencoded, Url};

const BASE_GITHUB_URL: &str = "https://github.com";

#[pg_extern(sql = r#"
    DO $$
    BEGIN
    IF NOT EXISTS (
        SELECT 1 
        FROM pg_namespace 
        WHERE nspname = 'paradedb'
    ) THEN
        CREATE SCHEMA paradedb;
    END IF;
    IF NOT EXISTS (SELECT FROM pg_proc p
        JOIN pg_namespace n ON p.pronamespace = n.oid
        WHERE n.nspname = 'paradedb' AND p.proname = 'help') THEN
            CREATE OR REPLACE FUNCTION paradedb.help(subject TEXT, body TEXT) RETURNS TEXT
            STRICT
            LANGUAGE c 
            AS '@MODULE_PATHNAME@', '@FUNCTION_NAME@';
        END IF;
    END $$;
"#)]
pub fn help(subject: &str, body: &str) -> String {
    let mut url = Url::parse(BASE_GITHUB_URL).expect("Failed to parse Github URL");
    url.set_path("/orgs/paradedb/discussions/new");

    let query = form_urlencoded::Serializer::new(String::new())
        .append_pair("category", "q-a")
        .append_pair("title", subject)
        .append_pair("body", body)
        .finish();

    url.set_query(Some(&query));

    format!("To file this ticket, visit {}", String::from(url))
}
