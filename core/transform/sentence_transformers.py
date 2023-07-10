from sentence_transformers import SentenceTransformer
from typing import List, cast

from core.transform.base import Embedding


class SentenceTransformerEmbedding(Embedding):
    def __init__(self, model: str):
        self.model = SentenceTransformer(model)

    def create_embeddings(self, documents: List[str]) -> List[List[float]]:
        return cast(List[List[float]], self.model.encode(documents).tolist())
