use pgrx::*;
use serde_json::{json, Map, Value};

#[pg_extern(immutable, parallel_safe)]
#[allow(clippy::too_many_arguments)]
pub fn field(
    name: &str,
    indexed: default!(Option<bool>, "NULL"),
    stored: default!(Option<bool>, "NULL"),
    fast: default!(Option<bool>, "NULL"),
    fieldnorms: default!(Option<bool>, "NULL"),
    record: default!(Option<String>, "NULL"),
    expand_dots: default!(Option<bool>, "NULL"),
    tokenizer: default!(Option<JsonB>, "NULL"),
    normalizer: default!(Option<String>, "NULL"),
) -> JsonB {
    let mut config = Map::new();

    indexed.map(|v| config.insert("indexed".to_string(), Value::Bool(v)));
    stored.map(|v| config.insert("stored".to_string(), Value::Bool(v)));
    fast.map(|v| config.insert("fast".to_string(), Value::Bool(v)));
    fieldnorms.map(|v| config.insert("fieldnorms".to_string(), Value::Bool(v)));
    record.map(|v| config.insert("record".to_string(), Value::String(v)));
    expand_dots.map(|v| config.insert("expand_dots".to_string(), Value::Bool(v)));
    tokenizer.map(|v| config.insert("tokenizer".to_string(), v.0));
    normalizer.map(|v| config.insert("normalizer".to_string(), Value::String(v)));

    JsonB(json!({ name: config }))
}

#[pg_extern(immutable, parallel_safe)]
pub fn tokenizer(
    name: &str,
    min_gram: default!(Option<i32>, "NULL"),
    max_gram: default!(Option<i32>, "NULL"),
    prefix_only: default!(Option<bool>, "NULL"),
    language: default!(Option<String>, "NULL"),
    pattern: default!(Option<String>, "NULL"),
) -> JsonB {
    let mut config = Map::new();

    config.insert("type".to_string(), Value::String(name.to_string()));

    min_gram.map(|v| config.insert("min_gram".to_string(), Value::Number(v.into())));
    max_gram.map(|v| config.insert("max_gram".to_string(), Value::Number(v.into())));
    prefix_only.map(|v| config.insert("prefix_only".to_string(), Value::Bool(v)));
    language.map(|v| config.insert("language".to_string(), Value::String(v)));
    pattern.map(|v| config.insert("pattern".to_string(), Value::String(v)));
    JsonB(json!(config))
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_field() {
        let expected = json!({
            "field1": {
                "indexed": true,
                "stored": false,
                "fast": true,
                "fieldnorms": false,
                "record": "position",
                "expand_dots": true,
                "tokenizer": {"type": "ngram", "min_gram": 4, "max_gram": 4, "prefix_only": false},
                "normalizer": "lowercase"
            }
        });

        let JsonB(actual) = field(
            "field1",
            Some(true),
            Some(false),
            Some(true),
            Some(false),
            Some("position".to_string()),
            Some(true),
            Some(tokenizer("ngram", Some(4), Some(4), Some(false), None, None)),
            Some("lowercase".to_string()),
        );

        assert_eq!(expected, actual);
    }
}
