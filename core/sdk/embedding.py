from pydantic import BaseModel
from typing import Callable, List


class OpenAIEmbedding(BaseModel):
    api_key: str
    model: str


class SentenceTransformerEmbedding(BaseModel):
    model: str


class CohereEmbedding(BaseModel):
    api_key: str
    model: str


class CustomEmbedding(BaseModel):
    func: Callable[[List[str]], List[List[float]]]


class Embedding:
    @classmethod
    def OpenAI(cls, api_key: str, model: str) -> OpenAIEmbedding:
        return OpenAIEmbedding(api_key=api_key, model=model)

    @classmethod
    def SentenceTransformer(
        cls, model: str = "all-MiniLM-L6-v2"
    ) -> SentenceTransformerEmbedding:
        return SentenceTransformerEmbedding(model=model)

    @classmethod
    def Cohere(cls, api_key: str, model: str) -> CohereEmbedding:
        return CohereEmbedding(api_key=api_key, model=model)

    @classmethod
    def Custom(cls, func: Callable[[List[str]], List[List[float]]]) -> CustomEmbedding:
        return CustomEmbedding(func=func)
