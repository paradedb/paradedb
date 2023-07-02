from pydantic import BaseModel
from typing import Union


class Collection(BaseModel):
    name: str
    source: Union[PostgresSource]
    transform: Union[PostgresTransform]
    model: Union[OpenAIModel]


class Collection:
    def _init__(
        self, name: str, source: Source, transform: Transform, model: EmbeddingModel
    ):
        return Collection(name=name, source=source, transform=transform, model=model)
