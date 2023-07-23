from core.load import elasticsearch, opensearch, pinecone, weaviate
from core.sdk.target import (
    ElasticSearchTarget,
    OpenSearchTarget,
    PineconeTarget,
    WeaviateTarget,
)
from typing import Dict


def load(key: str, value: Dict[str, str]) -> None:
    sink_loader_mapping = {
        "elasticsearch": {
            "loader": elasticsearch.Loader,
            "target": ElasticSearchTarget,
        },
        "opensearch": {"loader": opensearch.Loader, "target": OpenSearchTarget},
        "pinecone": {"loader": pinecone.Loader, "target": PineconeTarget},
        "weaviate": {"loader": weaviate.Loader, "target": WeaviateTarget},
    }

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

    print(key)
    
    # Create the appropriate Loader object and call bulk_upsert_embeddings()
    loader_class = sink_loader_mapping[sink]["loader"]
    target = sink_loader_mapping[sink]["target"]
    loader = loader_class()

    target.index_name = index_name
    target.field_name = field_name
    loader.bulk_upsert_embeddings(target, doc, [], metadata)
