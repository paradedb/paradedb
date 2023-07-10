import cohere

from typing import List, cast

from core.transform.base import Embedding


class CohereEmbedding(Embedding):
    def __init__(self, api_key: str, model: str):
        self.client = cohere.Client(api_key)
        self.model = model

    def create_embeddings(self, documents: List[str]) -> List[List[float]]:
        results = self.client.embed(
            texts=documents,
            model=self.model,
        )
        return cast(List[List[float]], results.embeddings)
