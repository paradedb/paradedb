from tqdm import tqdm
from typing import Union, Tuple, Callable, Any

from retake.embedding import OpenAIEmbedding, SentenceTransformerEmbedding
from retake.source import PostgresSource
from retake.transform import PostgresTransform
from retake.sink import ElasticSearchSink
from retake.target import ElasticSearchTarget
from core.load.elasticsearch import ElasticSearchLoader
from core.extract.postgres import PostgresExtractor
from core.transform.embedding import OpenAIEmbedding as OpenAI
from core.transform.embedding import SentenceTransformerEmbedding as SentenceTransformer

Source = Union[PostgresSource]
Transform = Union[PostgresTransform]
Embedding = Union[OpenAIEmbedding, SentenceTransformerEmbedding]
Sink = Union[ElasticSearchSink]
Target = Union[ElasticSearchTarget]


class Pipeline:
    def __init__(
        self,
        source: Source,
        transform: Transform,
        embedding: Embedding,
        sink: Sink,
        target: Target,
    ):
        self.source = source
        self.transform = transform
        self.embedding = embedding
        self.sink = sink
        self.target = target

        self.extractor = self._get_extractor()
        self.loader = self._get_loader()
        self.model = self._get_model()

    def _get_extractor(self):
        if isinstance(self.source, PostgresSource):
            return PostgresExtractor(self.source.dsn)
        else:
            raise ValueError("Invalid Source type")

    def _get_loader(self):
        if isinstance(self.target, ElasticSearchTarget):
            return ElasticSearchLoader(
                host=self.sink.host,
                user=self.sink.user,
                password=self.sink.password,
                ssl_assert_fingerprint=self.sink.ssl_assert_fingerprint,
                cloud_id=self.sink.cloud_id,
            )
        else:
            raise ValueError("Invalid Target type")

    def _get_model(self):
        if isinstance(self.embedding, OpenAIEmbedding):
            return OpenAI(api_key=self.embedding.api_key, model=self.embedding.model)
        elif isinstance(self.embedding, SentenceTransformerEmbedding):
            return SentenceTransformer(model=self.embedding.model)
        else:
            raise ValueError("Invalid Embedding type")

    def _apply_transform(self, row: Tuple[str, ...]):
        return self.transform.transform_func(*row)

    def _create_metadata(self, row: Tuple[str, ...]):
        if self.transform.optional_metadata:
            return self.transform.optional_metadata(*row)
        else:
            return None

    def pipe_once(self, verbose=True):
        total_rows = self.extractor.count(self.transform.relation)
        chunk_size = 1000

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

            documents = [self._apply_transform(row) for row in rows]
            metadata_list = [self._create_metadata(row) for row in rows]
            embeddings = self.model.create_embeddings(documents)
            self.loader.bulk_upsert_embeddings(
                index_name=self.target.index_name,
                embeddings=embeddings,
                ids=primary_keys,
                field_name=self.target.field_name,
                metadata=metadata_list,
            )

            progress_bar.update(chunk_size)

        progress_bar.close()

    def pipe_real_time(
        self,
        cdc_server_url: str,
        on_success: Callable[..., Any],
        on_error: Callable[..., Any],
    ):
        raise NotImplementedError("TODO: Implement real-time sync with CDC server")

    def teardown(self):
        self.extractor.teardown()
