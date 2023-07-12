import os
from dotenv import load_dotenv
from typing import Optional

RETAKE_ENV_PATH = os.path.expanduser("~/.config/retake/.env")
if not load_dotenv(RETAKE_ENV_PATH):
    raise EnvironmentError("Make sure the retake environment file exists by running the deploy script")

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
    def schema_registry_internal_server(self) -> str:
        host = self.get_property("SCHEMA_REGISTRY_INTERNAL_HOST")
        port = self.get_property("SCHEMA_REGISTRY_PORT")
        return f"http://{host}:{port}"

    @property
    def connect_server(self) -> str:
        host = self.get_property("KAFKA_CONNECT_HOST")
        port = self.get_property("KAFKA_CONNECT_PORT")
        return f"http://{host}:{port}"
