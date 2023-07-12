import click
import questionary
from realtime_server.connectors import (
    create_source_connector,
    create_sink_connector,
    register_sink_value_schema,
)

POSTGRES_DEFAULT_PORT = "5432"
POSTGRES_DEFAULT_SCHEMA_NAME = "public"
ELASTICSEARCH_DEFAULT_PORT = "9200"


@click.group()
def cli():
    pass


@click.command()
def init():
    source_type = questionary.select(
        "What is the source of your data?", choices=["postgres"]
    ).ask()

    source_conn = {}
    if source_type == "postgres":
        source_conn = questionary.form(
            source_host=questionary.text("host:"),
            source_port=questionary.text("port:", default=POSTGRES_DEFAULT_PORT),
            source_db_name=questionary.text("database name:"),
            source_schema_name=questionary.text(
                "schema name:", default=POSTGRES_DEFAULT_SCHEMA_NAME
            ),
            source_table_name=questionary.text("table name:"),
            source_user=questionary.text("user:"),
            source_password=questionary.password("password:"),
        ).ask()
    else:
        print("Not suppported yet!")
        return

    sink_type = questionary.select(
        "What is the target sink for your data?", choices=["elasticsearch"]
    ).ask()

    sink_conn = {}
    if sink_type == "elasticsearch":
        sink_conn = questionary.form(
            sink_host=questionary.text("host:"),
            sink_port=questionary.text("port:", default=ELASTICSEARCH_DEFAULT_PORT),
            sink_index=questionary.text("index:"),
            sink_user=questionary.text("user:"),
            sink_password=questionary.password("password:"),
        ).ask()
    else:
        print("Not supported yet!")
        return

    create_source_connector(source_conn)
    create_sink_connector(sink_conn)
    register_sink_value_schema(sink_conn["sink_index"])


def setup_cli():
    cli.add_command(init)
    cli()


def main():
    setup_cli()


if __name__ == "__main__":
    main()
