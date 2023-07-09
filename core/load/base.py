from abc import ABC, abstractmethod
from typing import Dict, List, Union, Any, Optional


class Loader(ABC):
    @abstractmethod
    def check_and_setup_index(self, *args, **kwargs) -> None:
        pass

    def upsert_embedding(self, *args, **kwargs) -> None:
        pass

    @abstractmethod
    def bulk_upsert_embeddings(self, *args, **kwargs) -> None:
        pass
