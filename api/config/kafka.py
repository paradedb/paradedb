import os
from typing import Optional
from api.config.base import Config

kafka_host = os.environ.get("KAFKA_HOST")
kafka_port = os.environ.get("KAFKA_PORT")
schema_registry_host = os.environ.get("SCHEMA_REGISTRY_HOST")
schema_registry_port = os.environ.get("SCHEMA_REGISTRY_PORT")
connect_host = os.environ.get("KAFKA_CONNECT_HOST")
connect_port = os.environ.get("KAFKA_CONNECT_PORT")


class KafkaConfig(Config):
    @property
    def bootstrap_servers(self) -> str:
        host = self.get_property("KAFKA_HOST")
        port = self.get_property("KAFKA_PORT")
        return f"{host}:{port}"

    @property
    def schema_registry_server(self) -> str:
        host = self.get_property("SCHEMA_REGISTRY_HOST")
        port = self.get_property("SCHEMA_REGISTRY_PORT")
        return f"http://{host}:{port}"

    @property
    def connect_server(self) -> str:
        host = self.get_property("KAFKA_CONNECT_HOST")
        port = self.get_property("KAFKA_CONNECT_PORT")
        return f"http://{host}:{port}"
