from qdrant_client import QdrantClient
from qdrant_client.models import Distance, VectorParams, PointStruct

from core.load.base import Loader
from typing import Dict, List, Union, Optional, Any, cast
from core.sdk.target import QdrantTarget, QdrantSimilarity


class QdrantLoader(Loader):
    def __init__(
        self,
        host: Optional[str] = None,
        port: Optional[int] = None,
        url: Optional[str] = None,
        api_key: Optional[str] = None,
        similarity: Optional[QdrantSimilarity] = None,
    ) -> None:
        if url and api_key:
            self.client = QdrantClient(url=url, api_key=api_key)
        elif host and port:
            self.client = QdrantClient(host=host, port=port)
        else:
            raise ValueError(
                "Either url and api_key (for Qdrant Cloud) or host and port (for self-hosted Qdrant) must be provided"
            )

        self.similarity = similarity

    def _check_index_exists(self, index_name: str) -> bool:
        response = self.client.get_collections()
        return index_name in [collection.name for collection in response.collections]

    def _create_index(self, index_name: str) -> None:
        similarity = self.similarity.value if self.similarity else Distance.COSINE
        self.client.recreate_collection(
            collection_name=index_name,
            vectors_config=VectorParams(size=100, distance=cast(Distance, similarity)),
        )

    def check_and_setup_index(
        self, target: QdrantTarget, num_dimensions: int = 0
    ) -> None:
        index_name = target.index_name

        if not self._check_index_exists(index_name=index_name):
            self._create_index(index_name=index_name)

    @Loader.validate
    def bulk_upsert_embeddings(
        self,
        target: QdrantTarget,
        embeddings: List[List[float]],
        ids: List[Union[str, int]],
        metadata: Optional[List[Dict[str, Any]]],
    ) -> None:
        collection_name = target.index_name
        metadata = metadata if metadata else [{} for _ in range(len(ids))]
        points = [
            PointStruct(id=_id, vector=vector, payload=metadata)
            for (_id, vector, metadata) in zip(ids, embeddings, metadata)
        ]

        self.client.upsert(collection_name=collection_name, points=points)
