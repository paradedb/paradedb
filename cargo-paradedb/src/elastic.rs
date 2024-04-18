use once_cell::sync::Lazy;
use serde_json::json;

pub static ELASTIC_INDEX_CONFIG: Lazy<serde_json::Value> = Lazy::new(|| {
    json!({
        "settings": {
            "index": {
                "number_of_shards": 1,
                "number_of_replicas": 0
            },
            "analysis": {
                "analyzer": {
                    "default": {
                        "type": "standard"
                    }
                }
            }
        },
        "mappings": {
            "properties": {
                "message": {
                    "type": "text",
                    "analyzer": "standard"
                }
            }
        }
    })
});
