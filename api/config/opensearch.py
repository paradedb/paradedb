import os
from api.config.base import Config

opensearch_host = os.environ.get("OPENSEARCH_HOST")
opensearch_port = os.environ.get("OPENSEARCH_PORT")
opensearch_user = os.environ.get("OPENSEARCH_USER")
opensearch_password = os.environ.get("OPENSEARCH_PASSWORD")
opensearch_verify_certs = os.environ.get("OPENSEARCH_VERIFY_CERTS")
opensearch_cacerts = os.environ.get("OPENSEARCH_CACERTS")

if not (
    opensearch_host
    and opensearch_port
    and opensearch_user
    and opensearch_password
    and opensearch_verify_certs
):
    raise EnvironmentError("No opensearch environment variables found")


class OpenSearchConfig(Config):
    @property
    def host(self) -> str:
        return self.get_property("OPENSEARCH_HOST")

    @property
    def port(self) -> int:
        return int(self.get_property("OPENSEARCH_PORT"))

    @property
    def user(self) -> str:
        return self.get_property("OPENSEARCH_USER")

    @property
    def password(self) -> str:
        return self.get_property("OPENSEARCH_PASSWORD")

    @property
    def verify_certs(self) -> bool:
        env = self.get_property("OPENSEARCH_VERIFY_CERTS")
        return env == "True" or env == "true"
