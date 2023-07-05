from abc import ABC, abstractmethod
from typing import List


class Embedding(ABC):
    @abstractmethod
    def create_embeddings(self, document: List[str]):
        pass
