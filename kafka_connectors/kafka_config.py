import os
from typing import Optional

kafka_host = os.environ.get("KAFKA_HOST")
kafka_port = os.environ.get("KAFKA_PORT")
schema_registry_host = os.environ.get("SCHEMA_REGISTRY_HOST")
schema_registry_port = os.environ.get("SCHEMA_REGISTRY_PORT")
connect_host = os.environ.get("KAFKA_CONNECT_HOST")
connect_port = os.environ.get("KAFKA_CONNECT_PORT")
http_sink_host = os.environ.get("HTTP_SINK_HOST")
http_sink_port = os.environ.get("HTTP_SINK_PORT")
http_sink_topic = os.environ.get("HTTP_SINK_TOPIC")

if not (
    kafka_host
    and kafka_port
    and schema_registry_host
    and schema_registry_port
    and connect_host
    and connect_port
):
    raise EnvironmentError(
        "No environment variables found. Make sure to export them directly or use the deploy script"
    )


class Config:
    def get_property(self, property_name: str) -> Optional[str]:
        value = os.environ.get(property_name)
        if not value:
            raise EnvironmentError(
                f"{property_name} environment variable is not defined."
            )
        return value


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

    @property
    def http_sink_server(self) -> str:
        host = self.get_property("HTTP_SINK_HOST")
        port = self.get_property("HTTP_SINK_PORT")
        return f"http://{host}:{port}"

    @property
    def http_sink_topic(self) -> str:
        return self.get_property("HTTP_SINK_TOPIC")
