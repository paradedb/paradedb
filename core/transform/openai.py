import openai

from typing import List, cast

from core.transform.base import Embedding


class OpenAIEmbedding(Embedding):
    def __init__(self, api_key: str, model: str):
        openai.api_key = api_key
        self.model = model

    def create_embeddings(self, documents: List[str]) -> List[List[float]]:
        return cast(
            List[List[float]],
            openai.Embedding.create(input=[documents], model=self.model),  # type: ignore
        )
