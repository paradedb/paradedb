import os
from dotenv import load_dotenv

load_dotenv()  # take environment variables from .env.


class Config:
    def get_property(self, property_name: str) -> str:
        return os.environ[property_name]


class SourceConfig(Config):
    @property
    def host(self) -> str:
        return self.get_property("DB_INTERNAL_HOST")


class KafkaConfig(Config):
    @property
    def bootstrap_servers(self) -> str:
        return f"{self.get_property('KAFKA_EXTERNAL_HOST')}:{self.get_property('KAFKA_EXTERNAL_PORT')}"

    @property
    def schema_registry_internal_server(self) -> str:
        return f"http://{self.get_property('SCHEMA_REGISTRY_INTERNAL_HOST')}:{self.get_property('SCHEMA_REGISTRY_PORT')}"

    @property
    def schema_registry_external_server(self) -> str:
        return f"http://{self.get_property('SCHEMA_REGISTRY_EXTERNAL_HOST')}:{self.get_property('SCHEMA_REGISTRY_PORT')}"

    @property
    def connect_server(self) -> str:
        return f"http://{self.get_property('KAFKA_CONNECT_EXTERNAL_HOST')}:{self.get_property('KAFKA_CONNECT_PORT')}"


class SinkConfig(Config):
    @property
    def server(self) -> str:
        return f"http://{self.get_property('SINK_INTERNAL_HOST')}:{self.get_property('SINK_PORT')}"
