from pydantic import BaseModel
from typing import Optional


class ElasticSearchSink(BaseModel):
    host: Optional[str]
    user: Optional[str]
    password: Optional[str]
    ssl_assert_fingerprint: Optional[str]
    cloud_id: Optional[str]


class PineconeSink(BaseModel):
    api_key: str
    environment: str


class Sink:
    @classmethod
    def ElasticSearch(
        cls,
        host: Optional[str] = None,
        user: Optional[str] = None,
        password: Optional[str] = None,
        ssl_assert_fingerprint: Optional[str] = None,
        cloud_id: Optional[str] = None,
    ) -> ElasticSearchSink:
        params = {
            "host": host,
            "user": user,
            "password": password,
            "ssl_assert_fingerprint": ssl_assert_fingerprint,
            "cloud_id": cloud_id,
        }
        # Remove keys with None values
        params = {k: v for k, v in params.items() if v is not None}
        return ElasticSearchSink(**params)

    @classmethod
    def Pinecone(cls, api_key: str, environment: str) -> PineconeSink:
        return PineconeSink(
            api_key=api_key,
            environment=environment,
        )
