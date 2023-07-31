import os
from api.config.base import Config

pgsync_host = os.environ.get("PGSYNC_HOST")
pgsync_port = os.environ.get("PGSYNC_PORT")
pgsync_use_tls = os.environ.get("PGSYNC_SSL_ENABLED")

if not (pgsync_host and pgsync_port and pgsync_use_tls):
    raise EnvironmentError("No pgsync environment variables found")


class PgSyncConfig(Config):
    @property
    def url(self) -> str:
        host = self.get_property("PGSYNC_HOST")
        port = self.get_property("PGSYNC_PORT")
        pgsync_use_tls = self.get_property("PGSYNC_SSL_ENABLED")

        if not bool(pgsync_use_tls):
            return f"http://{host}{port}"

        return f"https://{host}:{port}"
