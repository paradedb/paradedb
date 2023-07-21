from pydantic import BaseModel
from typing import Optional, List, Dict


class ElasticSearchSink(BaseModel):
    host: Optional[str] = None
    user: Optional[str] = None
    password: Optional[str] = None
    ssl_assert_fingerprint: Optional[str] = None
    cloud_id: Optional[str] = None

    @property
    def config(self) -> dict[str, Optional[str]]:
        if self.cloud_id is not None:
            return {"cloud_id": self.cloud_id}

        return {
            "host": self.host,
            "user": self.user,
            "password": self.password,
            "ssl_assert_fingerprint": self.ssl_assert_fingerprint,
        }


class OpenSearchSink(BaseModel):
    hosts: List[Dict[str, str]]
    user: str
    password: str
    use_ssl: bool
    cacerts: str

    @property
    def config(self) -> dict[str, Optional[str]]:
        # Unimplemented
        return {}


class PineconeSink(BaseModel):
    api_key: str
    environment: str

    @property
    def config(self) -> dict[str, Optional[str]]:
        # Unimplemented
        return {}


class WeaviateSink(BaseModel):
    api_key: str
    url: str

    @property
    def config(self) -> dict[str, Optional[str]]:
        # Unimplemented
        return {}


class QdrantSink(BaseModel):
    host: Optional[str] = None
    port: Optional[int] = None
    url: Optional[str] = None
    api_key: Optional[str] = None

    @property
    def config(self) -> dict[str, Optional[str]]:
        # Unimplemented
        return {}


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
    def OpenSearch(
        cls,
        hosts: List[Dict[str, str]],
        user: str,
        password: str,
        use_ssl: bool,
        cacerts: str,
    ) -> OpenSearchSink:
        return OpenSearchSink(
            hosts=hosts, user=user, password=password, use_ssl=use_ssl, cacerts=cacerts
        )

    @classmethod
    def Pinecone(cls, api_key: str, environment: str) -> PineconeSink:
        return PineconeSink(
            api_key=api_key,
            environment=environment,
        )

    @classmethod
    def Weaviate(cls, api_key: str, url: str) -> WeaviateSink:
        return WeaviateSink(
            api_key=api_key,
            url=url,
        )

    @classmethod
    def Qdrant(
        cls,
        host: Optional[str] = None,
        port: Optional[int] = None,
        url: Optional[str] = None,
        api_key: Optional[str] = None,
    ) -> QdrantSink:
        return QdrantSink(host=host, port=port, url=url, api_key=api_key)
