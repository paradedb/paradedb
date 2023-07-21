from pydantic import BaseModel
from enum import Enum
from typing import Optional, Dict, Any


class ElasticSimilarity(Enum):
    L2_NORM = "l2_norm"
    DOT_PRODUCT = "dot_product"
    COSINE = "cosine"


class QdrantSimilarity(Enum):
    COSINE = "Cosine"
    EUCLID = "Euclid"
    DOT = "Dot"


class WeaviateVectorizer(Enum):
    COHERE = "text2vec-cohere"
    OPENAI = "text2vec-openai"
    PALM = "text2vec-palm"
    HUGGINGFACE = "text2vec-huggingface"
    TRANSFORMERS = "text2vec-transformers"
    CONTEXTIONARY = "text2vec-contextionary"


class ElasticSearchTarget(BaseModel):
    index_name: str
    field_name: str
    should_index: bool
    similarity: Optional[ElasticSimilarity] = None


class OpenSearchTarget(BaseModel):
    index_name: str
    field_name: str


class PineconeTarget(BaseModel):
    index_name: str
    namespace: str


class WeaviateTarget(BaseModel):
    index_name: str
    default_vectorizer: WeaviateVectorizer
    default_vectorizer_config: Dict[str, Any]


class QdrantTarget(BaseModel):
    index_name: str
    similarity: Optional[QdrantSimilarity] = None


class Target:
    @classmethod
    def ElasticSearch(
        cls,
        index_name: str,
        field_name: str,
        should_index: bool = True,
        similarity: Optional[ElasticSimilarity] = None,
    ) -> ElasticSearchTarget:
        return ElasticSearchTarget(
            index_name=index_name,
            field_name=field_name,
            should_index=should_index,
            similarity=similarity,
        )

    @classmethod
    def OpenSearch(cls, index_name: str, field_name: str) -> OpenSearchTarget:
        return OpenSearchTarget(index_name=index_name, field_name=field_name)

    @classmethod
    def Pinecone(cls, index_name: str, namespace: str) -> PineconeTarget:
        return PineconeTarget(
            index_name=index_name,
            namespace=namespace,
        )

    @classmethod
    def Weaviate(
        cls,
        index_name: str,
        default_vectorizer: WeaviateVectorizer,
        default_vectorizer_config: Dict[str, str],
    ) -> WeaviateTarget:
        return WeaviateTarget(
            index_name=index_name,
            default_vectorizer=default_vectorizer,
            default_vectorizer_config=default_vectorizer_config,
        )

    @classmethod
    def Qdrant(
        cls,
        index_name: str,
        similarity: Optional[QdrantSimilarity] = None,
    ) -> QdrantTarget:
        return QdrantTarget(
            index_name=index_name,
            similarity=similarity,
        )
