from pydantic import BaseModel

DEFAULT_BROKER_PORT = "9094"
DEFAULT_SCHEMA_REGISTRY_PORT = "8081"

class RealtimeServer(BaseModel):
    host: str
    broker_port: str = DEFAULT_BROKER_PORT
    schema_registry_port: str = DEFAULT_SCHEMA_REGISTRY_PORT
    use_tls: bool = False # TODO: make this default to True

    @property
    def broker_host(self) -> str:
        return f"{self.host}:{self.broker_port}"

    @property
    def schema_registry_url(self) -> str:
        if self.use_tls:
            return f"https://{self.host}:{self.schema_registry_port}"

        return f"http://{self.host}:{self.schema_registry_port}"
