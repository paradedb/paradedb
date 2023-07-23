from elasticsearch import Elasticsearch, helpers
from typing import Dict, List, Union, Optional, Any, cast
from core.load.base import Loader
from core.sdk.target import ElasticSearchTarget


class FieldTypeError(Exception):
    pass


class ElasticSearchLoader(Loader):
    def __init__(
        self,
        host: Optional[str] = None,
        user: Optional[str] = None,
        password: Optional[str] = None,
        ssl_assert_fingerprint: Optional[str] = None,
        cloud_id: Optional[str] = None,
        index: Optional[bool] = False,
        similarity: Optional[str] = None,
    ) -> None:
        if index and similarity is None:
            raise ValueError("Similarity must be provided if index is True")

        if cloud_id:
            self.es = Elasticsearch(cloud_id=cloud_id)
        elif host and user and password and ssl_assert_fingerprint:
            self.es = Elasticsearch(
                hosts=[host],
                basic_auth=(user, password),
                ssl_assert_fingerprint=ssl_assert_fingerprint,
                verify_certs=True,
            )
        elif host and user and password:
            self.es = Elasticsearch(
                hosts=[host],
                basic_auth=(user, password),
                verify_certs=False,
            )
        else:
            raise ValueError(
                "Either cloud_id or host, user, and password must be provided"
            )

        self.index = index
        self.similarity = similarity

    def _check_index_exists(self, index_name: str) -> bool:
        return cast(bool, self.es.indices.exists(index=index_name))

    def _create_index(
        self, index_name: str, field_name: str, num_dimensions: int
    ) -> None:
        mapping = cast(
            Dict[str, Any],
            {
                "dynamic": True,
                "_source": {"enabled": True},
                "properties": {
                    field_name: {
                        "type": "dense_vector",
                        "dims": num_dimensions,
                        "index": self.index,
                    }
                },
            },
        )

        if self.similarity is not None:
            mapping["properties"][field_name]["similarity"] = self.similarity

        self.es.indices.create(index=index_name, mappings=mapping)

    # Public Methods

    def check_and_setup_index(
        self, target: ElasticSearchTarget, num_dimensions: int
    ) -> None:
        index_name = target.index_name
        field_name = target.field_name

        if not self._check_index_exists(index_name=index_name):
            self._create_index(
                index_name=index_name,
                field_name=field_name,
                num_dimensions=num_dimensions,
            )
        else:
            current_mapping = self.es.indices.get_mapping(index=index_name)
            if field_name in current_mapping[index_name]["mappings"]["properties"]:
                # The field exists, check if it's a dense_vector field with the correct number of dimensions
                field_mapping = current_mapping[index_name]["mappings"]["properties"][
                    field_name
                ]
                if field_mapping["type"] != "dense_vector":
                    raise FieldTypeError(
                        f"Field '{field_name}' exists but is not a dense_vector field"
                    )
                if field_mapping["dims"] != num_dimensions:
                    raise FieldTypeError(
                        f"Field '{field_name}' expects {field_mapping['dims']} dimensions but the embedding has {num_dimensions}"
                    )
            else:
                # The field does not exist, create it
                new_field_mapping = {
                    field_name: {
                        "type": "dense_vector",
                        "dims": num_dimensions,
                        "index": True,
                    }
                }
                self.es.indices.put_mapping(
                    index=index_name, properties=new_field_mapping
                )

    # Public Methods

    @Loader.validate
    def bulk_upsert_embeddings(
        self,
        target: ElasticSearchTarget,
        embeddings: List[List[float]],
        ids: List[Union[str, int]],
        metadata: Optional[List[Dict[str, Any]]],
    ) -> None:
        index_name = target.index_name
        field_name = target.field_name
        num_embeddings = len(embeddings)

        if metadata is None:
            metadata = [{}] * num_embeddings

        docs = [
            {"_id": doc_id, "_source": {**{field_name: embedding}, **meta}}
            for doc_id, embedding, meta in zip(ids, embeddings, metadata)
        ]

        actions = [
            {
                "_op_type": "update",
                "_index": index_name,
                "_id": doc["_id"],
                "doc": doc["_source"],
                "doc_as_upsert": True,
            }
            for doc in docs
        ]

        helpers.bulk(self.es, actions)
