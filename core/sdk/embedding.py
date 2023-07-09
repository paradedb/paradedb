from pydantic import BaseModel


class OpenAIEmbedding(BaseModel):
    api_key: str
    model: str


class SentenceTransformerEmbedding(BaseModel):
    model: str


class Embedding:
    @classmethod
    def OpenAI(cls, api_key: str, model: str) -> OpenAIEmbedding:
        return OpenAIEmbedding(api_key=api_key, model=model)

    @classmethod
    def SentenceTransformer(
        cls, model: str = "all-MiniLM-L6-v2"
    ) -> SentenceTransformerEmbedding:
        return SentenceTransformerEmbedding(model=model)

    # TODO: Add more embedding models, e.g. Cohere, Google, custom model, etc.
