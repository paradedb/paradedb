from abc import ABC, abstractmethod
from typing import Dict, List, Union


class Loader(ABC):
    @abstractmethod
    def upsert_embedding(
        self,
        document: str,
        embedding: List[float],
        id: Union[str, int],
        field_name: str,
        metadata: Dict[str, any] = None,
    ):
        pass

    @abstractmethod
    def bulk_upsert_embeddings(
        self,
        documents: List[str],
        embeddings: List[List[float]],
        id: Union[str, int],
        field_name: str,
        metadata: Dict[str, any] = None,
    ):
        pass
