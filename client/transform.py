from pydantic import BaseModel


class PostgresTransform(BaseModel):
    relation: str
    primary_key: str
    columns: list
    optional_transform: callable = None
    optional_metadata: callable = None


class Transform:
    @classmethod
    def Postgres(
        cls,
        relation: str,
        primary_key: str,
        columns: list,
        optional_transform: callable = None,
        optional_metadata: callable = None,
    ) -> PostgresTransform:
        return PostgresTransform(
            relation=relation,
            primary_key=primary_key,
            columns=columns,
            optional_transform=optional_transform,
            optional_metadata=optional_metadata,
        )
