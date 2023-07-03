from pydantic import BaseModel


class PostgresTransform(BaseModel):
    relation: str
    primary_key: str
    columns: list
    transform_func: callable
    optional_metadata: callable = None


class Transform:
    @classmethod
    def Postgres(
        cls,
        relation: str,
        primary_key: str,
        columns: list,
        transform_func: callable = None,
        optional_metadata: callable = None,
    ) -> PostgresTransform:
        return PostgresTransform(
            relation=relation,
            primary_key=primary_key,
            columns=columns,
            transform_func=transform_func,
            optional_metadata=optional_metadata,
        )
