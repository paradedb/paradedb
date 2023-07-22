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
        "elasticsearch": elasticsearch.Loader,
        "opensearch": opensearch.Loader,
        "pinecone": pinecone.Loader,
        "weaviate": weaviate.Loader,
    }

    sink = value.get("sink")
    if sink not in sink_loader_mapping:
        raise NotImplementedError(f"sink {sink} not supported")

    index = value.get("index")
    if not index:
        raise NotImplementedError("index not present")

    # Create the appropriate Loader object and call bulk_upsert_embeddings()
    loader_class = sink_loader_mapping[sink]
    loader = loader_class()
    loader.bulk_upsert_embeddings()
