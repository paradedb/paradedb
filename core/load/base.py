from abc import ABC, abstractmethod
from typing import Dict, List, Union, Any, Optional


class Loader(ABC):
    @abstractmethod
    def upsert_embedding(
        self,
        index_name: str,
        embedding: List[float],
        id: Union[str, int],
        field_name: str,
        metadata: Optional[Dict[str, Any]],
    ) -> None:
        pass

    @abstractmethod
    def bulk_upsert_embeddings(
        self,
        index_name: str,
        embeddings: List[List[float]],
        ids: List[Union[str, int]],
        field_name: str,
        metadata: Optional[List[Dict[str, Any]]],
    ) -> None:
        pass
