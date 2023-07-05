from pydantic import BaseModel


class ElasticSearchTarget(BaseModel):
    index: str
    field_name: str


class Target:
    @classmethod
    def ElasticSearch(
        cls,
        index: str,
        field_name: str,
    ) -> ElasticSearchTarget:
        return ElasticSearchTarget(
            index=index,
            field_name=field_name,
        )
