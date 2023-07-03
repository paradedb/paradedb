from pydantic import BaseModel
from typing import Union, Tuple

from core.transform.embeddings import OpenAI
from core.extract.postgres import PostgresReader
from core.load.opensearch import OpenSearch


class Collection(BaseModel):
    name: str
    source: Union[PostgresSource]
    transform: Union[PostgresTransform]
    model: Union[OpenAIModel]


class Collection:
    def _init__(
        self, name: str, source: Source, transform: Transform, embedding: Embedding
    ):
        self.name = name
        self.source = source
        self.transform = transform
        self.embedding = embedding
        self.cdc = None

    def _create_embedding(self, document: str):
        model = OpenAI(api_key=self.embedding.api_key, model=self.embedding.model)
        return model.create_embedding(document)

    def _apply_transform(self, row: Tuple[str, ...]):
        return self.transform.transform_func(*row)

    def _create_metadata(self, row: Tuple[str, ...]):
        if self.transform.optional_metadata:
            return self.transform.optional_metadata(*row)
        else:
            return None

    def create(self):
        # Connect to DB
        postgres = PostgresReader(self.source.dsn)
        # Initialize DB loader
        opensearch = OpenSearch()
        # Extract, transform, and load chunks as embeddings
        for chunk in postgres.fetch_rows(
            self.transform.relation, self.transform.columns
        ):
            for row in chunk:
                document = self._apply_transform(row)
                metadata = self._create_metadata(row)
                embedding = self._create_embedding(transformed)
                opensearch.insert(embedding, row[self.transform.primary_key], metadata)
