from pydantic import BaseModel
from enum import Enum
from typing import Optional, Dict


class Similarity(Enum):
    L2_NORM = "l2_norm"
    DOT_PRODUCT = "dot_product"
    COSINE = "cosine"


class WeaviateVecotorizer(Enum):
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
    similarity: Optional[Similarity]


class PineconeTarget(BaseModel):
    index_name: str
    namespace: str


class WeaviateTarget(BaseModel):
    index_name: str
    default_vectorizer: WeaviateVecotorizer
    default_vectorizer_config: Dict[str, str]


class Target:
    @classmethod
    def ElasticSearch(
        cls,
        index_name: str,
        field_name: str,
        should_index: bool = True,
        similarity: Optional[Similarity] = None,
    ) -> ElasticSearchTarget:
        return ElasticSearchTarget(
            index_name=index_name,
            field_name=field_name,
            should_index=should_index,
            similarity=similarity,
        )

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
        default_vectorizer: WeaviateVecotorizer,
        default_vectorizer_config: Dict[str, str],
    ) -> WeaviateTarget:
        return WeaviateTarget(
            index_name=index_name,
            default_vectorizer=default_vectorizer,
            default_vectorizer_config=default_vectorizer_config,
        )
