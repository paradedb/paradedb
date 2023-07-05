from pydantic import BaseModel


class ElasticSearchTarget(BaseModel):
    index_name: str
    field_name: str


class Target:
    @classmethod
    def ElasticSearch(
        cls,
        index_name: str,
        field_name: str,
    ) -> ElasticSearchTarget:
        return ElasticSearchTarget(
            index_name=index_name,
            field_name=field_name,
        )
