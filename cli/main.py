import os
import click
import yaml
import psycopg2

from cli.endpoints import SourceTypeEnum, SourceTypeConverter
from engine.source_postgres import PostgresCDC


@click.group()
def cli():
    pass


@cli.command()
def init():
    # Create config.yaml file
    pass


@cli.command()
def watch():
    dbname = "postgres"
    user = "postgres"
    host = "postgres-instance-1.chqsp2e4eplp.us-east-2.rds.amazonaws.com"
    password = "Password123!"
    table = "ecoinvent_with_types"
    slot_name = "resync_slot"
    port = 5432
    output_plugin = "pgoutput"
    publication_name = "test_pub"

    # Run Python listener
    cdc = PostgresCDC(
        publication_name=publication_name,
        slot_name=slot_name,
        database=dbname,
        host=host,
        user=user,
        password=password,
        port=port,
    )

    for event in cdc:
        print(event)

    pass


if __name__ == "__main__":
    cli()
