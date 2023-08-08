import os
from api.config.base import Config

pgsync_host = os.environ.get("PGSYNC_HOST")
pgsync_port = os.environ.get("PGSYNC_PORT")

if not (pgsync_host and pgsync_port):
    raise EnvironmentError("No pgsync environment variables found")


class PgSyncConfig(Config):
    @property
    def url(self) -> str:
        host = self.get_property("PGSYNC_HOST")
        port = self.get_property("PGSYNC_PORT")

        return f"http://{host}:{port}"
