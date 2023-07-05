import openai

from sentence_transformers import SentenceTransformer
from typing import List

from core.transform.base import Embedding


class OpenAIEmbedding(Embedding):
    def __init__(self, api_key: str, model: str):
        openai.api_key = api_key
        self.model = model

    def create_embeddings(self, documents: List[str]):
        return openai.Embedding.create(input=[document], model=self.model)


class SentenceTransformerEmbedding(Embedding):
    def __init__(self, model: str):
        self.model = SentenceTransformer(model)

    def create_embeddings(self, documents: List[str]):
        return self.model.encode(documents)
