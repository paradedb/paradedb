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
    # Challenge is figuring out an extensible config.yaml format that can handle
    # multiple sources, sinks, and mappings between sources and sinks
    # I haven't figured out the best way to do this yet
    pass


@cli.command()
def watch():
    # TODO (Phil): Read this in from a config.yaml file
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
    try:
        print("Listening for changes...")
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

    except KeyboardInterrupt:
        print("Tearing down...")
        cdc.teardown()
        print("Teardown successful.")

    pass


if __name__ == "__main__":
    cli()
