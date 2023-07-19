from opensearchpy import OpenSearch
from typing import List, Union, Optional, Dict, Tuple, Any
from core.load.base import Loader
from core.sdk.target import OpenSearchTarget


class OpenSearchLoader(Loader):
    def __init__(
        self, hosts: List[Dict[str, str]], user: str, password: str, use_ssl: bool
    ) -> None:
        auth = (user, password)
        self.opensearch = OpenSearch(
            hosts=hosts,
            http_compress=True,  # enables gzip compression for request bodies
            http_auth=auth,
            use_ssl=use_ssl,
        )

    def _check_index_exists(self, index_name: str) -> bool:
        return self.opensearch.indices.exists(index_name)

    def _create_index(self, index_name: str) -> None:
        index_body = {"settings": {"index": {"number_of_shards": 4}}}
        self.opensearch.indices.create(index_name, body=index_body)

    def check_and_setup_index(
        self, target: OpenSearchTarget, num_dimensions: int = 0
    ) -> None:
        index_name = target.index_name

        if not self._check_index_exists(index_name=index_name):
            self._create_index(index_name=index_name)

    @Loader.validate
    def bulk_upsert_embeddings(
        self,
        target: OpenSearchTarget,
        embeddings: List[List[float]],
        ids: List[Union[str, int]],
        metadata: Optional[List[Dict[str, Any]]],
    ) -> None:
        index_name = target.index_name

        if metadata is not None:
            docs = []
            for doc_id, embedding, meta in zip(ids, embeddings, metadata):
                docs.append({"index": {"_index": index_name, "_id": doc_id}})
                docs.append(
                    {
                        "values": embedding,
                        "metadata": metadata,
                    }
                )

        else:
            docs = []
            for doc_id, embedding in zip(ids, embeddings):
                docs.append({"index": {"_index": index_name, "_id": doc_id}})
                docs.append(
                    {
                        "values": embedding,
                    }
                )
        self.opensearch.bulk(docs)
