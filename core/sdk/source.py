from pydantic import BaseModel


class PostgresSource(BaseModel):
    dsn: str
    internal_host: str = None

    def parse_connection_string(self):
        fields = {}
        pairs = self.dsn.split(" ")
        for pair in pairs:
            key, value = pair.split("=")
            fields[key] = value

        # In case the internal host is set,
        # use it instead of the parsed host.
        if self.internal_host is not None:
            fields["host"] = self.internal_host
        return fields


class Source:
    @classmethod
    def Postgres(
        cls,
        host: str,
        internal_host: str,
        database: str,
        user: str,
        password: str,
        port: int,
    ) -> PostgresSource:
        dsn = (
            f"dbname={database} user={user} password={password} host={host} port={port}"
        )
        return PostgresSource(dsn=dsn, internal_host=internal_host)
