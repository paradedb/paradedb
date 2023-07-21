from pydantic import BaseModel


class PostgresSource(BaseModel):
    dsn: str

    @property
    def config(self) -> dict[str, str]:
        fields = {}
        pairs = self.dsn.split(" ")
        for pair in pairs:
            key, value = pair.split("=")
            fields[key] = value
        return fields


class Source:
    @classmethod
    def Postgres(
        cls,
        host: str,
        database: str,
        user: str,
        password: str,
        port: int,
    ) -> PostgresSource:
        dsn = (
            f"dbname={database} user={user} password={password} host={host} port={port}"
        )
        return PostgresSource(dsn=dsn)
