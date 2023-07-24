import shelve
from core.load import elasticsearch, opensearch, pinecone, weaviate
from core.sdk.target import (
    ElasticSearchTarget,
    OpenSearchTarget,
    PineconeTarget,
    WeaviateTarget,
)
from kafka_connectors.secrets_handler import get_env_secret, SecretNotFoundError
from typing import Dict

sink_loader_mapping = {
    "elasticsearch": {
        "loader": elasticsearch.Loader,
        "target": ElasticSearchTarget,
        "config": {"host", "user", "password", "ssl_assert_fingerprint" "cloud_id"},
    },
    "opensearch": {
        "loader": opensearch.Loader,
        "target": OpenSearchTarget,
        "config": {"hosts", "user", "password", "use_ssl", "cacerts"},
    },
    "pinecone": {
        "loader": pinecone.Loader,
        "target": PineconeTarget,
        "config": {"api_key", "environment"},
    },
    "weaviate": {
        "loader": weaviate.Loader,
        "target": WeaviateTarget,
        "config": {"api_key", "url"},
    },
}


def populate_config(sink_type: str) -> Dict[str, str]:
    sink = sink_loader_mapping[sink_type]
    config = {}
    for k in sink["config"]:
        env_key = f"{sink_type.upper()}_{k.upper()}"
        try:
            secret = get_env_secret(env_key)
            config[k] = secret
        except SecretNotFoundError as e:
            print(e)
    return config


def load(key: str, value: Dict[str, str]) -> None:
    sink = value.get("sink")
    if sink not in sink_loader_mapping:
        raise NotImplementedError(f"sink {sink} not supported")

    index_name = value.get("index_name")
    if not index_name:
        raise ValueError("index_name not present")

    field_name = value.get("field_name")
    if not index_name:
        raise ValueError("field_name not present")

    doc = value.get("doc")
    if not index:
        raise ValueError("embedding not present")

    metadata = value.get("metadata")
    config = populate_config(sink)

    # Create the appropriate Loader object and call bulk_upsert_embeddings()
    loader_class = sink_loader_mapping[sink]["loader"]
    target = sink_loader_mapping[sink]["target"]
    loader = loader_class(**config)

    target.index_name = index_name
    target.field_name = field_name
    loader.bulk_upsert_embeddings(target, doc, [], metadata)
