use pgrx::*;
use url::{form_urlencoded, Url};

const BASE_GITHUB_URL: &str = "https://github.com";

#[pg_extern]
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
