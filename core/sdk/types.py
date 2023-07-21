from core.sdk.source import PostgresSource
from core.sdk.transform import PostgresTransform
from core.sdk.embedding import (
    OpenAIEmbedding,
    SentenceTransformerEmbedding,
    CohereEmbedding,
    CustomEmbedding,
)
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
from core.extract.postgres import PostgresExtractor
from core.load.elasticsearch import ElasticSearchLoader
from core.load.opensearch import OpenSearchLoader
from core.load.pinecone import PineconeLoader
from core.load.weaviate import WeaviateLoader
from core.load.qdrant import QdrantLoader
from core.transform.openai import OpenAIEmbedding as OpenAI
from core.transform.sentence_transformers import (
    SentenceTransformerEmbedding as SentenceTransformer,
)
from core.transform.cohere import CohereEmbedding as Cohere
from core.transform.custom import CustomEmbedding as Custom

from typing import Union

Source = PostgresSource
Transform = PostgresTransform
Embedding = Union[
    OpenAIEmbedding, SentenceTransformerEmbedding, CohereEmbedding, CustomEmbedding
]
Sink = Union[ElasticSearchSink, OpenSearchSink, PineconeSink, WeaviateSink, QdrantSink]
Target = Union[
    ElasticSearchTarget, OpenSearchTarget, PineconeTarget, WeaviateTarget, QdrantTarget
]
Extractor = PostgresExtractor
Loader = Union[
    ElasticSearchLoader, OpenSearchLoader, PineconeLoader, WeaviateLoader, QdrantLoader
]
Model = Union[OpenAI, SentenceTransformer, Cohere, Custom]
