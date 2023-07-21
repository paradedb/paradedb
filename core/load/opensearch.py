from opensearchpy import OpenSearch
from typing import List, Union, Optional, Dict, Any
from core.load.base import Loader
from core.sdk.target import OpenSearchTarget


class OpenSearchLoader(Loader):
    def __init__(
        self,
        hosts: List[Dict[str, str]],
        user: str,
        password: str,
        use_ssl: bool,
        cacerts: str,
    ) -> None:
        auth = (user, password)
        self.opensearch = OpenSearch(
            hosts=hosts,
            http_compress=True,  # enables gzip compression for request bodies
            http_auth=auth,
            use_ssl=use_ssl,
            verify_certs=use_ssl,
            ssl_assert_hostname=False,
            ssl_show_warn=False,
            ca_certs=cacerts,
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
        field_name = target.field_name

        if metadata is not None:
            docs: List[Dict[str, Any]] = []
            for doc_id, embedding, meta in zip(ids, embeddings, metadata):
                docs.append({"update": {"_index": index_name, "_id": doc_id}})
                docs.append(
                    {"doc": {field_name: embedding, **meta}, "doc_as_upsert": True}
                )

        else:
            docs = []
            for doc_id, embedding in zip(ids, embeddings):
                docs.append({"update": {"_index": index_name, "_id": doc_id}})
                docs.append(
                    {
                        "doc": {
                            field_name: embedding,
                        },
                        "doc_as_upsert": True,
                    }
                )
        self.opensearch.bulk(body=docs)
