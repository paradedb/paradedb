from typing import List, Callable

from core.transform.base import Embedding


class CustomEmbedding(Embedding):
    def __init__(self, func: Callable[[List[str]], List[List[float]]]):
        self.func = func

    def create_embeddings(self, documents: List[str]) -> List[List[float]]:
        return self.func(documents)
