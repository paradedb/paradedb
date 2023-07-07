from pydantic import BaseModel
from enum import Enum


class Similarity(Enum):
    L2_NORM = "l2_norm"
    DOT_PRODUCT = "dot_product"
    COSINE = "cosine"


class ElasticSearchTarget(BaseModel):
    index_name: str
    field_name: str
    should_index: bool
    similarity: Similarity = None


class Target:
    @classmethod
    def ElasticSearch(
        cls,
        index_name: str,
        field_name: str,
        should_index: bool = True,
        similarity: Similarity = None,
    ) -> ElasticSearchTarget:
        return ElasticSearchTarget(
            index_name=index_name,
            field_name=field_name,
            should_index=should_index,
            similarity=similarity,
        )
