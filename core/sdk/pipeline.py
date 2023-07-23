from tqdm import tqdm
from typing import Union, Tuple, Any, Optional, Dict, List, cast

from core.sdk.embedding import (
    OpenAIEmbedding,
    SentenceTransformerEmbedding,
    CohereEmbedding,
    CustomEmbedding,
)
from core.sdk.source import PostgresSource
from core.sdk.sink import (
    ElasticSearchSink,
    OpenSearchSink,
    PineconeSink,
    WeaviateSink,
    QdrantSink,
)
from core.sdk.target import (
    ElasticSearchTarget,
    OpenSearchTarget,
    PineconeTarget,
    WeaviateTarget,
    QdrantTarget,
)
from core.sdk.realtime import RealtimeServer
from core.load.elasticsearch import ElasticSearchLoader
from core.load.opensearch import OpenSearchLoader
from core.load.pinecone import PineconeLoader
from core.load.weaviate import WeaviateLoader
from core.load.qdrant import QdrantLoader
from core.extract.postgres import PostgresExtractor
from core.transform.openai import OpenAIEmbedding as OpenAI
from core.transform.sentence_transformers import (
    SentenceTransformerEmbedding as SentenceTransformer,
)
from core.transform.cohere import CohereEmbedding as Cohere
from core.transform.custom import CustomEmbedding as Custom
from streams.app import (
    register_connector_conf,
    wait_for_config_success,
    register_agents,
    start_worker,
)
from core.sdk.types import (
    Source,
    Transform,
    Embedding,
    Sink,
    Target,
    Extractor,
    Loader,
    Model,
)

BATCH_SIZE = 100


class Pipeline:
    def __init__(
        self,
        source: Source,
        sink: Sink,
        target: Target,
        embedding: Embedding,
        transform: Optional[Transform],
    ):
        self.source = source
        self.transform = transform
        self.embedding = embedding
        self.sink = sink
        self.target = target

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
        elif isinstance(self.sink, OpenSearchSink) and isinstance(
            self.target, OpenSearchTarget
        ):
            return OpenSearchLoader(
                hosts=self.sink.hosts,
                user=self.sink.user,
                password=self.sink.password,
                use_ssl=self.sink.use_ssl,
                cacerts=self.sink.cacerts,
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
                default_vectorizer=self.target.default_vectorizer,
                default_vectorizer_config=self.target.default_vectorizer_config,
            )

        elif isinstance(self.sink, QdrantSink) and isinstance(
            self.target, QdrantTarget
        ):
            return QdrantLoader(
                host=self.sink.host,
                port=self.sink.port,
                url=self.sink.url,
                api_key=self.sink.api_key,
                similarity=self.target.similarity,
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
        if not self.transform:
            raise ValueError(
                "Transform expected but got None. Did you forget to provide a transform argument?"
            )

        return cast(str, self.transform.transform_func(*row))

    def _create_metadata(self, row: Tuple[str, ...]) -> Dict[str, Any]:
        if not self.transform:
            raise ValueError(
                "Transform expected but got None. Did you forget to provide a transform argument?"
            )

        if not self.transform.optional_metadata:
            raise ValueError("_create_metadata called when optional_metadata is None")

        return cast(Dict[str, Any], self.transform.optional_metadata(*row))

    def pipe(
        self,
        ids: List[Union[str, int]],
        embeddings: Optional[List[List[float]]] = None,
        documents: Optional[List[str]] = None,
        metadata: Optional[List[Dict[str, Any]]] = None,
        verbose: bool = True,
    ) -> None:
        if not embeddings and not documents:
            raise ValueError("Both embeddings and documents cannot be None")

        if embeddings and documents:
            raise ValueError("Both embeddings and documents cannot be provided")

        num_rows = len(ids)
        num_batches = num_rows // BATCH_SIZE
        progress_bar = tqdm(
            total=num_rows,
            desc="Piping embeddings",
            disable=not verbose,
        )

        for i in range(num_batches + 1):
            start = i * BATCH_SIZE
            end = (i + 1) * BATCH_SIZE

            batch_ids = ids[start:end]
            batch_embeddings = embeddings[start:end] if embeddings else None
            batch_documents = documents[start:end] if documents else None
            batch_metadata = metadata[start:end] if metadata else None

            if batch_documents:
                batch_embeddings = self.model.create_embeddings(batch_documents)

            self.loader.bulk_upsert_embeddings(
                target=self.target,
                embeddings=cast(List[List[float]], batch_embeddings),
                ids=batch_ids,
                metadata=batch_metadata,
            )

            progress_bar.update(BATCH_SIZE)

        progress_bar.close()

    def pipe_all(self, verbose: bool = True) -> None:
        if not self.transform:
            raise ValueError(
                "Transform expected but got None. Did you forget to provide a transform argument?"
            )

        total_rows = self.extractor.count(self.transform.relation)
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
            chunk_size=BATCH_SIZE,
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

                # Check and setup index
                if not index_checked:
                    self.loader.check_and_setup_index(
                        target=self.target,  # type: ignore
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

            progress_bar.update(BATCH_SIZE)

        progress_bar.close()

    def create_real_time(self, server: RealtimeServer) -> None:
        if self.transform is None:
            raise ValueError(
                "Transform expected but got None. Did you forget to provide a transform argument?"
            )
        index = self.target.index_name
        db_schema_name = self.transform.schema_name
        table_name = self.transform.relation

        register_connector_conf(
            server, index, db_schema_name, table_name, self.source, self.sink
        )
        wait_for_config_success(server)

    def pipe_real_time(self, server: RealtimeServer) -> None:
        if self.transform is None:
            raise ValueError(
                "Transform expected but got None. Did you forget to provide a transform argument?"
            )

        index = self.target.index_name
        db_schema_name = self.transform.schema_name
        table_name = self.transform.relation
        topic = f"{table_name}.{db_schema_name}.{table_name}"

        worker = register_agents(
            topic,
            index,
            server,
            self.model.create_embeddings,
            self.transform.transform_func,
            self.transform.optional_metadata,
        )
        start_worker(worker)

    def teardown(self) -> None:
        self.extractor.teardown()
