from pydantic import BaseModel


class OpenAI(BaseModel):
    api_key: str
    model: str


class Embedding:
    @classmethod
    def OpenAI(cls, api_key: str, model: str) -> OpenAI:
        return OpenAI(api_key=api_key, model=model)
