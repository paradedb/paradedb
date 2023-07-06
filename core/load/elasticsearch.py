from abc import ABC, abstractmethod
from elasticsearch import Elasticsearch, helpers
from typing import Dict, List, Union
from core.load.base import Loader


class FieldTypeError(Exception):
    pass


class ElasticSearchLoader(Loader):
    def __init__(
        self,
        host: str = None,
        user: str = None,
        password: str = None,
        ssl_assert_fingerprint: str = None,
        cloud_id: str = None,
        index: bool = False,
        similarity: str = None,
    ):
        if index and similarity is None:
            raise ValueError("Similarity must be provided if index is True")

        kwargs = {
            "hosts": [host] if host else None,
            "basic_auth": (user, password) if user and password else None,
            "ssl_assert_fingerprint": ssl_assert_fingerprint
            if ssl_assert_fingerprint
            else None,
            "verify_certs": True if host else None,
            "cloud_id": cloud_id if cloud_id else None,
        }

        self.es = Elasticsearch(**{k: v for k, v in kwargs.items() if v is not None})
        self.index = index
        self.similarity = similarity

    ### Private Methods ###

    def _check_index_exists(self, index_name: str):
        return self.es.indices.exists(index=index_name)

    def _create_index(self, index_name: str, field_name: str, num_dimensions: int):
        if self._check_index_exists(index_name=index_name):
            raise IndexAlreadyExistsError(f"Index {index_name} already exists")

        mapping = {
            "mappings": {
                "dynamic": True,
                "_source": {"enabled": True},
                "properties": {
                    field_name: {
                        "type": "dense_vector",
                        "dims": num_dimensions,
                        "index": self.index,
                    }
                },
            }
        }

        if self.similarity is not None:
            mapping["mappings"]["properties"][field_name][
                "similarity"
            ] = self.similarity

        self.es.indices.create(index=index_name, body=mapping)

    def _check_and_setup_index(
        self, index_name: str, field_name: str, num_dimensions: int
    ):
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
                        f"Field '{field_name}' expects {field_mapping['dims']} dimensions but the embedding has {len(embedding)}"
                    )
            else:
                # The field does not exist, create it
                new_field_mapping = {
                    "properties": {
                        field_name: {
                            "type": "dense_vector",
                            "dims": num_dimensions,
                            "index": True,
                        }
                    }
                }
                self.es.indices.put_mapping(index=index_name, body=new_field_mapping)

    ### Public Methods ###

    def upsert_embedding(
        self,
        index_name: str,
        embedding: List[float],
        id: Union[str, int],
        field_name: str,
        metadata: Dict[str, any] = None,
    ):
        self._check_and_setup_index(
            index_name=index_name,
            field_name=field_name,
            num_dimensions=len(embedding),
        )

        doc = dict()
        doc[field_name] = embedding

        if not metadata is None:
            doc.update(metadata)

        self.es.update(
            index=index_name, id=id, body={"doc": doc, "doc_as_upsert": True}
        )

    def bulk_upsert_embeddings(
        self,
        index_name: str,
        embeddings: List[List[float]],
        ids: List[Union[str, int]],
        field_name: str,
        metadata: List[Dict[str, any]] = None,
    ):
        num_dimensions = len(embeddings[0])

        if not all(len(embedding) == num_dimensions for embedding in embeddings):
            raise ValueError("Not all embeddings have the same number of dimensions")

        self._check_and_setup_index(
            index_name=index_name,
            field_name=field_name,
            num_dimensions=num_dimensions,
        )

        if metadata is None:
            metadata = [{}] * len(ids)

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
