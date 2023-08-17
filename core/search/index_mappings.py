from enum import Enum
from opensearchpy import AsyncOpenSearch
from typing import Dict, Any


class FieldType(Enum):
    # Core Data Types
    TEXT = "text"
    KEYWORD = "keyword"
    INTEGER = "integer"
    LONG = "long"
    SHORT = "short"
    BYTE = "byte"
    DOUBLE = "double"
    FLOAT = "float"
    HALF_FLOAT = "half_float"
    SCALED_FLOAT = "scaled_float"
    DATE = "date"
    BOOLEAN = "boolean"
    BINARY = "binary"

    # Complex Data Types
    OBJECT = "object"
    NESTED = "nested"

    # Geo Data Types
    GEO_POINT = "geo_point"
    GEO_SHAPE = "geo_shape"

    # Specialized Data Types
    IP = "ip"
    COMPLETION = "completion"
    TOKEN_COUNT = "token_count"
    CUSTOM = "custom"
    SEARCH_AS_YOU_TYPE = "search_as_you_type"
    CONSTANT_KEYWORD = "constant_keyword"
    DENSE_VECTOR = "dense_vector"
    SPARSE_VECTOR = "sparse_vector"
    KNN_VECTOR = "knn_vector"
    RANK_FEATURE = "rank_feature"
    RANK_FEATURES = "rank_features"
    PERCOLATOR = "percolator"

    # Range Data Types
    INTEGER_RANGE = "integer_range"
    FLOAT_RANGE = "float_range"
    LONG_RANGE = "long_range"
    DATE_RANGE = "date_range"
    DOUBLE_RANGE = "double_range"

    # Join Data Types
    JOIN = "join"


class Engine(Enum):
    FAISS = "faiss"
    LUCENE = "lucene"
    NMSLIB = "nmslib"


class SpaceType(Enum):
    L2 = "l2"
    COSINE = "cosinesimil"
    INNERPRODUCT = "innerproduct"


engine_space_mapping: Dict[str, Any] = {
    Engine.FAISS.value: {SpaceType.L2.value, SpaceType.INNERPRODUCT.value},
    Engine.LUCENE.value: {SpaceType.L2.value, SpaceType.COSINE.value},
    Engine.NMSLIB.value: {
        SpaceType.L2.value,
        SpaceType.COSINE,
        SpaceType.INNERPRODUCT.value,
    },
}


class IndexMappings:
    def __init__(self, name: str, client: AsyncOpenSearch):
        self.name = name
        self.client = client

    def _validate_knn_method(self, property: Dict[str, Any]) -> None:
        dimension = property.get("dimension", None)
        method = property.get("method", dict())
        engine = method.get("engine", None)
        space_type = method.get("space_type", None)

        if not engine or not space_type or not dimension:
            raise Exception(
                "Parameters for engine, space_type, and dimension must be provided for knn_vector type"
            )

        if space_type not in engine_space_mapping[engine]:
            raise Exception(
                f"The spacetype {space_type} is not supported by the engine {engine}"
            )

    async def upsert(self, properties: Dict[str, Any]) -> None:
        for attribute, values in properties.items():
            if values.get("type") == FieldType.KNN_VECTOR.value:
                self._validate_knn_method(values)

            body = {"properties": {attribute: values}}
            await self.client.indices.put_mapping(index=self.name, body=body)
