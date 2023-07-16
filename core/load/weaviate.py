import os

from uuid import UUID
from weaviate import Client, Schema, AuthApiKey

from abc import ABC, abstractmethod
from core.load.base import Loader
from typing import Dict, List, Union, Optional, Any, cast
from core.sdk.target import WeaviateTarget, WeaviateVecotorizer

DEFAULT_BATCH_SIZE = 100
NUM_RETRIES = 4


class WeaviateLoader(Loader):
    def __init__(
        self,
        api_key: str,
        url: str,
        default_vectorizer: WeaviateVecotorizer,
        default_vectorizer_config: Dict[str, str],
    ) -> None:
        self.wc = Client(
            url=url,
            auth_client_secret=AuthApiKey(api_key=api_key),
        )
        self.default_vectorizer = default_vectorizer
        self.default_vectorizer_config = default_vectorizer_config

    def _check_index_exists(self, index_name: str) -> bool:
        return cast(bool, self.wc.schema.exists(index_name))

    def _create_index(self, index_name: str) -> None:
        self.wc.schema.create_class(
            {
                "class": index_name,
                "vectorizer": self.default_vectorizer,
                "moduleConfig": {
                    self.default_vectorizer: self.default_vectorizer_config
                },
            }
        )

    def check_and_setup_index(
        self, target: WeaviateTarget, num_dimensions: int = 0
    ) -> None:
        index_name = target.index_name

        if not self._check_index_exists(index_name=index_name):
            self._create_index(index_name=index_name)

    def upsert_embedding(
        self,
        target: WeaviateTarget,
        embedding: Optional[List[float]],
        id: Union[str, int],
        metadata: Optional[Dict[str, Any]],
    ) -> None:
        class_name = target.index_name
        vectorizer = target.default_vectorizer
        vectorizer_config = target.default_vectorizer_config

        data_object = metadata if metadata else {}

        with self.wc.batch(batch_size=DEFAULT_BATCH_SIZE) as batch:
            if embedding:
                self.wc.batch.add_data_object(
                    class_name=class_name,
                    data_object=data_object,
                    vector=embedding,
                    uuid=str(UUID(str(id))),
                )
            else:
                self.wc.batch.add_data_object(
                    class_name=class_name,
                    data_object=data_object,
                    uuid=str(UUID(str(id))),
                )

    def bulk_upsert_embeddings(
        self,
        target: WeaviateTarget,
        embeddings: List[List[float]],
        ids: List[Union[str, int]],
        metadatas: Optional[List[Dict[str, Any]]],
    ) -> None:
        class_name = target.index_name
        vectorizer = target.default_vectorizer
        vectorizer_config = target.default_vectorizer_config

        data_objects = metadatas if metadatas else [{} for _ in range(len(ids))]

        with self.wc.batch(
            batch_size=DEFAULT_BATCH_SIZE,
            num_workers=os.cpu_count(),
            num_retries=NUM_RETRIES,
            connection_error_retries=NUM_RETRIES,
            dynamic=True,
        ):
            for embedding, id, metadata in zip(embeddings, ids, data_objects):
                if embedding:
                    self.wc.batch.add_data_object(
                        class_name=class_name,
                        data_object=metadata,
                        vector=embedding,
                        uuid=str(UUID(str(id))),
                    )
                else:
                    self.wc.batch.add_data_object(
                        class_name=class_name,
                        data_object=metadata,
                        uuid=str(UUID(str(id))),
                    )
