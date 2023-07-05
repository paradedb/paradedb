from pydantic import BaseModel


class ElasticSearchSink(BaseModel):
    host: str = None
    user: str = None
    password: str = None
    ssl_assert_fingerprint: str = None
    cloud_id: str = None


class Sink:
    @classmethod
    def ElasticSearch(
        cls,
        host: str = None,
        user: str = None,
        password: str = None,
        ssl_assert_fingerprint: str = None,
        cloud_id: str = None,
    ) -> ElasticSearchSink:
        return ElasticSearchSink(
            host=host,
            user=user,
            password=password,
            ssl_assert_fingerprint=ssl_assert_fingerprint,
            cloud_id=cloud_id,
        )
