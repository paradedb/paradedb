from tqdm import tqdm
from typing import Union, Tuple, Callable, Any, Optional, Dict, cast

from core.sdk.embedding import (
    OpenAIEmbedding,
    SentenceTransformerEmbedding,
    CohereEmbedding,
    CustomEmbedding,
)
from core.sdk.source import PostgresSource
from core.sdk.transform import PostgresTransform
from core.sdk.sink import ElasticSearchSink, PineconeSink, WeaviateSink
from core.sdk.target import ElasticSearchTarget, PineconeTarget, WeaviateTarget
from core.sdk.realtime import RealtimeServer
from core.load.elasticsearch import ElasticSearchLoader
from core.load.pinecone import PineconeLoader
from core.load.weaviate import WeaviateLoader
from core.extract.postgres import PostgresExtractor
from core.transform.openai import OpenAIEmbedding as OpenAI
from core.transform.sentence_transformers import (
    SentenceTransformerEmbedding as SentenceTransformer,
)
from core.transform.cohere import CohereEmbedding as Cohere
from core.transform.custom import CustomEmbedding as Custom
from streams.app import register_agents, start_worker

Source = Union[PostgresSource]
Transform = Union[PostgresTransform]
Embedding = Union[
    OpenAIEmbedding, SentenceTransformerEmbedding, CohereEmbedding, CustomEmbedding
]
Sink = Union[ElasticSearchSink, PineconeSink, WeaviateSink]
Target = Union[ElasticSearchTarget, PineconeTarget, WeaviateTarget]
Extractor = Union[PostgresExtractor]
Loader = Union[ElasticSearchLoader, PineconeLoader, WeaviateLoader]
Model = Union[OpenAI, SentenceTransformer, Cohere, Custom]


class Pipeline:
    def __init__(
        self,
        source: Source,
        transform: Transform,
        embedding: Embedding,
        sink: Sink,
        target: Target,
        realtime_server: RealtimeServer,
    ):
        self.source = source
        self.transform = transform
        self.embedding = embedding
        self.sink = sink
        self.target = target
        self.realtime_server = realtime_server

        self.extractor = self._get_extractor()
        self.loader = self._get_loader()
        self.model = self._get_model()

    def _get_extractor(self) -> Extractor:
        if isinstance(self.source, PostgresSource):
            return PostgresExtractor(self.source.dsn)
        else:
            raise ValueError("Invalid Source type")

    def _get_loader(self) -> Loader:
        if isinstance(self.sink, ElasticSearchSink) and isinstance(
            self.target, ElasticSearchTarget
        ):
            return ElasticSearchLoader(
                host=self.sink.host,
                user=self.sink.user,
                password=self.sink.password,
                ssl_assert_fingerprint=self.sink.ssl_assert_fingerprint,
                cloud_id=self.sink.cloud_id,
            )
        elif isinstance(self.sink, PineconeSink) and isinstance(
            self.target, PineconeTarget
        ):
            return PineconeLoader(
                api_key=self.sink.api_key,
                environment=self.sink.environment,
            )
        elif isinstance(self.sink, WeaviateSink) and isinstance(
            self.target, WeaviateTarget
        ):
            return WeaviateLoader(
                api_key=self.sink.api_key,
                url=self.sink.url,
            )
        else:
            raise ValueError("Target and Sink types do not match")

    def _get_model(self) -> Model:
        if isinstance(self.embedding, OpenAIEmbedding):
            return OpenAI(api_key=self.embedding.api_key, model=self.embedding.model)
        elif isinstance(self.embedding, SentenceTransformerEmbedding):
            return SentenceTransformer(model=self.embedding.model)
        elif isinstance(self.embedding, CohereEmbedding):
            return Cohere(api_key=self.embedding.api_key, model=self.embedding.model)
        elif isinstance(self.embedding, CustomEmbedding):
            return Custom(func=self.embedding.func)
        else:
            raise ValueError("Invalid Embedding type")

    def _apply_transform(self, row: Tuple[str, ...]) -> str:
        return cast(str, self.transform.transform_func(*row))

    def _create_metadata(self, row: Tuple[str, ...]) -> Dict[str, Any]:
        if self.transform.optional_metadata:
            return cast(Dict[str, Any], self.transform.optional_metadata(*row))
        else:
            raise ValueError("_create_metadata called when optional_metadata is None")

    def pipe_once(self, verbose: bool = True) -> None:
        total_rows = self.extractor.count(self.transform.relation)
        chunk_size = 100
        index_checked = False

        progress_bar = tqdm(
            total=total_rows,
            desc="Piping embeddings",
            disable=not verbose,
        )

        for chunk in self.extractor.extract_all(
            relation=self.transform.relation,
            columns=self.transform.columns,
            primary_key=self.transform.primary_key,
            chunk_size=chunk_size,
        ):
            rows = chunk.get("rows")
            primary_keys = chunk.get("primary_keys")

            if rows and primary_keys:
                # Create lists for embeddings and metadata
                documents = [self._apply_transform(row) for row in rows]
                metadata_list = (
                    [self._create_metadata(row) for row in rows]
                    if self.transform.optional_metadata
                    else None
                )
                embeddings = self.model.create_embeddings(documents)

                # Appease Mypy by ensuring that Target and Loader types match
                if not (
                    (
                        isinstance(self.target, ElasticSearchTarget)
                        and isinstance(self.loader, ElasticSearchLoader)
                    )
                    or (
                        isinstance(self.target, PineconeTarget)
                        and isinstance(self.loader, PineconeLoader)
                    )
                    or (
                        isinstance(self.target, WeaviateTarget)
                        and isinstance(self.loader, WeaviateLoader)
                    )
                ):
                    raise ValueError("Target and Loader types do not match")

                # Check and setup index
                if not index_checked:
                    self.loader.check_and_setup_index(
                        target=self.target,
                        num_dimensions=len(embeddings[0]),
                    )
                    index_checked = True

                # Upsert embeddings
                self.loader.bulk_upsert_embeddings(
                    target=self.target,
                    embeddings=embeddings,
                    ids=primary_keys,
                    metadata=metadata_list,
                )

            progress_bar.update(chunk_size)

        progress_bar.close()

    def pipe_real_time(self) -> None:
        index = self.target.index_name
        db_schema_name = self.transform.schema_name
        relation = self.transform.relation
        topic = f"{relation}.{db_schema_name}.{relation}"

        worker = register_agents(
            topic,
            index,
            self.realtime_server,
            self.model.create_embeddings,
            self.transform.transform_func,
            self.transform.optional_metadata,
        )
        start_worker(worker)

    def teardown(self) -> None:
        self.extractor.teardown()
