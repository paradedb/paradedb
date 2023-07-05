from pydantic import BaseModel
from typing import Callable, Any


class PostgresTransform(BaseModel):
    relation: str
    primary_key: str
    columns: list
    transform_func: Callable[..., Any]
    optional_metadata: Callable[..., Any] = None


class Transform:
    @classmethod
    def Postgres(
        cls,
        relation: str,
        primary_key: str,
        columns: list,
        transform_func: Callable[..., Any] = None,
        optional_metadata: Callable[..., Any] = None,
    ) -> PostgresTransform:
        return PostgresTransform(
            relation=relation,
            primary_key=primary_key,
            columns=columns,
            transform_func=transform_func,
            optional_metadata=optional_metadata,
        )
