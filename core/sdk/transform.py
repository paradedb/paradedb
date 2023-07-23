from pydantic import BaseModel
from typing import Callable, Any, List, Optional


class PostgresTransform(BaseModel):
    relation: str
    primary_key: str
    columns: List[str]
    transform_func: Callable[..., Any]
    schema_name: str = "public"
    optional_metadata: Optional[Callable[..., Any]]


class Transform:
    @classmethod
    def Postgres(
        cls,
        relation: str,
        primary_key: str,
        columns: List[str],
        transform_func: Callable[..., Any],
        optional_metadata: Optional[Callable[..., Any]] = None,
    ) -> PostgresTransform:
        return PostgresTransform(
            relation=relation,
            primary_key=primary_key,
            columns=columns,
            transform_func=transform_func,
            optional_metadata=optional_metadata,
        )
