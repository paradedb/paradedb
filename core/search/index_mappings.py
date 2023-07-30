from enum import Enum
from opensearchpy import OpenSearch
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


class IndexMappings:
    def __init__(self, name: str, client: OpenSearch):
        self.name = name
        self.client = client

    def upsert(self, properties: Dict[str, Any]) -> None:
        # We upsert one-by-one, so if a single property fails, it does not
        # affect the others
        for attribute, values in properties.items():
            body = {"properties": {attribute: values}}
            print(body)
            self.client.indices.put_mapping(index=self.name, body=body)
