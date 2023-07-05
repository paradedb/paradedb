from typing import Union, Tuple

from retake.embedding import OpenAI
from retake.source import PostgresSource
from retake.transform import PostgresTransform
from retake.sink import ElasticSearchSink
from retake.target import ElasticSearchTarget
from core.load.elasticsearch import ElasticSearchLoader

Source = Union[PostgresSource]
Transform = Union[PostgresTransform]
Embedding = Union[OpenAI]
Sink = Union[ElasticSearchSink]
Target = Union[ElasticSearchTarget]


class Pipeline:
    def _init__(
        self,
        name: str,
        source: Source,
        transform: Transform,
        embedding: Embedding,
        sink: Sink,
        target: Target,
    ):
        self.name = name
        self.source = source
        self.transform = transform
        self.embedding = embedding
        self.sink = sink
        self.target = target

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

    def sync_once(self):
        # Connect to DB
        postgres = PostgresExtractor(self.source.dsn)
        # Initialize DB loader
        loader = ElasticSearchLoader()
        # Extract, transform, and load chunks as embeddings
        for chunk in postgres.fetch_rows(
            self.transform.relation, self.transform.columns
        ):
            for row in chunk:
                document = self._apply_transform(row)
                metadata = self._create_metadata(row)
                embedding = self._create_embedding(transformed)
                # loader.upsert_embedding(embedding, row[self.transform.primary_key], metadata)

    def sync_real_time(self, cdc_server_url: str, optional_webhook: str = None):
        pass
