import os
import uuid

from weaviate import Client, AuthApiKey

from core.load.base import Loader
from typing import Dict, List, Union, Optional, Any, cast
from core.sdk.target import WeaviateTarget, WeaviateVectorizer

DEFAULT_BATCH_SIZE = 100
UUID_NAMESPACE = uuid.NAMESPACE_DNS


class WeaviateLoader(Loader):
    def __init__(
        self,
        api_key: str,
        url: str,
        default_vectorizer: WeaviateVectorizer,
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
        default_vectorizer = str(self.default_vectorizer.value)

        self.wc.schema.create_class(
            {
                "class": index_name,
                "vectorizer": default_vectorizer,
                "moduleConfig": {default_vectorizer: self.default_vectorizer_config},
            }
        )

    def check_and_setup_index(
        self, target: WeaviateTarget, num_dimensions: int = 0
    ) -> None:
        index_name = target.index_name

        if not self._check_index_exists(index_name=index_name):
            self._create_index(index_name=index_name)

    @Loader.validate
    def bulk_upsert_embeddings(
        self,
        target: WeaviateTarget,
        embeddings: List[List[float]],
        ids: List[Union[str, int]],
        metadata: Optional[List[Dict[str, Any]]],
    ) -> None:
        class_name = target.index_name
        data_objects = metadata if metadata else [{} for _ in range(len(ids))]

        with self.wc.batch(
            batch_size=DEFAULT_BATCH_SIZE,
            num_workers=os.cpu_count(),
            dynamic=True,
        ):
            for embedding, id, data in zip(embeddings, ids, data_objects):
                if embedding:
                    self.wc.batch.add_data_object(
                        class_name=class_name,
                        data_object=data,
                        vector=embedding,
                        uuid=str(uuid.uuid5(UUID_NAMESPACE, str(id))),
                    )
                else:
                    self.wc.batch.add_data_object(
                        class_name=class_name,
                        data_object=data,
                        uuid=str(uuid.uuid5(UUID_NAMESPACE, str(id))),
                    )
