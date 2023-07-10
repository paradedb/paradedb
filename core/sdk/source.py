from pydantic import BaseModel


class PostgresSource(BaseModel):
    dsn: str


class Source:
    @classmethod
    def Postgres(
        cls, host: str, database: str, user: str, password: str, port: int
    ) -> PostgresSource:
        dsn = (
            f"dbname={database} user={user} password={password} host={host} port={port}"
        )
        return PostgresSource(dsn=dsn)
